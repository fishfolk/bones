use crate::{atlas_pool::AtlasPool, *};
use bones_framework::{
    prelude::{self as bones, BitSet, ComponentIterBitset, Transform, Ustr},
    render::transform,
};
use guillotiere::Allocation;
use std::collections::HashMap;

// Functions used to load sprites, atlas sprites and tile sprites, and update them.

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraTransform {
    transform: [[f32; 4]; 4],
    screen_size: [f32; 2],
    _pad0: u32,
    _pad1: u32,
}

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct AtlasSpriteUniform {
    // Base parameters
    pub entity_type: u32,
    pub camera_index: u32,
    pub _pad0: u32,
    pub _pad1: u32,
    pub transform: [[f32; 4]; 4],
    pub color_tint: [f32; 4],

    // Sprite and Atlas parameters
    pub flip_x: u32,
    pub flip_y: u32,
    pub uv_min: [f32; 2],
    pub uv_max: [f32; 2],

    // Atlas parameters
    pub tile_size: [f32; 2],
    pub image_size: [f32; 2],
    pub padding: [f32; 2],
    pub offset: [f32; 2],

    pub columns: u32,
    pub index: u32,
}

impl AtlasSpriteUniform {
    pub fn from_atlas_sprite(
        atlas_sprite: &bones::AtlasSprite,
        atlas: &bones::Atlas,
        transform: &bones::Transform,
        uvs: ([f32; 2], [f32; 2]),
    ) -> Self {
        let image_size = [
            atlas.offset.x + ((atlas.tile_size.x + atlas.padding.x) * atlas.columns as f32),
            atlas.offset.y + ((atlas.tile_size.y + atlas.padding.y) * atlas.rows as f32),
        ];

        if atlas_sprite.flip_x {
            println!("Flipping X for sprite: {:?}", atlas_sprite.index);
        } else {
            println!("Not flipping X for sprite: {:?}", atlas_sprite.index);
        }

        Self {
            tile_size: atlas.tile_size.into(),
            columns: atlas.columns,
            padding: atlas.padding.into(),
            offset: atlas.offset.into(),
            index: atlas_sprite.index,
            image_size,
            entity_type: 1,
            flip_x: atlas_sprite.flip_x as u32,
            flip_y: atlas_sprite.flip_y as u32,
            color_tint: atlas_sprite.color.as_rgba_f32(),
            transform: transform.to_matrix_none().to_cols_array_2d(),
            uv_min: uvs.0,
            uv_max: uvs.1,
            ..Default::default()
        }
    }

    //DONE I need to spawn sprite in a place based on its position on the tile layer array
    pub fn from_tile(
        tile: &bones::Tile,
        atlas: &bones::Atlas,
        transform: &bones::Transform,
        uvs: ([f32; 2], [f32; 2]),
        index: usize,
        tile_layer: &bones::TileLayer,
    ) -> Self {
        let image_size = [
            atlas.offset.x + ((atlas.tile_size.x + atlas.padding.x) * atlas.columns as f32),
            atlas.offset.y + ((atlas.tile_size.y + atlas.padding.y) * atlas.rows as f32),
        ];
        let mut transform = transform.clone();
        transform.translation.x +=
            tile_layer.tile_size.x * (index as u32 % tile_layer.grid_size.x) as f32;
        transform.translation.y +=
            tile_layer.tile_size.y * (index as u32 / tile_layer.grid_size.x) as f32;

        Self {
            tile_size: atlas.tile_size.into(),
            columns: atlas.columns,
            padding: atlas.padding.into(),
            offset: atlas.offset.into(),
            index: 0,
            image_size,
            entity_type: 1,
            flip_x: tile.flip_x as u32,
            flip_y: tile.flip_y as u32,
            color_tint: tile.color.as_rgba_f32(),
            transform: transform.to_matrix_none().to_cols_array_2d(),
            uv_min: uvs.0,
            uv_max: uvs.1,
            ..Default::default()
        }
    }

    pub fn from_sprite(
        sprite: &bones::Sprite,
        transform: &bones::Transform,
        uvs: ([f32; 2], [f32; 2]),
        image_size: Vec2,
    ) -> Self {
        if sprite.flip_x {
            println!("Flipping X for sprite: {:?}", sprite);
        } else {
            println!("Not flipping X for sprite: {:?}", sprite);
        }

        Self {
            color_tint: sprite.color.as_rgba_f32(),
            flip_x: sprite.flip_x as u32,
            flip_y: sprite.flip_y as u32,
            entity_type: 0,
            transform: transform.to_matrix_none().to_cols_array_2d(),
            uv_min: uvs.0,
            uv_max: uvs.1,
            tile_size: image_size.into(),
            ..Default::default()
        }
    }
}

#[derive(bones_schema::HasSchema, Clone)]
#[schema(no_default)]
/// Points to the atlas and rectangle this sprite lives in.
pub struct AtlasPoolHandle {
    pub atlas_id: usize,
    pub alloc: Allocation,
    pub uv_min: [f32; 2], // Atlas UV coordinates (min_x, min_y)
    pub uv_max: [f32; 2], // Atlas UV coordinates (max_x, max_y)
    pub image_size: [f32; 2], // Original image size (width, height)

    sender: crossbeam_channel::Sender<(bones::Entity, usize, Allocation)>,
    entity: bones::Entity,
}

impl Drop for AtlasPoolHandle {
    fn drop(&mut self) {
        // Send the entity to the atlas pool for deallocation
        if let Err(e) = self.sender.send((self.entity, self.atlas_id, self.alloc)) {
            eprintln!("Failed to send entity for deallocation: {:?}", e);
        }
    }
}

//TODO Add tiles
pub fn update_atlas_pool(game: &mut bones::Game, atlas_pool: &mut AtlasPool) {
    let assets = game.shared_resource_cell::<bones::AssetServer>().unwrap();
    let queue = game.shared_resource_cell::<WgpuQueue>().unwrap();

    for (_, session) in game.sessions.iter_mut() {
        if !session.active {
            continue;
        }

        let entities = session.world.resource::<bones::Entities>();
        let sprites = session.world.component::<bones::Sprite>();
        let atlases = session.world.component::<bones::AtlasSprite>();
        let tile_layers = session.world.component::<bones::TileLayer>();
        let mut atlas_pool_handles = session.world.component_mut::<AtlasPoolHandle>();

        let mut bitset = sprites.bitset().clone();
        bitset.bit_or(atlases.bitset());
        bitset.bit_or(tile_layers.bitset());

        let mut not_atlas_handle_bitset = atlas_pool_handles.bitset().clone();
        not_atlas_handle_bitset.bit_not();

        let mut without_handle = bitset.clone();
        without_handle.bit_and(&not_atlas_handle_bitset);

        for ent in entities.iter_with_bitset(&without_handle) {
            println!("Adding sprite to atlas pool: {:?}", ent);

            let image;
            if let Some(sprite) = sprites.get(ent) {
                image = sprite.image;
            } else if let Some(atlas_sprite) = atlases.get(ent) {
                let assets = assets.borrow().unwrap();
                let atlas = assets.get(atlas_sprite.atlas);
                image = atlas.image.clone();
            } else {
                let tile_layer = tile_layers.get(ent).unwrap();
                let assets = assets.borrow().unwrap();
                let atlas = assets.get(tile_layer.atlas);
                image = atlas.image.clone();
            }
            let assets = assets.borrow().unwrap();
            let image = assets.get(image);

            if let bones::Image::Data(img) = &*image {
                //Allocate in the atlas pool guiliotiere
                let (atlas_id, alloc) = atlas_pool
                    .allocate((img.width() as i32, img.height() as i32))
                    .unwrap_or_else(|_| {
                        panic!("Failed to allocate space for sprite image: {:?}", image);
                    });

                // 5) Compute and store UVs for your sprite
                let rect: guillotiere::euclid::Box2D<i32, guillotiere::euclid::UnknownUnit> =
                    alloc.rectangle;
                let (atlas_w, atlas_h) = (
                    atlas_pool.atlas_size.0 as f32,
                    atlas_pool.atlas_size.1 as f32,
                );
                let uv_min = [rect.min.x as f32 / atlas_w, rect.min.y as f32 / atlas_h];
                let uv_max = [rect.max.x as f32 / atlas_w, rect.max.y as f32 / atlas_h];

                println!("{} {}", uv_min[0], uv_min[1]);
                println!("{} {}", uv_max[0], uv_max[1]);

                atlas_pool_handles.insert(
                    ent,
                    AtlasPoolHandle {
                        atlas_id,
                        alloc,
                        uv_min,
                        uv_max,
                        image_size: [img.width() as f32, img.height() as f32],
                        entity: ent,
                        sender: atlas_pool.sender.clone(),
                    },
                );

                // 1) Convert to RGBA8 and grab the raw bytes
                let rgba = img.to_rgba8(); // image::RgbaImage (Vec<u8> under the hood)
                let (w, h) = (rgba.width(), rgba.height());
                let raw = rgba.into_raw(); // Vec<u8>, length = w*h*4

                // 2) Compute the origin in the atlas texture
                let origin = wgpu::Origin3d {
                    x: rect.min.x as u32,
                    y: rect.min.y as u32,
                    z: 0,
                };

                let unpadded_stride = (4 * w) as usize;
                let padded_stride = ((unpadded_stride + 255) / 256) * 256;
                let mut padded_data = vec![0u8; padded_stride * h as usize];

                // copy each scanline into the padded buffer
                for row in 0..(h as usize) {
                    let src_offset = row * unpadded_stride;
                    let dst_offset = row * padded_stride;
                    padded_data[dst_offset..dst_offset + unpadded_stride]
                        .copy_from_slice(&raw[src_offset..src_offset + unpadded_stride]);
                }

                println!("Texture {:?}", atlas_pool.atlases[atlas_id].texture);

                let data_layout = wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_stride as u32),
                    rows_per_image: Some(h),
                };
                let queue = queue.borrow().unwrap();
                queue.get().write_texture(
                    wgpu::ImageCopyTexture {
                        texture: &atlas_pool.atlases[atlas_id].texture,
                        mip_level: 0,
                        origin,
                        aspect: wgpu::TextureAspect::All,
                    },
                    &padded_data,
                    data_layout,
                    wgpu::Extent3d {
                        width: w,
                        height: h,
                        depth_or_array_layers: 1,
                    },
                );
                queue.get().submit([]);
            }
        }

        // Deallocate sprites that were removed
        let removed: Vec<_> = atlas_pool.receiver.try_iter().collect();
        for (ent, atlas_id, alloc) in removed {
            println!("Removing sprite from atlas pool: {:?}", ent);
            atlas_pool.deallocate(atlas_id, alloc);
            atlas_pool_handles.remove(ent);
        }
    }
}

pub fn update_uniforms(game: &mut bones::Game, dynamic_storage: &mut DynamicBuffer) {
    // Prepare instance data
    //TODO put lenght
    let mut instances = Vec::new();

    let queue = game.shared_resource_cell::<WgpuQueue>().unwrap();
    let device = game.shared_resource_cell::<WgpuDevice>().unwrap();
    let assets = game.shared_resource_cell::<bones::AssetServer>().unwrap();

    for (_, session) in game.sessions.iter_mut() {
        if !session.visible {
            continue;
        }

        let entities = session.world.resource::<bones::Entities>();
        let sprite_lists = session.world.resource::<SpriteLists>();

        let transforms = session.world.component::<bones::Transform>();
        let sprites = session.world.component::<bones::Sprite>();
        let atlases = session.world.component::<bones::AtlasSprite>();
        let tile_layers = session.world.component::<bones::TileLayer>();
        let tiles = session.world.component::<bones::Tile>();
        let paths = session.world.component::<bones::Path2d>();
        let atlas_handles = session.world.component::<AtlasPoolHandle>();

        for entity in sprite_lists
            .opaque_list
            .iter()
            .chain(&sprite_lists.transparent_list)
        {
            let transform = transforms.get(*entity).unwrap_or_else(|| {
                transforms
                    .get(sprite_lists.tile_layer.get(entity).unwrap().0)
                    .unwrap()
            });

            if let Some(sprite) = sprites.get(*entity) {
                let Some(atlas_handle) = atlas_handles.get(*entity) else {
                    panic!("Texture not loaded in atlas pool!")
                };
                let (uv_min, uv_max) = (atlas_handle.uv_min, atlas_handle.uv_max);
                
                // Get dynamic image size from the atlas handle (stored when the image was loaded)
                let image_size = Vec2::new(atlas_handle.image_size[0], atlas_handle.image_size[1]);

                let atlas_sprite =
                    AtlasSpriteUniform::from_sprite(sprite, &transform, (uv_min, uv_max), image_size);

                println!("Adding atlas sprite: {:?}", atlas_sprite);

                instances.push(atlas_sprite);
            } else if let Some(atlas_sprite) = atlases.get(*entity) {
                let assets = assets.borrow().unwrap();
                let atlas = assets.get(atlas_sprite.atlas).clone();

                let Some(atlas_handle) = atlas_handles.get(*entity) else {
                    panic!("Texture not loaded in atlas pool!")
                };
                let (uv_min, uv_max) = (atlas_handle.uv_min, atlas_handle.uv_max);

                let atlas_sprite = AtlasSpriteUniform::from_atlas_sprite(
                    atlas_sprite,
                    &atlas,
                    &transform,
                    (uv_min, uv_max),
                );

                println!("Adding atlas sprite: {:?}", atlas_sprite);

                instances.push(atlas_sprite);
            } else if let Some(tile) = tiles.get(*entity) {
                let tile_layer_ent = sprite_lists.tile_layer.get(&entity).unwrap();
                let tile_layer = tile_layers.get(tile_layer_ent.0).unwrap();
                let assets = assets.borrow().unwrap();
                let atlas = assets.get(tile_layer.atlas).clone();

                let Some(atlas_handle) = atlas_handles.get(tile_layer_ent.0) else {
                    panic!("Texture not loaded in atlas pool!")
                };
                let (uv_min, uv_max) = (atlas_handle.uv_min, atlas_handle.uv_max);

                let tile_sprite = AtlasSpriteUniform::from_tile(
                    tile,
                    &atlas,
                    &transform,
                    (uv_min, uv_max),
                    tile_layer_ent.1,
                    tile_layer,
                );

                instances.push(tile_sprite);
            } else {
                unreachable!()
            }
        }

        for (entity, path) in entities.iter_with(&paths) {
            let Some(transform) = transforms.get(entity) else {
                unreachable!()
            };

            instances.push(AtlasSpriteUniform {
                entity_type: 2,
                transform: transform.to_matrix_none().to_cols_array_2d(),
                color_tint: path.color.as_rgba_f32(),
                ..Default::default()
            });
        }
    }

    let device = device.borrow().unwrap();
    let queue = queue.borrow().unwrap();

    println!("{}: {:?}", instances.len(), instances);

    // Update buffers (with dynamic resizing)
    dynamic_storage.write_pods(&device.get(), queue.get(), &instances);
}

#[derive(bones_schema::HasSchema, Default, Clone)]
/// Holds sorted sprite indices grouped by transparency for rendering order:
pub struct SpriteLists {
    pub transparent_list: Vec<bones::Entity>,
    pub opaque_list: Vec<bones::Entity>,
    pub index_of: HashMap<bones::Entity, u32>,
    pub tile_layer: HashMap<bones::Entity, (bones::Entity, usize)>,
}

pub fn sort_sprites(game: &mut bones::Game) {
    for (_, session) in game.sessions.iter_mut() {
        if !session.visible {
            continue;
        }

        {
            let mut sprite_lists = session.world.init_resource::<SpriteLists>();

            sprite_lists.transparent_list.clear();
            sprite_lists.opaque_list.clear();
        }

        // Now safe to borrow entities immutably
        let entities = session.world.resource::<bones::Entities>();

        let atlas_handles = session.world.component::<AtlasPoolHandle>();
        let transforms = session.world.component::<bones::Transform>();

        let sprites = session.world.component::<bones::Sprite>();
        let atlases = session.world.component::<bones::AtlasSprite>();
        let tile_layers = session.world.component::<bones::TileLayer>();
        let tiles = session.world.component::<bones::Tile>();

        //Get entities with atlas handle, and that have sprite or atlas sprite
        let mut sprites_atlases = sprites.bitset().clone();
        sprites_atlases.bit_or(atlases.bitset());
        sprites_atlases.bit_or(tile_layers.bitset());
        let mut bitset = atlas_handles.bitset().clone();
        bitset.bit_and(&sprites_atlases);

        // Pre‐frame (CPU) work:
        let mut opaque_list: Vec<bones::Entity> = Vec::new();
        let mut transparent_list: Vec<bones::Entity> = Vec::new();

        let mut tile_layer_hash = HashMap::with_capacity(tiles.bitset().len());

        //TODO This is wrong! When adding sprite we should also check if we find a transparent pixel!
        //And add a flag, but its fine for now i guess
        for ent in entities.iter_with_bitset(&bitset) {
            println!("Sorting sprite: {:?}", ent);

            if let Some(sprite) = sprites.get(ent) {
                let color = sprite.color;

                if color.a() != 1.0 {
                    transparent_list.push(ent);
                } else {
                    opaque_list.push(ent);
                }
            } else if let Some(atlas_sprite) = atlases.get(ent) {
                let color = atlas_sprite.color;

                if color.a() != 1.0 {
                    transparent_list.push(ent);
                } else {
                    opaque_list.push(ent);
                }
            } else if let Some(tile_layer) = tile_layers.get(ent) {
                for (i, tile) in tile_layer.tiles.iter().enumerate() {
                    if let Some(tile_ent) = tile {
                        let tile = tiles.get(*tile_ent).unwrap();
                        if tile.color.a() != 1.0 {
                            transparent_list.push(*tile_ent);
                        } else {
                            opaque_list.push(*tile_ent);
                        }
                        tile_layer_hash.insert(*tile_ent, (ent, i));
                    }
                }
            }
        }

        // sort opaque front‐to‐back by layer, then atlas
        opaque_list.sort_by_key(|ent| {
            let atlas_handle = atlas_handles.get(*ent).unwrap_or_else(|| {
                atlas_handles
                    .get(tile_layer_hash.get(ent).unwrap().0)
                    .unwrap()
            });

            let layer = transforms
                .get(*ent)
                .unwrap_or_else(|| transforms.get(tile_layer_hash.get(ent).unwrap().0).unwrap())
                .translation
                .z;

            (layer.to_bits(), atlas_handle.atlas_id)
        });
        // sort transparent back‐to‐front, then atlas
        transparent_list.sort_by_key(|ent| {
            let atlas_handle = atlas_handles.get(*ent).unwrap_or_else(|| {
                atlas_handles
                    .get(tile_layer_hash.get(ent).unwrap().0)
                    .unwrap()
            });

            let layer = transforms
                .get(*ent)
                .unwrap_or_else(|| transforms.get(tile_layer_hash.get(ent).unwrap().0).unwrap())
                .translation
                .z;

            (!layer.to_bits(), atlas_handle.atlas_id)
        });

        /*
        println!(
            "Opaque sprites: {}, Transparent sprites: {}",
            opaque_list.len(),
            transparent_list.len()
        );
        */

        // Create index map for fast lookup
        let mut index_of = HashMap::with_capacity(opaque_list.len() + transparent_list.len());
        for (i, &e) in opaque_list.iter().chain(&transparent_list).enumerate() {
            index_of.insert(e, i as u32);
        }

        let mut sprite_lists = session.world.resource_mut::<SpriteLists>();
        sprite_lists.index_of = index_of;

        sprite_lists.opaque_list = opaque_list;
        sprite_lists.transparent_list = transparent_list;
        sprite_lists.tile_layer = tile_layer_hash;
    }
}

pub fn update_cameras_uniform(
    game: &mut bones::Game,
    dynamic_uniform: &mut DynamicBuffer,
    mut window_size: IVec2,
) {
    let mut instances: Vec<(&Ustr, Vec<(i32, CameraTransform, bones::Entity, Vec2)>)> = Vec::new();

    let queue = game.shared_resource_cell::<WgpuQueue>().unwrap();
    let device = game.shared_resource_cell::<WgpuDevice>().unwrap();

    for (session_name, session) in game.sessions.iter() {
        if !session.visible {
            continue;
        }

        let mut session_instances = Vec::new();

        let entities = session.world.resource::<bones::Entities>();
        let cameras = session.world.component::<bones::Camera>();
        let transforms = session.world.component::<bones::Transform>();

        let mut bitset = cameras.bitset().clone();
        bitset.bit_and(transforms.bitset());

        for ent in entities.iter_with_bitset(&bitset) {
            let Some(transform) = transforms.get(ent) else {
                unreachable!()
            };

            let Some(camera) = cameras.get(ent) else {
                unreachable!()
            };

            if !camera.active {
                continue;
            }

            let scale_ratio;
            if let Some(viewport) = camera.viewport.option() {
                //Get the viewport size, cropping it if needed
                window_size -= viewport.position.as_ivec2();
                window_size = IVec2::new(
                    (viewport.size.x as i32).min(window_size.x),
                    (viewport.size.y as i32).min(window_size.y),
                );

                if window_size.x <= 0 || window_size.y <= 0 {
                    continue;
                }

                if viewport.size.y <= viewport.size.x {
                    scale_ratio = Mat4::from_scale(Vec3::new(
                        window_size.x as f32 / viewport.size.x as f32,
                        viewport.size.y as f32 / viewport.size.x as f32 * window_size.y as f32
                            / viewport.size.y as f32,
                        1.,
                    ))
                    .inverse();
                } else {
                    scale_ratio = Mat4::from_scale(Vec3::new(
                        viewport.size.x as f32 / viewport.size.y as f32 * window_size.x as f32
                            / viewport.size.x as f32,
                        window_size.y as f32 / viewport.size.y as f32,
                        1.,
                    ))
                    .inverse();
                }
            } else if window_size.x <= 0 || window_size.y <= 0 {
                continue;
            } else if window_size.y <= window_size.x {
                scale_ratio = Mat4::from_scale(Vec3::new(
                    1.,
                    window_size.y as f32 / window_size.x as f32,
                    1.,
                ))
                .inverse();
            } else {
                scale_ratio = Mat4::from_scale(Vec3::new(
                    window_size.x as f32 / window_size.y as f32,
                    1.,
                    1.,
                ))
                .inverse();
            }

            let pixel_to_clip_space = Mat4::from_scale(Vec3::new(
                2.0 / window_size.x as f32,
                -2.0 / window_size.y as f32,
                1.0,
            )) * Mat4::from_translation(Vec3::new(-1.0, 1.0, 0.0));

            session_instances.push((
                camera.priority,
                CameraTransform{
                    transform: (/*Mat4::IDENTITYtransform.to_matrix((1.0 / window_size.as_vec2()).extend(1.0)) */ scale_ratio)
                        .to_cols_array_2d(),
                        screen_size: window_size.as_vec2().into(), ..Default::default()
                },
                ent,
                window_size.as_vec2()
            ));
        }

        session_instances.sort_by(|a, b| a.0.cmp(&b.0));
        instances.push((session_name, session_instances));
    }

    instances.sort_by(|a, b| a.0.cmp(&b.0));

    let device = device.borrow().unwrap();
    let queue = queue.borrow().unwrap();
    let mut cameras = game.shared_resource_mut::<Cameras>().unwrap();

    cameras.0 = Vec::new();
    let mut transforms = Vec::new();
    instances
        .iter()
        .for_each(|(session_name, session_instances)| {
            let mut entities = Vec::new();
            for (_, transform, entity, window_size) in session_instances {
                transforms.push(transform.clone());
                entities.push((*entity, *window_size));
            }

            cameras.0.push((**session_name, entities));
        });

    dynamic_uniform.write_pods(&device.get(), queue.get(), &transforms);
}

#[derive(bones_schema::HasSchema, Clone)]
#[schema(no_default)]
/// Camera entities sorted by `update_cameras_uniform` with their respective sizes.
pub struct Cameras(pub Vec<(Ustr, Vec<(bones::Entity, Vec2)>)>);

/* OLD
pub fn load_sprite(game: &mut bones::Game) {
    let assets = game.shared_resource_cell::<bones::AssetServer>().unwrap();
    let device = game.shared_resource_cell::<WgpuDevice>().unwrap();
    let queue = game.shared_resource_cell::<WgpuQueue>().unwrap();
    let texture_sender = game.shared_resource_cell::<TextureSender>().unwrap();
    let pixel_art = game.shared_resource_cell::<PixelArt>().unwrap();

    for (session_name, session) in game.sessions.iter_mut() {
        if !session.visible {
            continue;
        }

        let entities = session.world.resource::<bones::Entities>();
        let sprites = session.world.component::<bones::Sprite>();
        let mut buffers = session.world.component_mut::<AtlasSpriteBuffer>();
        let mut texture_loaded = session.world.component_mut::<TextureLoaded>();
        let transforms = session.world.component::<bones::Transform>();

        let mut not_loaded = texture_loaded.bitset().clone();
        not_loaded.bit_not();
        not_loaded.bit_and(sprites.bitset());

        for entity in entities.iter_with_bitset(&not_loaded) {
            let Some(sprite) = sprites.get(entity) else {
                unreachable!();
            };
            let Some(transform) = transforms.get(entity) else {
                panic!("No transform found!");
            };

            //Load and send texture
            let assets = assets.borrow().unwrap();
            let image = assets.get(sprite.image);
            if let bones::Image::Data(img) = &*image {
                let texture = Arc::new(
                    Texture::from_image(
                        device.borrow().unwrap().get(),
                        queue.borrow().unwrap().get(),
                        img,
                        None,
                        pixel_art.borrow().unwrap().0,
                    )
                    .unwrap(),
                );

                let base = BaseInstance {
                    color: sprite.color.as_rgba_f32(),
                    entity_type: 0,
                    transform: transform.to_matrix_none().to_cols_array_2d(),
                };
                let sprite_flags = SpriteFlags {
                    flip: [sprite.flip_x as u32, sprite.flip_y as u32]
                };

                let atlas_sprite_buffer =
                    Arc::new(device.borrow().unwrap().get().create_buffer_init(
                        &wgpu::util::BufferInitDescriptor {
                            label: Some("Atlas Sprite Buffer"),
                            contents: bytemuck::cast_slice(&[base]),
                            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                        },
                    ));

                //Add buffer to bones so we can update it
                buffers.insert(entity, AtlasSpriteBuffer(atlas_sprite_buffer.clone()));

                texture_sender
                    .borrow()
                    .unwrap()
                    .0
                    .send((texture, entity, atlas_sprite_buffer, *session_name))
                    .unwrap();
                texture_loaded.insert(entity, TextureLoaded);
            } else {
                unreachable!()
            };
        }
    }
}

pub fn load_atlas_sprite(game: &mut bones::Game) {
    let assets = game.shared_resource_cell::<bones::AssetServer>().unwrap();
    let device = game.shared_resource_cell::<WgpuDevice>().unwrap();
    let queue = game.shared_resource_cell::<WgpuQueue>().unwrap();
    let texture_sender = game.shared_resource_cell::<TextureSender>().unwrap();
    let pixel_art = game.shared_resource_cell::<PixelArt>().unwrap();

    for (session_name, session) in game.sessions.iter_mut() {
        if !session.visible {
            continue;
        }

        let entities = session.world.resource::<bones::Entities>();
        let atlas_sprites = session.world.component::<bones::AtlasSprite>();
        let mut buffers = session.world.component_mut::<AtlasSpriteBuffer>();
        let mut texture_loaded = session.world.component_mut::<TextureLoaded>();

        let mut not_loaded = texture_loaded.bitset().clone();
        not_loaded.bit_not();
        not_loaded.bit_and(atlas_sprites.bitset());

        for entity in entities.iter_with_bitset(&not_loaded) {
            let Some(atlas_sprite) = atlas_sprites.get(entity) else {
                unreachable!();
            };

            //Load and send texture
            let assets = assets.borrow().unwrap();
            let atlas = assets.get(atlas_sprite.atlas);
            let image = assets.get(atlas.image);
            if let bones::Image::Data(img) = &*image {
                let texture = Arc::new(
                    Texture::from_image(
                        device.borrow().unwrap().get(),
                        queue.borrow().unwrap().get(),
                        img,
                        None,
                        pixel_art.borrow().unwrap().0,
                    )
                    .unwrap(),
                );
                // create and send the atlas sprite uniform along with the texture and entity
                let uniform = AtlasSpriteUniform::from_atlas_sprite(
                    atlas_sprite,
                    &assets.get(atlas_sprite.atlas),
                );

                let atlas_sprite_buffer =
                    Arc::new(device.borrow().unwrap().get().create_buffer_init(
                        &wgpu::util::BufferInitDescriptor {
                            label: Some("Atlas Sprite Buffer"),
                            contents: bytemuck::cast_slice(&[uniform]),
                            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                        },
                    ));

                //Add buffer to bones so we can update it
                buffers.insert(entity, AtlasSpriteBuffer(atlas_sprite_buffer.clone()));

                texture_sender
                    .borrow()
                    .unwrap()
                    .0
                    .send((texture, entity, atlas_sprite_buffer, *session_name))
                    .unwrap();
                texture_loaded.insert(entity, TextureLoaded);
            } else {
                unreachable!()
            };
        }
    }
}

pub fn load_tile_sprite(game: &mut bones::Game) {
    let assets = game.shared_resource_cell::<bones::AssetServer>().unwrap();
    let device = game.shared_resource_cell::<WgpuDevice>().unwrap();
    let queue = game.shared_resource_cell::<WgpuQueue>().unwrap();
    let texture_sender = game.shared_resource_cell::<TextureSender>().unwrap();
    let pixel_art = game.shared_resource_cell::<PixelArt>().unwrap();

    for (session_name, session) in game.sessions.iter_mut() {
        if !session.visible {
            continue;
        }

        let entities = session.world.resource::<bones::Entities>();
        let tile_layers = session.world.component::<bones::TileLayer>();
        let tiles = session.world.component::<bones::Tile>();
        let mut buffers = session.world.component_mut::<AtlasSpriteBuffer>();
        let mut texture_loaded = session.world.component_mut::<TextureLoaded>();

        let mut not_loaded = texture_loaded.bitset().clone();
        not_loaded.bit_not();
        not_loaded.bit_and(tile_layers.bitset());

        for layer_ent in entities.iter_with_bitset(&not_loaded) {
            let Some(tile_layer) = tile_layers.get(layer_ent) else {
                unreachable!();
            };

            //Load and send texture
            let assets = assets.borrow().unwrap();
            let atlas = assets.get(tile_layer.atlas);
            let image = assets.get(atlas.image);
            if let bones::Image::Data(img) = &*image {
                let texture = Arc::new(
                    Texture::from_image(
                        device.borrow().unwrap().get(),
                        queue.borrow().unwrap().get(),
                        img,
                        None,
                        pixel_art.borrow().unwrap().0,
                    )
                    .unwrap(),
                );

                for (tile_pos_idx, tile) in tile_layer.tiles.iter().enumerate() {
                    let Some(tile_ent) = tile else {
                        continue;
                    };
                    let Some(tile) = tiles.get(*tile_ent) else {
                        panic!("Couldn't find tile entity!");
                    };
                    let mut transforms = session.world.component_mut::<bones::Transform>();

                    let transform = if let Some(t) = transforms.get_mut(*tile_ent) {
                        t
                    } else {
                        transforms.insert(*tile_ent, bones::Transform::default());
                        transforms.get_mut(*tile_ent).unwrap()
                    };

                    let tile_pos = tile_layer.pos(tile_pos_idx as u32);
                    let tile_offset = tile_pos.as_vec2() * tile_layer.tile_size;

                    /*let sprite_idx = tile.idx;
                    let y = sprite_idx / atlas.columns;
                    let x = sprite_idx - (y * atlas.columns);
                    let cell = Vec2::new(x as f32, y as f32);
                    let current_padding = atlas.padding
                        * Vec2::new(if x > 0 { 1.0 } else { 0.0 }, if y > 0 { 1.0 } else { 0.0 });
                    let min = (atlas.tile_size + current_padding) * cell + atlas.offset;
                    let rect = Rect {
                        min,
                        max: min + atlas.tile_size,
                    };*/

                    transform.translation += tile_offset.extend(0.0);
                    // Scale up slightly to avoid bleeding between tiles.
                    // TODO: Improve tile rendering
                    // Currently we do a small hack here, scaling up the tiles a little bit, to prevent
                    // visible gaps between tiles. This solution isn't perfect and we probably need to
                    // create a proper tile renderer. That can render multiple tiles on one quad instead
                    // of using a separate quad for each tile.
                    transform.scale += Vec3::new(0.01, 0.01, 0.0);

                    // create and send the atlas sprite uniform along with the texture and entity
                    let uniform =
                        AtlasSpriteUniform::from_tile(tile, &assets.get(tile_layer.atlas));

                    let atlas_sprite_buffer =
                        Arc::new(device.borrow().unwrap().get().create_buffer_init(
                            &wgpu::util::BufferInitDescriptor {
                                label: Some("Atlas Sprite Buffer"),
                                contents: bytemuck::cast_slice(&[uniform]),
                                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                            },
                        ));

                    //Add buffer to bones so we can update it
                    buffers.insert(*tile_ent, AtlasSpriteBuffer(atlas_sprite_buffer.clone()));

                    texture_sender
                        .borrow()
                        .unwrap()
                        .0
                        .send((
                            texture.clone(),
                            *tile_ent,
                            atlas_sprite_buffer,
                            *session_name,
                        ))
                        .unwrap();
                }
            } else {
                unreachable!()
            };
            texture_loaded.insert(layer_ent, TextureLoaded);
        }
    }
}

// System for updating atlas uniforms
pub fn update_atlas_uniforms(game: &mut bones::Game) {
    let assets = game.shared_resource_cell::<bones::AssetServer>().unwrap();
    let queue = game.shared_resource_cell::<WgpuQueue>().unwrap();

    for (_, session) in game.sessions.iter_mut() {
        if !session.visible {
            continue;
        }

        let entities = session.world.resource::<bones::Entities>();
        let atlases = session.world.component::<bones::AtlasSprite>();
        let mut buffers = session.world.component_mut::<AtlasSpriteBuffer>();

        for (_, (atlas_sprite, atlas_sprite_buffer)) in entities.iter_with((&atlases, &mut buffers))
        {
            let assets = assets.borrow().unwrap();
            let atlas = assets.get(atlas_sprite.atlas).clone();
            let uniform = AtlasSpriteUniform::from_atlas_sprite(atlas_sprite, &atlas);
            queue.borrow().unwrap().get().write_buffer(
                &atlas_sprite_buffer.0,
                0,
                bytemuck::bytes_of(&uniform),
            );
        }
    }
}

// System for updating sprite uniforms
pub fn update_sprite_uniforms(game: &mut bones::Game) {
    let queue = game.shared_resource_cell::<WgpuQueue>().unwrap();

    for (_, session) in game.sessions.iter_mut() {
        if !session.visible {
            continue;
        }

        let entities = session.world.resource::<bones::Entities>();
        let sprites = session.world.component::<bones::Sprite>();
        let mut buffers = session.world.component_mut::<RenderBuffers>();
        let transform = session.world.component::<bones::Transform>();

        for (_, (sprite, atlas_sprite_buffer)) in entities.iter_with((&sprites, &mut buffers)) {
            let base = BaseInstance {
                color: sprite.color.as_rgba_f32(),
                entity_type: 0,
                transform: transform.to_matrix_none().to_cols_array_2d(),
            };
            let sprite_flags = SpriteFlags {
                flip: [sprite.flip_x as u32, sprite.flip_y as u32],
            };

            queue.borrow().unwrap().get().write_buffer(
                &atlas_sprite_buffer.0,
                0,
                bytemuck::bytes_of(&uniform),
            );
        }
    }
}

// System for updating tiles uniforms
pub fn update_tiles_uniforms(game: &mut bones::Game) {
    let assets = game.shared_resource_cell::<bones::AssetServer>().unwrap();
    let queue = game.shared_resource_cell::<WgpuQueue>().unwrap();

    for (_, session) in game.sessions.iter_mut() {
        if !session.visible {
            continue;
        }

        let entities = session.world.resource::<bones::Entities>();
        let tile_layers = session.world.component::<bones::TileLayer>();
        let tiles = session.world.component::<bones::Tile>();
        let mut buffers = session.world.component_mut::<AtlasSpriteBuffer>();

        for (_, (tile_layer, atlas_sprite_buffer)) in
            entities.iter_with((&tile_layers, &mut buffers))
        {
            let assets = assets.borrow().unwrap();
            let atlas = assets.get(tile_layer.atlas).clone();
            for tile in &tile_layer.tiles {
                let Some(tile) = tile else {
                    continue;
                };
                let Some(tile) = tiles.get(*tile) else {
                    panic!("Couldn't find tile entity!");
                };

                let uniform = AtlasSpriteUniform::from_tile(tile, &atlas);
                queue.borrow().unwrap().get().write_buffer(
                    &atlas_sprite_buffer.0,
                    0,
                    bytemuck::bytes_of(&uniform),
                );
            }
        }
    }
}
*/
