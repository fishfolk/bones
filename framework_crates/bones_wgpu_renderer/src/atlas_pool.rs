use bones_framework::prelude as bones;
use guillotiere::{size2, AllocId, Allocation, AtlasAllocator};
use std::{collections::HashMap, sync::Arc};

#[derive(Clone, Copy, Debug)]
pub struct CachedAtlasAllocation {
    pub atlas_id: usize,
    pub alloc: Allocation,
    pub uv_min: [f32; 2],
    pub uv_max: [f32; 2],
    pub image_size: [f32; 2],
    pub last_used_generation: u64,
}

/// A pool of GPU texture atlases, each capable of holding multiple packed sprites.
pub struct AtlasPool {
    device: Arc<wgpu::Device>,
    layout: Arc<wgpu::BindGroupLayout>,
    pub atlases: Vec<TextureAtlas>,
    max_atlases: usize,
    pub default_atlas_size: (u32, u32),
    pub max_texture_dimension_2d: u32,
    pixel_art: bool,
    pub generation: u64,
    pub cache: HashMap<(bones::Handle<bones::Image>, bones::Cid), CachedAtlasAllocation>,
}

impl AtlasPool {
    pub fn new(
        device: &Arc<wgpu::Device>,
        layout: &Arc<wgpu::BindGroupLayout>,
        preferred_atlas_size: (u32, u32),
        max_atlases: usize,
        pixel_art: bool,
    ) -> Self {
        let max_texture_dimension_2d = device.limits().max_texture_dimension_2d;
        let default_atlas_size = (
            preferred_atlas_size.0.min(max_texture_dimension_2d),
            preferred_atlas_size.1.min(max_texture_dimension_2d),
        );

        AtlasPool {
            device: device.clone(),
            layout: layout.clone(),
            atlases: Vec::new(),
            max_atlases,
            default_atlas_size,
            max_texture_dimension_2d,
            pixel_art,
            generation: 0,
            cache: HashMap::new(),
        }
    }

    /// Allocates space for an image of `image_size` `(width, height)` in one of the atlases,
    /// reserving a 1px extruded border on each side whenever the GPU 2D texture limit permits so
    /// sampling never bleeds into adjacent packed sprites.
    ///
    /// Returns `(atlas_id, allocation, atlas_size, (pad_x, pad_y))`.
    pub fn allocate_for_image(
        &mut self,
        image_size: (u32, u32),
    ) -> Result<(usize, Allocation, (u32, u32), (u32, u32)), String> {
        let (img_w, img_h) = image_size;
        if img_w == 0 || img_h == 0 {
            return Err(format!(
                "AtlasPool: cannot allocate zero-sized image {:?}",
                image_size
            ));
        }
        if img_w > self.max_texture_dimension_2d || img_h > self.max_texture_dimension_2d {
            return Err(format!(
                "AtlasPool: image {:?} exceeds GPU max_texture_dimension_2d ({})",
                image_size, self.max_texture_dimension_2d
            ));
        }

        let pad_x = u32::from(img_w.saturating_add(2) <= self.max_texture_dimension_2d);
        let pad_y = u32::from(img_h.saturating_add(2) <= self.max_texture_dimension_2d);
        let alloc_w = img_w + 2 * pad_x;
        let alloc_h = img_h + 2 * pad_y;
        let alloc_size = (alloc_w as i32, alloc_h as i32);

        // 1) Try existing atlases
        for (idx, atlas) in self.atlases.iter_mut().enumerate() {
            if let Some(alloc) = atlas.allocate(alloc_size) {
                return Ok((idx, alloc, atlas.size, (pad_x, pad_y)));
            }
        }

        // 2) Need a new atlas (grow beyond `default_atlas_size` if a large image requires it)
        if self.atlases.len() < self.max_atlases {
            let id = self.atlases.len();
            let atlas_size = (
                self.default_atlas_size.0.max(alloc_w),
                self.default_atlas_size.1.max(alloc_h),
            );

            let mut atlas =
                TextureAtlas::new(&self.device, &self.layout, id, atlas_size, self.pixel_art);
            let alloc = atlas.allocate(alloc_size).ok_or_else(|| {
                format!(
                    "AtlasPool: newly created {:?} atlas could not fit {:?}",
                    atlas_size, alloc_size
                )
            })?;
            self.atlases.push(atlas);
            return Ok((id, alloc, atlas_size, (pad_x, pad_y)));
        }

        // 3) All atlases full
        Err(format!(
            "AtlasPool: all {} atlases are full, cannot allocate image {:?}",
            self.max_atlases, image_size
        ))
    }

    /// Frees the given allocation back into its atlas, making space reusable.
    pub fn deallocate(&mut self, atlas_id: usize, alloc: Allocation) -> bool {
        if let Some(atlas) = self.atlases.get_mut(atlas_id) {
            atlas.deallocate(alloc.id);
            true
        } else {
            false
        }
    }
}

/// Builds a 256-byte row-aligned RGBA8 upload buffer with `pad_x`/`pad_y` extruded edge texels so
/// linear and nearest sampling at UV borders never bleed across packed atlas neighbors.
pub fn build_extruded_rgba_upload(
    raw_rgba: &[u8],
    w: u32,
    h: u32,
    pad_x: u32,
    pad_y: u32,
) -> (Vec<u8>, u32, u32, u32) {
    let upload_w = w + 2 * pad_x;
    let upload_h = h + 2 * pad_y;
    let unpadded_src_stride = (w as usize) * 4;
    let unpadded_dst_stride = (upload_w as usize) * 4;
    let padded_stride = unpadded_dst_stride.div_ceil(256) * 256;
    let mut padded_data = vec![0u8; padded_stride * (upload_h as usize)];

    let write_row = |dst_row_slice: &mut [u8], src_row: &[u8]| {
        let inner_start = (pad_x as usize) * 4;
        let inner_end = inner_start + unpadded_src_stride;
        dst_row_slice[inner_start..inner_end].copy_from_slice(src_row);
        if pad_x > 0 && w > 0 {
            let first_px = &src_row[0..4];
            let last_px = &src_row[unpadded_src_stride - 4..unpadded_src_stride];
            for px in 0..(pad_x as usize) {
                dst_row_slice[px * 4..(px + 1) * 4].copy_from_slice(first_px);
                let right_start = inner_end + px * 4;
                dst_row_slice[right_start..right_start + 4].copy_from_slice(last_px);
            }
        }
    };

    for row in 0..(h as usize) {
        let src_offset = row * unpadded_src_stride;
        let src_row = &raw_rgba[src_offset..src_offset + unpadded_src_stride];
        let dst_row = (pad_y as usize) + row;
        let dst_offset = dst_row * padded_stride;
        write_row(
            &mut padded_data[dst_offset..dst_offset + unpadded_dst_stride],
            src_row,
        );
    }

    if pad_y > 0 && h > 0 {
        let first_src_row = &raw_rgba[0..unpadded_src_stride];
        let last_src_offset = ((h as usize) - 1) * unpadded_src_stride;
        let last_src_row = &raw_rgba[last_src_offset..last_src_offset + unpadded_src_stride];

        for py in 0..(pad_y as usize) {
            let top_offset = py * padded_stride;
            write_row(
                &mut padded_data[top_offset..top_offset + unpadded_dst_stride],
                first_src_row,
            );

            let bot_row = (pad_y as usize) + (h as usize) + py;
            let bot_offset = bot_row * padded_stride;
            write_row(
                &mut padded_data[bot_offset..bot_offset + unpadded_dst_stride],
                last_src_row,
            );
        }
    }

    (padded_data, upload_w, upload_h, padded_stride as u32)
}

pub struct TextureAtlas {
    pub id: usize,
    pub size: (u32, u32),
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
        let filter_mode = if pixel_art {
            wgpu::FilterMode::Nearest
        } else {
            wgpu::FilterMode::Linear
        };
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: filter_mode,
            min_filter: filter_mode,
            mipmap_filter: filter_mode,
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
            size,
            allocator,
            texture,
            view,
            bind_group,
            allocated: 0,
        }
    }

    /// Attempt to allocate; returns None if sprite doesn't fit
    pub fn allocate(&mut self, size: (i32, i32)) -> Option<Allocation> {
        let alloc = self.allocator.allocate(size.into());
        if alloc.is_some() {
            self.allocated += 1;
        }
        alloc
    }

    /// Free a single rectangle back into this atlas’s free-list
    pub fn deallocate(&mut self, alloc_id: AllocId) {
        self.allocated = self.allocated.saturating_sub(1);
        self.allocator.deallocate(alloc_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extruded_upload_replicates_edges_and_aligns_rows() {
        // 2x2 image: TL=red, TR=green, BL=blue, BR=white
        let raw = [
            255, 0, 0, 255, 0, 255, 0, 255, // row 0
            0, 0, 255, 255, 255, 255, 255, 255, // row 1
        ];
        let (buf, up_w, up_h, stride) = build_extruded_rgba_upload(&raw, 2, 2, 1, 1);
        assert_eq!((up_w, up_h, stride), (4, 4, 256));
        assert_eq!(buf.len(), 256 * 4);

        let px = |x: usize, y: usize| -> [u8; 4] {
            let off = y * (stride as usize) + x * 4;
            [buf[off], buf[off + 1], buf[off + 2], buf[off + 3]]
        };

        // Corners and edges match nearest interior pixel
        assert_eq!(px(0, 0), [255, 0, 0, 255]);
        assert_eq!(px(1, 0), [255, 0, 0, 255]);
        assert_eq!(px(2, 0), [0, 255, 0, 255]);
        assert_eq!(px(3, 0), [0, 255, 0, 255]);

        assert_eq!(px(1, 1), [255, 0, 0, 255]);
        assert_eq!(px(2, 1), [0, 255, 0, 255]);
        assert_eq!(px(1, 2), [0, 0, 255, 255]);
        assert_eq!(px(2, 2), [255, 255, 255, 255]);

        assert_eq!(px(0, 3), [0, 0, 255, 255]);
        assert_eq!(px(3, 3), [255, 255, 255, 255]);
    }
}
