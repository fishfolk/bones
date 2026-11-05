use crate::{
    atlas_pool::{AtlasAllocationGuard, AtlasPool, CachedAtlasAllocation},
    *,
};
use bones_framework::prelude::{self as bones, BitSet, ComponentIterBitset, Ustr};
use guillotiere::Allocation;
use std::{collections::HashMap, ops::Range};

// Functions used to load sprites, atlas sprites and tile sprites, and update them.

/// Per-camera uniform padded to 256 bytes so each camera can be bound with a WebGPU dynamic offset.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraTransform {
    pub transform: [[f32; 4]; 4],
    pub screen_size: [f32; 2],
    pub _pad0: u32,
    pub _pad1: u32,
    pub _align_pad: [[u32; 4]; 11],
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
        image_size: Vec2,
    ) -> Self {
        Self {
            tile_size: atlas.tile_size.into(),
            columns: atlas.columns,
            padding: atlas.padding.into(),
            offset: atlas.offset.into(),
            index: atlas_sprite.index,
            image_size: image_size.into(),
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

    pub fn from_tile(
        tile: &bones::Tile,
        atlas: &bones::Atlas,
        transform: &bones::Transform,
        uvs: ([f32; 2], [f32; 2]),
        image_size: Vec2,
        index: usize,
        tile_layer: &bones::TileLayer,
    ) -> Self {
        let mut transform = *transform;
        let local_offset = Vec3::new(
            atlas.tile_size.x * ((index as u32 % tile_layer.grid_size.x) as f32 + 0.5),
            atlas.tile_size.y * ((index as u32 / tile_layer.grid_size.x) as f32 + 0.5),
            0.0,
        );
        transform.translation += transform.rotation * (local_offset * transform.scale);
        // Scale up slightly to avoid seam bleeding between adjacent tiles, matching Bevy renderer.
        transform.scale += Vec3::new(0.01, 0.01, 0.0);

        Self {
            tile_size: atlas.tile_size.into(),
            columns: atlas.columns,
            padding: atlas.padding.into(),
            offset: atlas.offset.into(),
            index: tile.idx,
            image_size: image_size.into(),
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
    #[allow(dead_code)]
    pub alloc: Allocation,
    pub uv_min: [f32; 2],     // Atlas UV coordinates (min_x, min_y)
    pub uv_max: [f32; 2],     // Atlas UV coordinates (max_x, max_y)
    pub image_size: [f32; 2], // Original image size (width, height)
    pub image: bones::Handle<bones::Image>,
    pub cid: bones::Cid,
    _guard: Arc<AtlasAllocationGuard>,
}

fn reclaim_dropped_atlas_allocations(atlas_pool: &mut AtlasPool) {
    while let Ok((image, cid, atlas_id, alloc)) = atlas_pool.receiver.try_recv() {
        log::debug!(
            "Reclaiming dropped texture allocation from atlas {}: {:?}",
            atlas_id,
            image
        );
        if atlas_pool
            .cache
            .get(&(image, cid))
            .is_some_and(|c| c.guard.strong_count() == 0)
        {
            atlas_pool.cache.remove(&(image, cid));
        }
        atlas_pool.deallocate(atlas_id, alloc);
    }
}

pub fn update_atlas_pool(game: &mut bones::Game, atlas_pool: &mut AtlasPool) {
    let Some(assets) = game.shared_resource_cell::<bones::AssetServer>() else {
        return;
    };
    let queue = game.shared_resource_cell::<WgpuQueue>().unwrap();

    // First, deallocate any atlas allocations whose last `AtlasPoolHandle` was dropped.
    reclaim_dropped_atlas_allocations(atlas_pool);

    for (_session_name, session) in game.sessions.iter_mut() {
        if !session.active && !session.visible {
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

        // Check existing handles to see if the entity lost its sprite component or changed its image/cid.
        let existing_handle_bitset = atlas_pool_handles.bitset().clone();
        let mut stale_entities = Vec::new();
        for ent in entities.iter_with_bitset(&existing_handle_bitset) {
            let current_image = if let Some(sprite) = sprites.get(ent) {
                (sprite.image != bones::Handle::default()).then_some(sprite.image)
            } else if let Some(atlas_sprite) = atlases.get(ent) {
                if atlas_sprite.atlas == bones::Handle::default() {
                    None
                } else {
                    let assets = assets.borrow().unwrap();
                    assets
                        .try_get(atlas_sprite.atlas)
                        .and_then(|a| a.ok())
                        .map(|a| a.image)
                }
            } else if let Some(tile_layer) = tile_layers.get(ent) {
                if tile_layer.atlas == bones::Handle::default() {
                    None
                } else {
                    let assets = assets.borrow().unwrap();
                    assets
                        .try_get(tile_layer.atlas)
                        .and_then(|a| a.ok())
                        .map(|a| a.image)
                }
            } else {
                None
            };

            let current_cid = current_image.and_then(|img| {
                let assets = assets.borrow().unwrap();
                assets
                    .store
                    .asset_ids
                    .get(&img.untyped())
                    .map(|entry| *entry.value())
            });

            let handle_matches = atlas_pool_handles
                .get(ent)
                .zip(current_image)
                .zip(current_cid)
                .is_some_and(|((h, img), cid)| h.image == img && h.cid == cid);
            if !handle_matches {
                stale_entities.push(ent);
            }
        }
        for ent in stale_entities {
            atlas_pool_handles.remove(ent);
        }

        // Reclaim any allocations that were just removed above before allocating new ones.
        reclaim_dropped_atlas_allocations(atlas_pool);

        let mut not_atlas_handle_bitset = atlas_pool_handles.bitset().clone();
        not_atlas_handle_bitset.bit_not();

        let mut without_handle = bitset.clone();
        without_handle.bit_and(&not_atlas_handle_bitset);

        for ent in entities.iter_with_bitset(&without_handle) {
            let image_handle;
            if let Some(sprite) = sprites.get(ent) {
                if sprite.image == bones::Handle::default() {
                    continue;
                }
                image_handle = sprite.image;
            } else if let Some(atlas_sprite) = atlases.get(ent) {
                if atlas_sprite.atlas == bones::Handle::default() {
                    continue;
                }
                let assets = assets.borrow().unwrap();
                let Some(Ok(atlas)) = assets.try_get(atlas_sprite.atlas) else {
                    continue;
                };
                if atlas.image == bones::Handle::default() {
                    continue;
                }
                image_handle = atlas.image;
            } else {
                let tile_layer = tile_layers.get(ent).unwrap();
                if tile_layer.atlas == bones::Handle::default() {
                    continue;
                }
                let assets = assets.borrow().unwrap();
                let Some(Ok(atlas)) = assets.try_get(tile_layer.atlas) else {
                    continue;
                };
                if atlas.image == bones::Handle::default() {
                    continue;
                }
                image_handle = atlas.image;
            }
            let assets = assets.borrow().unwrap();
            let Some(cid) = assets
                .store
                .asset_ids
                .get(&image_handle.untyped())
                .map(|entry| *entry.value())
            else {
                continue;
            };

            // Reuse existing GPU atlas allocation if another entity already uploaded this (image, cid).
            if let Some(cached) = atlas_pool.cache.get(&(image_handle, cid)) {
                if let Some(guard) = cached.guard.upgrade() {
                    atlas_pool_handles.insert(
                        ent,
                        AtlasPoolHandle {
                            atlas_id: cached.atlas_id,
                            alloc: cached.alloc,
                            uv_min: cached.uv_min,
                            uv_max: cached.uv_max,
                            image_size: cached.image_size,
                            image: image_handle,
                            cid,
                            _guard: guard,
                        },
                    );
                    continue;
                }
            }

            let image = assets.get(image_handle);

            if let bones::Image::Data(img) = &*image {
                let (atlas_id, alloc) = atlas_pool
                    .allocate((img.width() as i32, img.height() as i32))
                    .unwrap_or_else(|_| {
                        panic!("Failed to allocate space for sprite image: {:?}", image);
                    });

                let rect: guillotiere::euclid::Box2D<i32, guillotiere::euclid::UnknownUnit> =
                    alloc.rectangle;
                let (atlas_w, atlas_h) = (
                    atlas_pool.atlas_size.0 as f32,
                    atlas_pool.atlas_size.1 as f32,
                );
                let uv_min = [rect.min.x as f32 / atlas_w, rect.min.y as f32 / atlas_h];
                let uv_max = [rect.max.x as f32 / atlas_w, rect.max.y as f32 / atlas_h];
                let image_size = [img.width() as f32, img.height() as f32];

                let guard = Arc::new(AtlasAllocationGuard {
                    image: image_handle,
                    cid,
                    atlas_id,
                    alloc,
                    sender: atlas_pool.sender.clone(),
                });

                atlas_pool.cache.insert(
                    (image_handle, cid),
                    CachedAtlasAllocation {
                        atlas_id,
                        alloc,
                        uv_min,
                        uv_max,
                        image_size,
                        guard: Arc::downgrade(&guard),
                    },
                );

                atlas_pool_handles.insert(
                    ent,
                    AtlasPoolHandle {
                        atlas_id,
                        alloc,
                        uv_min,
                        uv_max,
                        image_size,
                        image: image_handle,
                        cid,
                        _guard: guard,
                    },
                );

                // 1) Convert to RGBA8 and grab the raw bytes
                let rgba = img.to_rgba8();
                let (w, h) = (rgba.width(), rgba.height());
                let raw = rgba.into_raw();

                // 2) Compute the origin in the atlas texture
                let origin = wgpu::Origin3d {
                    x: rect.min.x as u32,
                    y: rect.min.y as u32,
                    z: 0,
                };

                let unpadded_stride = (4 * w) as usize;
                let padded_stride = unpadded_stride.div_ceil(256) * 256;
                let mut padded_data = vec![0u8; padded_stride * h as usize];

                // copy each scanline into the padded buffer
                for row in 0..(h as usize) {
                    let src_offset = row * unpadded_stride;
                    let dst_offset = row * padded_stride;
                    padded_data[dst_offset..dst_offset + unpadded_stride]
                        .copy_from_slice(&raw[src_offset..src_offset + unpadded_stride]);
                }

                log::debug!(
                    "Uploaded sprite to atlas {} at {:?}..{:?}",
                    atlas_id,
                    rect.min,
                    rect.max
                );

                let data_layout = wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_stride as u32),
                    rows_per_image: Some(h),
                };
                let queue = queue.borrow().unwrap();
                queue.get().write_texture(
                    wgpu::TexelCopyTextureInfo {
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
            }
        }
    }
}

pub fn update_uniforms(
    game: &mut bones::Game,
    dynamic_storage: &mut DynamicBuffer,
    line_buffers: &mut line::LineBuffers,
) {
    let mut instances = Vec::new();
    line_buffers.session_draws.clear();
    line_buffers.camera_bg_draws.clear();
    let mut all_line_vertices: Vec<line::Vertex> = Vec::new();
    let mut all_line_indices: Vec<u32> = Vec::new();
    let mut viewport_bg_quad_range: Option<(i32, Range<u32>)> = None;

    let queue = game.shared_resource_cell::<WgpuQueue>().unwrap();
    let device = game.shared_resource_cell::<WgpuDevice>().unwrap();
    let assets = game.shared_resource_cell::<bones::AssetServer>();

    for (session_name, session) in game.sessions.iter_mut() {
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
        let cameras = session.world.component::<bones::Camera>();
        let clear_color = session.world.get_resource::<bones::ClearColor>();
        let atlas_handles = session.world.component::<AtlasPoolHandle>();

        // Emit clip-space background clear quads for any active camera with a custom viewport
        // and `draw_background_color == true`.
        for (camera_ent, (camera, _)) in entities.iter_with((&cameras, &transforms)) {
            if !camera.active || !camera.draw_background_color || camera.viewport.option().is_none()
            {
                continue;
            }

            let color = match (camera.background_color.option(), clear_color.as_deref()) {
                (Some(color), _) => color,
                (None, Some(color)) => color.0,
                (None, None) => bones::Color::BLACK,
            };

            let (base_vertex, index_range) = viewport_bg_quad_range
                .get_or_insert_with(|| {
                    let bv = all_line_vertices.len() as i32;
                    let is = all_line_indices.len() as u32;
                    let white = [1.0, 1.0, 1.0, 1.0];
                    all_line_vertices.extend_from_slice(&[
                        line::Vertex {
                            position: [-1.0, -1.0],
                            color: white,
                        },
                        line::Vertex {
                            position: [1.0, -1.0],
                            color: white,
                        },
                        line::Vertex {
                            position: [1.0, 1.0],
                            color: white,
                        },
                        line::Vertex {
                            position: [-1.0, 1.0],
                            color: white,
                        },
                    ]);
                    all_line_indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
                    let ie = all_line_indices.len() as u32;
                    (bv, is..ie)
                })
                .clone();

            let instance_index = instances.len() as u32;
            instances.push(AtlasSpriteUniform {
                entity_type: 3,
                color_tint: color.as_rgba_f32(),
                ..Default::default()
            });

            line_buffers.camera_bg_draws.insert(
                (*session_name, camera_ent),
                line::LineDrawCall {
                    index_range,
                    base_vertex,
                    instance_index,
                },
            );
        }

        let mut batches: Vec<(usize, Range<u32>)> = Vec::new();

        for entity in &sprite_lists.sprite_list {
            let transform = transforms.get(*entity).unwrap_or_else(|| {
                transforms
                    .get(sprite_lists.tile_layer.get(entity).unwrap().0)
                    .unwrap()
            });

            let instance_idx = instances.len() as u32;
            let atlas_id: usize;

            if let Some(sprite) = sprites.get(*entity) {
                let Some(atlas_handle) = atlas_handles.get(*entity) else {
                    panic!("Texture not loaded in atlas pool!")
                };
                atlas_id = atlas_handle.atlas_id;
                let (uv_min, uv_max) = (atlas_handle.uv_min, atlas_handle.uv_max);
                let image_size = Vec2::new(atlas_handle.image_size[0], atlas_handle.image_size[1]);

                instances.push(AtlasSpriteUniform::from_sprite(
                    sprite,
                    transform,
                    (uv_min, uv_max),
                    image_size,
                ));
            } else if let Some(atlas_sprite) = atlases.get(*entity) {
                let assets = assets.as_ref().unwrap().borrow().unwrap();
                let atlas = assets.get(atlas_sprite.atlas).clone();

                let Some(atlas_handle) = atlas_handles.get(*entity) else {
                    panic!("Texture not loaded in atlas pool!")
                };
                atlas_id = atlas_handle.atlas_id;
                let (uv_min, uv_max) = (atlas_handle.uv_min, atlas_handle.uv_max);
                let image_size = Vec2::new(atlas_handle.image_size[0], atlas_handle.image_size[1]);

                instances.push(AtlasSpriteUniform::from_atlas_sprite(
                    atlas_sprite,
                    &atlas,
                    transform,
                    (uv_min, uv_max),
                    image_size,
                ));
            } else if let Some(tile) = tiles.get(*entity) {
                let tile_layer_ent = sprite_lists.tile_layer.get(entity).unwrap();
                let tile_layer = tile_layers.get(tile_layer_ent.0).unwrap();
                let assets = assets.as_ref().unwrap().borrow().unwrap();
                let atlas = assets.get(tile_layer.atlas).clone();

                let Some(atlas_handle) = atlas_handles.get(tile_layer_ent.0) else {
                    panic!("Texture not loaded in atlas pool!")
                };
                atlas_id = atlas_handle.atlas_id;
                let (uv_min, uv_max) = (atlas_handle.uv_min, atlas_handle.uv_max);
                let image_size = Vec2::new(atlas_handle.image_size[0], atlas_handle.image_size[1]);

                instances.push(AtlasSpriteUniform::from_tile(
                    tile,
                    &atlas,
                    transform,
                    (uv_min, uv_max),
                    image_size,
                    tile_layer_ent.1,
                    tile_layer,
                ));
            } else {
                unreachable!()
            }

            if let Some((last_atlas, range)) = batches.last_mut() {
                if *last_atlas == atlas_id && range.end == instance_idx {
                    range.end = instance_idx + 1;
                    continue;
                }
            }
            batches.push((atlas_id, instance_idx..instance_idx + 1));
        }

        drop(sprite_lists);
        let mut sprite_lists = session.world.resource_mut::<SpriteLists>();
        sprite_lists.batches = batches;
        drop(sprite_lists);

        let mut path_entities: Vec<(&bones::Path2d, &bones::Transform)> =
            entities.iter_with((&paths, &transforms)).map(|(_, pt)| pt).collect();
        path_entities.sort_by(|a, b| a.1.translation.z.total_cmp(&b.1.translation.z));

        let mut session_line_draws = Vec::with_capacity(path_entities.len());
        for (path, transform) in path_entities {
            let geometry = line::tessellate_path(path);
            if geometry.indices.is_empty() || geometry.vertices.is_empty() {
                continue;
            }

            let instance_index = instances.len() as u32;
            let mut transform = *transform;
            // Offset the path towards the camera slightly to make sure it renders on top of a
            // sprite/etc. if it is applied to an entity with both a sprite and a path.
            transform.translation.z += 0.0001;

            instances.push(AtlasSpriteUniform {
                entity_type: 2,
                transform: transform.to_matrix_none().to_cols_array_2d(),
                color_tint: path.color.as_rgba_f32(),
                ..Default::default()
            });

            let base_vertex = all_line_vertices.len() as i32;
            let index_start = all_line_indices.len() as u32;
            all_line_vertices.extend_from_slice(&geometry.vertices);
            all_line_indices.extend_from_slice(&geometry.indices);
            let index_end = all_line_indices.len() as u32;

            session_line_draws.push(line::LineDrawCall {
                index_range: index_start..index_end,
                base_vertex,
                instance_index,
            });
        }
        if !session_line_draws.is_empty() {
            line_buffers
                .session_draws
                .insert(*session_name, session_line_draws);
        }
    }

    let device = device.borrow().unwrap();
    let queue = queue.borrow().unwrap();

    log::trace!("Updated sprite instances: {}", instances.len());

    // Update buffers (with dynamic resizing)
    dynamic_storage.write_pods(device.get(), queue.get(), &instances);
    line_buffers.write_buffers(
        device.get(),
        queue.get(),
        &all_line_vertices,
        &all_line_indices,
    );
}

#[derive(bones_schema::HasSchema, Default, Clone)]
/// Holds sorted sprite entities and precomputed instanced draw batches `(atlas_id, instance_range)`.
pub struct SpriteLists {
    pub sprite_list: Vec<bones::Entity>,
    pub batches: Vec<(usize, Range<u32>)>,
    pub tile_layer: HashMap<bones::Entity, (bones::Entity, usize)>,
}

pub fn sort_sprites(game: &mut bones::Game) {
    for (_, session) in game.sessions.iter_mut() {
        if !session.visible {
            continue;
        }

        {
            let mut sprite_lists = session.world.init_resource::<SpriteLists>();
            sprite_lists.sprite_list.clear();
            sprite_lists.batches.clear();
        }

        // Now safe to borrow entities immutably
        let entities = session.world.resource::<bones::Entities>();

        let atlas_handles = session.world.component::<AtlasPoolHandle>();
        let transforms = session.world.component::<bones::Transform>();

        let sprites = session.world.component::<bones::Sprite>();
        let atlases = session.world.component::<bones::AtlasSprite>();
        let tile_layers = session.world.component::<bones::TileLayer>();
        let tiles = session.world.component::<bones::Tile>();

        // Get entities with atlas handle, and that have sprite, atlas sprite, or tile layer
        let mut sprites_atlases = sprites.bitset().clone();
        sprites_atlases.bit_or(atlases.bitset());
        sprites_atlases.bit_or(tile_layers.bitset());
        let mut bitset = atlas_handles.bitset().clone();
        bitset.bit_and(&sprites_atlases);
        bitset.bit_and(transforms.bitset());

        let mut sprite_list: Vec<bones::Entity> = Vec::new();
        let mut tile_layer_hash = HashMap::with_capacity(tiles.bitset().len());

        for ent in entities.iter_with_bitset(&bitset) {
            if sprites.get(ent).is_some() || atlases.get(ent).is_some() {
                sprite_list.push(ent);
            } else if let Some(tile_layer) = tile_layers.get(ent) {
                for (i, tile) in tile_layer.tiles.iter().enumerate() {
                    if let Some(tile_ent) = tile {
                        if tiles.get(*tile_ent).is_some() {
                            sprite_list.push(*tile_ent);
                            tile_layer_hash.insert(*tile_ent, (ent, i));
                        }
                    }
                }
            }
        }

        // Stable sort back-to-front (ascending Z) by layer, then by atlas_id for instanced batching.
        // Because `Vec::sort_by` is stable and textures are deduplicated into shared atlases,
        // overlapping sprites at the same Z preserve deterministic entity order.
        sprite_list.sort_by(|a, b| {
            let atlas_a = atlas_handles
                .get(*a)
                .unwrap_or_else(|| {
                    atlas_handles
                        .get(tile_layer_hash.get(a).unwrap().0)
                        .unwrap()
                })
                .atlas_id;
            let atlas_b = atlas_handles
                .get(*b)
                .unwrap_or_else(|| {
                    atlas_handles
                        .get(tile_layer_hash.get(b).unwrap().0)
                        .unwrap()
                })
                .atlas_id;

            let layer_a = transforms
                .get(*a)
                .unwrap_or_else(|| transforms.get(tile_layer_hash.get(a).unwrap().0).unwrap())
                .translation
                .z;
            let layer_b = transforms
                .get(*b)
                .unwrap_or_else(|| transforms.get(tile_layer_hash.get(b).unwrap().0).unwrap())
                .translation
                .z;

            layer_a
                .total_cmp(&layer_b)
                .then_with(|| atlas_a.cmp(&atlas_b))
        });

        let mut sprite_lists = session.world.resource_mut::<SpriteLists>();
        sprite_lists.sprite_list = sprite_list;
        sprite_lists.tile_layer = tile_layer_hash;

        if log::log_enabled!(log::Level::Debug) {
            log::debug!(
                "SpriteLists: sprites={}, tiles={}",
                sprite_lists.sprite_list.len(),
                sprite_lists.tile_layer.len(),
            );
        }
    }
}

pub fn camera_view_size(camera: &bones::Camera, viewport_size: Vec2) -> (f32, f32) {
    let aspect = viewport_size.x / viewport_size.y;
    match camera.size {
        bones::CameraSize::Fixed { width, height } => (width, height),
        bones::CameraSize::FixedHeight(h) => (h * aspect, h),
        bones::CameraSize::FixedWidth(w) => (w, w / aspect),
        bones::CameraSize::Window { pixels_per_unit } => (
            viewport_size.x / pixels_per_unit,
            viewport_size.y / pixels_per_unit,
        ),
        bones::CameraSize::Min {
            min_width,
            min_height,
        } => {
            if viewport_size.x * min_height > min_width * viewport_size.y {
                (min_height * aspect, min_height)
            } else {
                (min_width, min_width / aspect)
            }
        }
        bones::CameraSize::Max {
            max_width,
            max_height,
        } => {
            if viewport_size.x * max_height < max_width * viewport_size.y {
                (max_height * aspect, max_height)
            } else {
                (max_width, max_width / aspect)
            }
        }
    }
}

/// Converts a physical screen cursor position `(x, y)` (top-left origin) to 2D world space
/// using the active camera.
pub fn screen_to_world_2d(
    game: &bones::Game,
    screen_pos: Vec2,
    window_size: Vec2,
) -> Option<Vec2> {
    if window_size.x <= 0.0 || window_size.y <= 0.0 {
        return None;
    }

    let mut candidates: Vec<(i32, Ustr, i32, bones::Camera, bones::Transform)> = Vec::new();
    for (session_name, session) in game.sessions.iter() {
        if !session.visible {
            continue;
        }
        let entities = session.world.resource::<bones::Entities>();
        let cameras = session.world.component::<bones::Camera>();
        let transforms = session.world.component::<bones::Transform>();

        for (_ent, (camera, transform)) in entities.iter_with((&cameras, &transforms)) {
            if !camera.active {
                continue;
            }
            candidates.push((
                session.priority,
                *session_name,
                camera.priority,
                camera.clone(),
                *transform,
            ));
        }
    }

    if candidates.is_empty() {
        return None;
    }

    candidates.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| a.1.cmp(&b.1))
            .then_with(|| a.2.cmp(&b.2))
    });

    // Prefer the highest-priority camera whose viewport contains `screen_pos`,
    // falling back to the highest-priority active camera overall.
    let chosen = candidates
        .iter()
        .rev()
        .find(|(_, _, _, camera, _)| {
            let (vp_pos, vp_size) = camera
                .viewport
                .option()
                .map(|v| (v.position.as_vec2(), v.size.as_vec2()))
                .unwrap_or((Vec2::ZERO, window_size));
            screen_pos.x >= vp_pos.x
                && screen_pos.x <= vp_pos.x + vp_size.x
                && screen_pos.y >= vp_pos.y
                && screen_pos.y <= vp_pos.y + vp_size.y
        })
        .or_else(|| candidates.last())?;

    let (_, _, _, ref camera, ref transform) = *chosen;
    let (viewport_pos, viewport_size) = camera
        .viewport
        .option()
        .map(|v| (v.position.as_vec2(), v.size.as_vec2()))
        .unwrap_or((Vec2::ZERO, window_size));
    if viewport_size.x <= 0.0 || viewport_size.y <= 0.0 {
        return None;
    }

    let (camera_width_px, camera_height_px) = camera_view_size(camera, viewport_size);
    let rel = screen_pos - viewport_pos;
    let ndc = Vec2::new(
        (rel.x / viewport_size.x) * 2.0 - 1.0,
        1.0 - (rel.y / viewport_size.y) * 2.0,
    );
    let local = Vec3::new(
        ndc.x * (camera_width_px * 0.5),
        ndc.y * (camera_height_px * 0.5),
        0.0,
    );
    let world = (transform.rotation * local) + transform.translation;
    Some(world.truncate())
}

pub fn update_cameras_uniform(
    game: &mut bones::Game,
    dynamic_uniform: &mut DynamicBuffer,
    window_size: IVec2,
) {
    // NOTE: This must match `pixel_scale` in `atlas_sprite.wgsl`.
    // It is used to keep world-space values (in pixels) numerically small on the GPU.
    const PIXEL_SCALE: f32 = 0.001;

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

            // IMPORTANT: treat the input `window_size` as the base size for each camera.
            // Cameras may have viewports (and there may be multiple cameras per session),
            // so we must not shrink `window_size` cumulatively across iterations.
            let viewport_size = camera
                .viewport
                .option()
                .map(|v| v.size.as_vec2())
                .unwrap_or(window_size.as_vec2());
            if viewport_size.x <= 0.0 || viewport_size.y <= 0.0 {
                continue;
            }

            // Compute the camera view size in game pixels, then scale to the shader's "pixel units".
            let (camera_width_px, camera_height_px) = camera_view_size(camera, viewport_size);

            let camera_width = camera_width_px * PIXEL_SCALE;
            let camera_height = camera_height_px * PIXEL_SCALE;

            // Build a proper view-projection matrix.
            // Sprites use X/Y in scaled pixel units (PIXEL_SCALE) but Z in world units, so the
            // camera translation must match that convention.
            let mut cam_translation = transform.translation;
            cam_translation.x *= PIXEL_SCALE;
            cam_translation.y *= PIXEL_SCALE;

            let cam_transform = bones::Transform {
                translation: cam_translation,
                rotation: transform.rotation,
                scale: Vec3::ONE,
            };

            let view = cam_transform.to_matrix_none().inverse();
            // Wide near/far to avoid clipping typical 2D layer Z values.
            let proj = Mat4::orthographic_rh(
                -camera_width * 0.5,
                camera_width * 0.5,
                -camera_height * 0.5,
                camera_height * 0.5,
                -10_000.0,
                10_000.0,
            );
            let camera_to_clip = proj * view;

            session_instances.push((
                camera.priority,
                CameraTransform {
                    transform: camera_to_clip.to_cols_array_2d(),
                    screen_size: viewport_size.into(),
                    ..Default::default()
                },
                ent,
                viewport_size,
            ));
        }

        session_instances.sort_by(|a, b| a.0.cmp(&b.0));
        instances.push((session_name, session_instances));
    }

    instances.sort_by(|a, b| {
        let prio_a = game.sessions.get(*a.0).map(|s| s.priority).unwrap_or(0);
        let prio_b = game.sessions.get(*b.0).map(|s| s.priority).unwrap_or(0);
        prio_a.cmp(&prio_b).then_with(|| a.0.cmp(b.0))
    });

    let device = device.borrow().unwrap();
    let queue = queue.borrow().unwrap();
    let mut cameras = game.shared_resource_mut::<Cameras>();

    cameras.0 = Vec::new();
    let mut transforms = Vec::new();
    instances
        .iter()
        .for_each(|(session_name, session_instances)| {
            let mut entities = Vec::new();
            for (_, transform, entity, window_size) in session_instances {
                transforms.push(*transform);
                entities.push((*entity, *window_size));
            }

            cameras.0.push((**session_name, entities));
        });

    dynamic_uniform.write_pods(device.get(), queue.get(), &transforms);
}

#[derive(bones_schema::HasSchema, Clone)]
#[schema(no_default)]
/// Camera entities sorted by `update_cameras_uniform` with their respective sizes.
pub struct Cameras(pub Vec<(Ustr, Vec<(bones::Entity, Vec2)>)>);
