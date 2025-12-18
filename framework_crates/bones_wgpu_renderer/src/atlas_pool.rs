use bones_framework::prelude::Entity;
use crossbeam_channel::bounded;
use guillotiere::{size2, AllocId, Allocation, AtlasAllocator};
use std::sync::Arc;

/// Code for the AtlasPool and the TextureAtlas.
/// A pool of texture atlases, each capable of holding multiple sprites.

pub struct AtlasPool {
    device: Arc<wgpu::Device>,
    layout: Arc<wgpu::BindGroupLayout>,
    pub atlases: Vec<TextureAtlas>,
    next_id: usize,
    max_atlases: usize,
    pub atlas_size: (u32, u32),
    pixel_art: bool,
    //Used to remove deleted textures
    pub sender: crossbeam_channel::Sender<(Entity, usize, Allocation)>,
    pub receiver: crossbeam_channel::Receiver<(Entity, usize, Allocation)>,
}

impl AtlasPool {
    pub fn new(
        device: &Arc<wgpu::Device>,
        layout: &Arc<wgpu::BindGroupLayout>,
        atlas_size: (u32, u32),
        max_atlases: usize,
        pixel_art: bool,
    ) -> Self {
        // I think 100 is a good number?
        let (sender, receiver) = bounded(1000);

        AtlasPool {
            device: device.clone(),
            layout: layout.clone(),
            atlases: Vec::new(),
            next_id: 0,
            max_atlases,
            atlas_size,
            pixel_art,
            sender,
            receiver,
        }
    }

    /// Allocate a rectangle of `size` in one of the atlases.
    /// If none has room and we’re under `max_atlases`, we create a new atlas.
    /// If at capacity and no atlas has space, returns an error.
    pub fn allocate(&mut self, size: (i32, i32)) -> Result<(usize, Allocation), String> {
        // 1) Try existing atlases
        if let Some((idx, alloc)) = self
            .atlases
            .iter_mut()
            .enumerate()
            .filter_map(|(i, a)| a.allocate(size).map(|alloc| (i, alloc)))
            .next()
        {
            return Ok((self.atlases[idx].id, alloc));
        }

        // 2) Need a new atlas?
        if self.atlases.len() < self.max_atlases {
            let id = self.next_id;
            self.next_id += 1;

            let mut atlas = TextureAtlas::new(
                &self.device,
                &self.layout,
                id,
                self.atlas_size,
                self.pixel_art,
            );
            let alloc = atlas
                .allocate(size)
                .expect("Newly-created atlas must fit the requested sprite size");
            self.atlases.push(atlas);
            return Ok((id, alloc));
        }

        // 3) All atlases full
        Err(format!(
            "AtlasPool: all {} atlases are full, cannot allocate size {:?}",
            self.max_atlases, size
        ))
    }

    /// Frees the given allocation back into its atlas, making space reusable.
    pub fn deallocate(&mut self, atlas_id: usize, alloc: Allocation) -> bool {
        if let Some(atlas) = self.atlases.iter_mut().find(|a| a.id == atlas_id) {
            atlas.deallocate(alloc.id);
            true
        } else {
            false
        }
    }
}

pub struct TextureAtlas {
    pub id: usize,
    pub allocator: AtlasAllocator, // guillotiere packing
    pub texture: wgpu::Texture,    // the GPU texture
    pub view: wgpu::TextureView,
    pub bind_group: wgpu::BindGroup, // for sampling it in shaders
    pub allocated: usize,
}

impl TextureAtlas {
    pub fn new(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        id: usize,
        size: (u32, u32),
        pixel_art: bool,
    ) -> Self {
        // initialize allocator
        let allocator = AtlasAllocator::new(size2(size.0 as i32, size.1 as i32));

        // create GPU texture
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&format!("Atlas #{}", id)),
            size: wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let view = texture.create_view(&Default::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: if pixel_art {
                wgpu::FilterMode::Nearest
            } else {
                wgpu::FilterMode::Linear
            },
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // create bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: Some(&format!("Atlas BG #{}", id)),
        });

        TextureAtlas {
            id,
            allocator,
            texture,
            view,
            bind_group,
            allocated: 0,
        }
    }

    /// Attempt to allocate; returns None if sprite doesn't fit
    pub fn allocate(&mut self, size: (i32, i32)) -> Option<Allocation> {
        self.allocated += 1;
        self.allocator.allocate(size.into())
    }

    /// Free a single rectangle back into this atlas’s free-list
    pub fn deallocate(&mut self, alloc_id: AllocId) {
        self.allocated -= 1;
        self.allocator.deallocate(alloc_id);
    }
}
