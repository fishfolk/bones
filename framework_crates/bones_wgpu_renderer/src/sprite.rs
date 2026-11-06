use crate::{
    atlas_pool::{build_extruded_rgba_upload, AtlasPool, CachedAtlasAllocation},
    *,
};
use bones_framework::prelude::{self as bones, Ustr};
use std::{collections::HashSet, ops::Range};

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
        let max_tiles = atlas.columns.saturating_mul(atlas.rows);
        let clamped_index = if max_tiles > 0 {
            atlas_sprite.index.min(max_tiles - 1)
        } else {
            0
        };

        Self {
            tile_size: atlas.tile_size.into(),
            columns: atlas.columns,
            padding: atlas.padding.into(),
            offset: atlas.offset.into(),
            index: clamped_index,
            image_size: image_size.into(),
            entity_type: 1,
            flip_x: atlas_sprite.flip_x as u32,
            flip_y: atlas_sprite.flip_y as u32,
            color_tint: convert::color_to_linear_rgba_f32(atlas_sprite.color),
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
        let grid_w = tile_layer.grid_size.x.max(1);
        let local_offset = Vec3::new(
            atlas.tile_size.x * ((index as u32 % grid_w) as f32 + 0.5),
            atlas.tile_size.y * ((index as u32 / grid_w) as f32 + 0.5),
            0.0,
        );
        transform.translation += transform.rotation * (local_offset * transform.scale);
        // Scale up slightly to avoid seam bleeding between adjacent tiles, matching Bevy renderer.
        transform.scale += Vec3::new(0.01, 0.01, 0.0);

        let max_tiles = atlas.columns.saturating_mul(atlas.rows);
        let clamped_index = if max_tiles > 0 {
            tile.idx.min(max_tiles - 1)
        } else {
            0
        };

        Self {
            tile_size: atlas.tile_size.into(),
            columns: atlas.columns,
            padding: atlas.padding.into(),
            offset: atlas.offset.into(),
            index: clamped_index,
            image_size: image_size.into(),
            entity_type: 1,
            flip_x: tile.flip_x as u32,
            flip_y: tile.flip_y as u32,
            color_tint: convert::color_to_linear_rgba_f32(tile.color),
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
            color_tint: convert::color_to_linear_rgba_f32(sprite.color),
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

/// Returns all visible session names sorted deterministically by `(session.priority, name)`.
///
/// Unlike `game.sorted_session_keys`, this is always accurate even before the first `game.step()`
/// (e.g. during asset preload) or when a system creates/modifies sessions mid-frame.
pub fn sorted_visible_sessions(game: &bones::Game) -> Vec<Ustr> {
    let mut sessions: Vec<(i32, Ustr)> = game
        .sessions
        .iter()
        .filter(|(_, s)| s.visible)
        .map(|(name, s)| (s.priority, *name))
        .collect();
    sessions.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    sessions.into_iter().map(|(_, name)| name).collect()
}

/// Physical viewport rectangle and depth range clamped to the render target bounds so
/// `wgpu::RenderPass::set_viewport` never fails validation on window resize.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResolvedViewport {
    pub position: Vec2,
    pub size: Vec2,
    pub depth_min: f32,
    pub depth_max: f32,
    pub is_custom: bool,
}

pub fn resolve_camera_viewport(
    camera: &bones::Camera,
    target_size: UVec2,
) -> Option<ResolvedViewport> {
    if target_size.x == 0 || target_size.y == 0 {
        return None;
    }

    if let Some(vp) = camera.viewport.option() {
        if vp.position.x >= target_size.x || vp.position.y >= target_size.y {
            return None;
        }
        let max_w = target_size.x - vp.position.x;
        let max_h = target_size.y - vp.position.y;
        let w = vp.size.x.min(max_w);
        let h = vp.size.y.min(max_h);
        if w == 0 || h == 0 {
            return None;
        }

        // `bones::Viewport::default()` initializes both depth bounds to 0.0; treat that as the
        // standard [0.0, 1.0] depth range, and clamp/order explicit bounds into WebGPU's [0.0, 1.0].
        let (depth_min, depth_max) = if vp.depth_min == 0.0 && vp.depth_max == 0.0 {
            (0.0, 1.0)
        } else {
            let d0 = vp.depth_min.clamp(0.0, 1.0);
            let d1 = vp.depth_max.clamp(0.0, 1.0);
            (d0.min(d1), d0.max(d1))
        };

        Some(ResolvedViewport {
            position: vp.position.as_vec2(),
            size: UVec2::new(w, h).as_vec2(),
            depth_min,
            depth_max,
            is_custom: true,
        })
    } else {
        Some(ResolvedViewport {
            position: Vec2::ZERO,
            size: target_size.as_vec2(),
            depth_min: 0.0,
            depth_max: 1.0,
            is_custom: false,
        })
    }
}

pub fn camera_view_size(
    camera: &bones::Camera,
    viewport_size: Vec2,
    scale_factor: f32,
) -> Option<(f32, f32)> {
    if viewport_size.x <= 0.0 || viewport_size.y <= 0.0 || !viewport_size.is_finite() {
        return None;
    }
    let safe_scale = if scale_factor.is_finite() && scale_factor > 0.0 {
        scale_factor
    } else {
        1.0
    };
    let aspect = viewport_size.x / viewport_size.y;
    let (w, h) = match camera.size {
        bones::CameraSize::Fixed { width, height } => (width, height),
        bones::CameraSize::FixedHeight(h) => (h * aspect, h),
        bones::CameraSize::FixedWidth(w) => (w, w / aspect),
        bones::CameraSize::Window { pixels_per_unit } => {
            let ppu = if pixels_per_unit.is_finite() && pixels_per_unit > 0.0 {
                pixels_per_unit
            } else {
                1.0
            };
            let logical_size = viewport_size / safe_scale;
            (logical_size.x / ppu, logical_size.y / ppu)
        }
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
    };

    if w <= 0.0 || h <= 0.0 || !w.is_finite() || !h.is_finite() {
        None
    } else {
        Some((w, h))
    }
}

/// Converts a physical screen cursor position `(x, y)` (top-left origin) to 2D world space
/// using the active camera.
pub fn screen_to_world_2d(
    game: &bones::Game,
    screen_pos: Vec2,
    target_size: UVec2,
    scale_factor: f32,
) -> Option<Vec2> {
    if target_size.x == 0 || target_size.y == 0 {
        return None;
    }

    let mut candidates: Vec<(ResolvedViewport, (f32, f32), bones::Transform)> = Vec::new();
    for session_name in sorted_visible_sessions(game) {
        let Some(session) = game.sessions.get(session_name) else {
            continue;
        };
        let entities = session.world.resource::<bones::Entities>();
        let cameras = session.world.component::<bones::Camera>();
        let transforms = session.world.component::<bones::Transform>();

        let mut session_cameras: Vec<(i32, ResolvedViewport, (f32, f32), bones::Transform)> =
            Vec::new();
        for (_ent, (camera, transform)) in entities.iter_with((&cameras, &transforms)) {
            if !camera.active {
                continue;
            }
            let Some(vp) = resolve_camera_viewport(camera, target_size) else {
                continue;
            };
            let Some(view_size) = camera_view_size(camera, vp.size, scale_factor) else {
                continue;
            };
            session_cameras.push((camera.priority, vp, view_size, *transform));
        }
        session_cameras.sort_by(|a, b| a.0.cmp(&b.0));
        candidates.extend(
            session_cameras
                .into_iter()
                .map(|(_, vp, vs, tr)| (vp, vs, tr)),
        );
    }

    if candidates.is_empty() {
        return None;
    }

    // Prefer the highest-priority camera whose viewport contains `screen_pos`,
    // falling back to the highest-priority active camera overall.
    let &(vp, (camera_width_px, camera_height_px), transform) = candidates
        .iter()
        .rev()
        .find(|(vp, _, _)| {
            screen_pos.x >= vp.position.x
                && screen_pos.x <= vp.position.x + vp.size.x
                && screen_pos.y >= vp.position.y
                && screen_pos.y <= vp.position.y + vp.size.y
        })
        .or_else(|| candidates.last())?;

    let rel = screen_pos - vp.position;
    let ndc = Vec2::new(
        (rel.x / vp.size.x) * 2.0 - 1.0,
        1.0 - (rel.y / vp.size.y) * 2.0,
    );
    let local = Vec3::new(
        ndc.x * (camera_width_px * 0.5),
        ndc.y * (camera_height_px * 0.5),
        0.0,
    );
    let world = (transform.rotation * local) + transform.translation;
    Some(world.truncate())
}

/// Synchronizes all active sprite/atlas/tilemap textures across `game.sessions` into `atlas_pool`
/// without storing any renderer components inside `session.world`.
pub fn update_atlas_pool(game: &bones::Game, atlas_pool: &mut AtlasPool, queue: &wgpu::Queue) {
    let Some(assets) = game.get_shared_resource::<bones::AssetServer>() else {
        return;
    };

    atlas_pool.generation = atlas_pool.generation.wrapping_add(1);
    let gen = atlas_pool.generation;

    let mut pending_uploads: Vec<(bones::Handle<bones::Image>, bones::Cid)> = Vec::new();
    let mut pending_seen: HashSet<(bones::Handle<bones::Image>, bones::Cid)> = HashSet::new();

    let mut mark_image = |image_handle: bones::Handle<bones::Image>| {
        if image_handle == bones::Handle::default() {
            return;
        }
        let Some(cid) = assets
            .store
            .asset_ids
            .get(&image_handle.untyped())
            .map(|entry| *entry.value())
        else {
            return;
        };

        if let Some(cached) = atlas_pool.cache.get_mut(&(image_handle, cid)) {
            cached.last_used_generation = gen;
        } else if pending_seen.insert((image_handle, cid)) {
            pending_uploads.push((image_handle, cid));
        }
    };

    // Mark every image referenced by any session so temporarily hidden sessions (e.g. behind a
    // pause overlay) keep their atlas allocations warm while despawned/hot-reloaded assets are freed.
    for (_session_name, session) in game.sessions.iter() {
        let entities = session.world.resource::<bones::Entities>();
        let sprites = session.world.component::<bones::Sprite>();
        let atlases = session.world.component::<bones::AtlasSprite>();
        let tile_layers = session.world.component::<bones::TileLayer>();

        for (_ent, sprite) in entities.iter_with(&sprites) {
            mark_image(sprite.image);
        }
        for (_ent, atlas_sprite) in entities.iter_with(&atlases) {
            if atlas_sprite.atlas == bones::Handle::default() {
                continue;
            }
            if let Some(Ok(atlas)) = assets.try_get(atlas_sprite.atlas) {
                mark_image(atlas.image);
            }
        }
        for (_ent, tile_layer) in entities.iter_with(&tile_layers) {
            if tile_layer.atlas == bones::Handle::default() {
                continue;
            }
            if let Some(Ok(atlas)) = assets.try_get(tile_layer.atlas) {
                mark_image(atlas.image);
            }
        }
    }

    // Sweep unused allocations BEFORE uploading new ones so freed atlas space is immediately reusable.
    let stale_keys: Vec<(bones::Handle<bones::Image>, bones::Cid)> = atlas_pool
        .cache
        .iter()
        .filter_map(|(k, v)| (v.last_used_generation != gen).then_some(*k))
        .collect();
    for key in stale_keys {
        if let Some(removed) = atlas_pool.cache.remove(&key) {
            log::debug!(
                "Reclaiming unused texture allocation from atlas {}: {:?}",
                removed.atlas_id,
                key.0
            );
            atlas_pool.deallocate(removed.atlas_id, removed.alloc);
        }
    }

    for (image_handle, cid) in pending_uploads {
        let Some(Ok(image)) = assets.try_get(image_handle) else {
            continue;
        };
        let bones::Image::Data(img) = &*image else {
            continue;
        };

        let rgba = img.to_rgba8();
        let (w, h) = (rgba.width(), rgba.height());
        let (atlas_id, alloc, (atlas_w, atlas_h), (pad_x, pad_y)) =
            match atlas_pool.allocate_for_image((w, h)) {
                Ok(res) => res,
                Err(err) => {
                    log::error!("{err}");
                    continue;
                }
            };

        let rect = alloc.rectangle;
        let inner_min_x = rect.min.x as u32 + pad_x;
        let inner_min_y = rect.min.y as u32 + pad_y;
        let uv_min = [
            inner_min_x as f32 / atlas_w as f32,
            inner_min_y as f32 / atlas_h as f32,
        ];
        let uv_max = [
            (inner_min_x + w) as f32 / atlas_w as f32,
            (inner_min_y + h) as f32 / atlas_h as f32,
        ];
        let image_size = [w as f32, h as f32];

        let raw = rgba.into_raw();
        let (padded_data, upload_w, upload_h, padded_stride) =
            build_extruded_rgba_upload(&raw, w, h, pad_x, pad_y);

        log::debug!(
            "Uploaded sprite to atlas {} at {:?}..{:?} (pad {}x{})",
            atlas_id,
            rect.min,
            rect.max,
            pad_x,
            pad_y
        );

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &atlas_pool.atlases[atlas_id].texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: rect.min.x as u32,
                    y: rect.min.y as u32,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            &padded_data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_stride),
                rows_per_image: Some(upload_h),
            },
            wgpu::Extent3d {
                width: upload_w,
                height: upload_h,
                depth_or_array_layers: 1,
            },
        );

        atlas_pool.cache.insert(
            (image_handle, cid),
            CachedAtlasAllocation {
                atlas_id,
                alloc,
                uv_min,
                uv_max,
                image_size,
                last_used_generation: gen,
            },
        );
    }
}

pub struct ResolvedCameraDraw {
    pub viewport: ResolvedViewport,
    pub uniform_offset: u32,
    pub load_clear_color: Option<wgpu::Color>,
    pub viewport_bg_draw: Option<line::LineDrawCall>,
}

pub enum SessionDrawCmd {
    SpriteBatch {
        atlas_id: usize,
        instances: Range<u32>,
    },
    Line(line::LineDrawCall),
}

pub struct PreparedSessionRender {
    pub session_name: Ustr,
    pub cameras: Vec<ResolvedCameraDraw>,
    pub draws: Vec<SessionDrawCmd>,
}

enum RenderItemPayload {
    Sprite(AtlasSpriteUniform),
    Line {
        uniform: AtlasSpriteUniform,
        vertices: Vec<line::Vertex>,
        indices: Vec<u32>,
    },
}

struct SortableRenderItem {
    z: f32,
    kind_order: u8,
    atlas_id: usize,
    payload: RenderItemPayload,
}

fn lookup_cached_image<'a>(
    assets: &bones::AssetServer,
    atlas_pool: &'a AtlasPool,
    image_handle: bones::Handle<bones::Image>,
) -> Option<&'a CachedAtlasAllocation> {
    if image_handle == bones::Handle::default() {
        return None;
    }
    let cid = *assets.store.asset_ids.get(&image_handle.untyped())?.value();
    atlas_pool.cache.get(&(image_handle, cid))
}

#[allow(clippy::too_many_arguments)]
pub fn prepare_render_frame(
    game: &bones::Game,
    atlas_pool: &AtlasPool,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    camera_dynamic_uniform: &mut DynamicBuffer,
    dynamic_storage: &mut DynamicBuffer,
    line_buffers: &mut line::LineBuffers,
    target_size: UVec2,
    scale_factor: f32,
    frame_clear_color: bones::Color,
) -> Vec<PreparedSessionRender> {
    // NOTE: This must match `pixel_scale` in `atlas_sprite.wgsl` and `line.wgsl`.
    // It is used to keep world-space values (in pixels) numerically small on the GPU.
    const PIXEL_SCALE: f32 = 0.001;

    let assets = game.get_shared_resource::<bones::AssetServer>();

    let mut camera_transforms: Vec<CameraTransform> = Vec::new();
    let mut instances: Vec<AtlasSpriteUniform> = Vec::new();
    let mut all_line_vertices: Vec<line::Vertex> = Vec::new();
    let mut all_line_indices: Vec<u32> = Vec::new();
    let mut viewport_bg_quad_range: Option<(i32, Range<u32>)> = None;
    let mut prepared_sessions: Vec<PreparedSessionRender> = Vec::new();
    let mut first_fullscreen_camera_seen = false;

    for session_name in sorted_visible_sessions(game) {
        let Some(session) = game.sessions.get(session_name) else {
            continue;
        };

        let entities = session.world.resource::<bones::Entities>();
        let transforms = session.world.component::<bones::Transform>();
        let cameras = session.world.component::<bones::Camera>();
        let clear_color = session.world.get_resource::<bones::ClearColor>();

        // 1) Collect and sort active cameras in this session
        let mut raw_cameras: Vec<(
            i32,
            &bones::Camera,
            &bones::Transform,
            ResolvedViewport,
            (f32, f32),
        )> = Vec::new();
        for (_ent, (camera, transform)) in entities.iter_with((&cameras, &transforms)) {
            if !camera.active {
                continue;
            }
            let Some(vp) = resolve_camera_viewport(camera, target_size) else {
                continue;
            };
            let Some(view_size) = camera_view_size(camera, vp.size, scale_factor) else {
                continue;
            };
            raw_cameras.push((camera.priority, camera, transform, vp, view_size));
        }

        if raw_cameras.is_empty() {
            continue;
        }
        raw_cameras.sort_by(|a, b| a.0.cmp(&b.0));

        let mut resolved_cameras = Vec::with_capacity(raw_cameras.len());
        for (_, camera, transform, vp, (camera_width_px, camera_height_px)) in raw_cameras {
            let camera_width = camera_width_px * PIXEL_SCALE;
            let camera_height = camera_height_px * PIXEL_SCALE;

            let mut cam_translation = transform.translation;
            cam_translation.x *= PIXEL_SCALE;
            cam_translation.y *= PIXEL_SCALE;

            let cam_transform = bones::Transform {
                translation: cam_translation,
                rotation: transform.rotation,
                scale: Vec3::ONE,
            };

            let view = cam_transform.to_matrix_none().inverse();
            let proj = Mat4::orthographic_rh(
                -camera_width * 0.5,
                camera_width * 0.5,
                -camera_height * 0.5,
                camera_height * 0.5,
                -10_000.0,
                10_000.0,
            );
            let camera_to_clip = proj * view;

            let uniform_offset =
                (camera_transforms.len() as u32) * (std::mem::size_of::<CameraTransform>() as u32);
            camera_transforms.push(CameraTransform {
                transform: camera_to_clip.to_cols_array_2d(),
                screen_size: vp.size.into(),
                ..Default::default()
            });

            let mut load_clear_color = None;
            let mut viewport_bg_draw = None;

            if vp.is_custom {
                if camera.draw_background_color {
                    let color = match (camera.background_color.option(), clear_color.as_deref()) {
                        (Some(color), _) => color,
                        (None, Some(color)) => color.0,
                        (None, None) => frame_clear_color,
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
                        color_tint: convert::color_to_linear_rgba_f32(color),
                        ..Default::default()
                    });
                    viewport_bg_draw = Some(line::LineDrawCall {
                        index_range,
                        base_vertex,
                        instance_index,
                    });
                }
            } else {
                let is_first_fullscreen = !first_fullscreen_camera_seen;
                first_fullscreen_camera_seen = true;

                if camera.draw_background_color {
                    if let Some(explicit_color) = camera.background_color.option() {
                        load_clear_color = Some(convert::color_to_wgpu(explicit_color));
                    } else if is_first_fullscreen {
                        let fallback = clear_color
                            .as_deref()
                            .map(|c| c.0)
                            .unwrap_or(frame_clear_color);
                        load_clear_color = Some(convert::color_to_wgpu(fallback));
                    }
                }
            }

            resolved_cameras.push(ResolvedCameraDraw {
                viewport: vp,
                uniform_offset,
                load_clear_color,
                viewport_bg_draw,
            });
        }

        // 2) Collect all renderable items in this session without mutating `session.world`
        let sprites = session.world.component::<bones::Sprite>();
        let atlases = session.world.component::<bones::AtlasSprite>();
        let tile_layers = session.world.component::<bones::TileLayer>();
        let tiles = session.world.component::<bones::Tile>();
        let paths = session.world.component::<bones::Path2d>();

        let mut render_items: Vec<SortableRenderItem> = Vec::new();

        if let Some(ref assets) = assets {
            for (_ent, (sprite, transform)) in entities.iter_with((&sprites, &transforms)) {
                let Some(cached) = lookup_cached_image(assets, atlas_pool, sprite.image) else {
                    continue;
                };
                let image_size = Vec2::new(cached.image_size[0], cached.image_size[1]);
                render_items.push(SortableRenderItem {
                    z: transform.translation.z,
                    kind_order: 0,
                    atlas_id: cached.atlas_id,
                    payload: RenderItemPayload::Sprite(AtlasSpriteUniform::from_sprite(
                        sprite,
                        transform,
                        (cached.uv_min, cached.uv_max),
                        image_size,
                    )),
                });
            }

            for (_ent, (atlas_sprite, transform)) in entities.iter_with((&atlases, &transforms)) {
                if atlas_sprite.atlas == bones::Handle::default() {
                    continue;
                }
                let Some(Ok(atlas)) = assets.try_get(atlas_sprite.atlas) else {
                    continue;
                };
                let Some(cached) = lookup_cached_image(assets, atlas_pool, atlas.image) else {
                    continue;
                };
                let image_size = Vec2::new(cached.image_size[0], cached.image_size[1]);
                render_items.push(SortableRenderItem {
                    z: transform.translation.z,
                    kind_order: 0,
                    atlas_id: cached.atlas_id,
                    payload: RenderItemPayload::Sprite(AtlasSpriteUniform::from_atlas_sprite(
                        atlas_sprite,
                        &atlas,
                        transform,
                        (cached.uv_min, cached.uv_max),
                        image_size,
                    )),
                });
            }

            for (_ent, (tile_layer, transform)) in entities.iter_with((&tile_layers, &transforms)) {
                if tile_layer.grid_size.x == 0
                    || tile_layer.grid_size.y == 0
                    || tile_layer.atlas == bones::Handle::default()
                {
                    continue;
                }
                let Some(Ok(atlas)) = assets.try_get(tile_layer.atlas) else {
                    continue;
                };
                let Some(cached) = lookup_cached_image(assets, atlas_pool, atlas.image) else {
                    continue;
                };
                let image_size = Vec2::new(cached.image_size[0], cached.image_size[1]);
                let max_slots = (tile_layer.grid_size.x as usize)
                    .saturating_mul(tile_layer.grid_size.y as usize);

                for (slot_idx, tile_ent) in tile_layer.tiles.iter().take(max_slots).enumerate() {
                    let Some(tile_ent) = tile_ent else {
                        continue;
                    };
                    let Some(tile) = tiles.get(*tile_ent) else {
                        continue;
                    };
                    render_items.push(SortableRenderItem {
                        z: transform.translation.z,
                        kind_order: 0,
                        atlas_id: cached.atlas_id,
                        payload: RenderItemPayload::Sprite(AtlasSpriteUniform::from_tile(
                            tile,
                            &atlas,
                            transform,
                            (cached.uv_min, cached.uv_max),
                            image_size,
                            slot_idx,
                            tile_layer,
                        )),
                    });
                }
            }
        }

        for (_ent, (path, transform)) in entities.iter_with((&paths, &transforms)) {
            let geometry = line::tessellate_path(path);
            if geometry.indices.is_empty() || geometry.vertices.is_empty() {
                continue;
            }
            let mut line_transform = *transform;
            // Offset the path towards the camera slightly to make sure it renders on top of a
            // sprite/etc. if it is applied to an entity with both a sprite and a path.
            line_transform.translation.z += 0.0001;

            render_items.push(SortableRenderItem {
                z: line_transform.translation.z,
                kind_order: 1,
                atlas_id: 0,
                payload: RenderItemPayload::Line {
                    uniform: AtlasSpriteUniform {
                        entity_type: 2,
                        transform: line_transform.to_matrix_none().to_cols_array_2d(),
                        color_tint: convert::color_to_linear_rgba_f32(path.color),
                        ..Default::default()
                    },
                    vertices: geometry.vertices,
                    indices: geometry.indices,
                },
            });
        }

        // Stable sort back-to-front (ascending Z), grouping same-Z sprites by `atlas_id` for
        // instanced batching while interleaving `Path2d` lines at their true Z depth.
        render_items.sort_by(|a, b| {
            a.z.total_cmp(&b.z)
                .then_with(|| a.kind_order.cmp(&b.kind_order))
                .then_with(|| a.atlas_id.cmp(&b.atlas_id))
        });

        let mut draws: Vec<SessionDrawCmd> = Vec::new();
        for item in render_items {
            match item.payload {
                RenderItemPayload::Sprite(uniform) => {
                    let instance_idx = instances.len() as u32;
                    instances.push(uniform);
                    if let Some(SessionDrawCmd::SpriteBatch {
                        atlas_id,
                        instances: range,
                    }) = draws.last_mut()
                    {
                        if *atlas_id == item.atlas_id && range.end == instance_idx {
                            range.end = instance_idx + 1;
                            continue;
                        }
                    }
                    draws.push(SessionDrawCmd::SpriteBatch {
                        atlas_id: item.atlas_id,
                        instances: instance_idx..instance_idx + 1,
                    });
                }
                RenderItemPayload::Line {
                    uniform,
                    vertices,
                    indices,
                } => {
                    let instance_index = instances.len() as u32;
                    instances.push(uniform);

                    let base_vertex = all_line_vertices.len() as i32;
                    let index_start = all_line_indices.len() as u32;
                    all_line_vertices.extend_from_slice(&vertices);
                    all_line_indices.extend_from_slice(&indices);
                    let index_end = all_line_indices.len() as u32;

                    draws.push(SessionDrawCmd::Line(line::LineDrawCall {
                        index_range: index_start..index_end,
                        base_vertex,
                        instance_index,
                    }));
                }
            }
        }

        prepared_sessions.push(PreparedSessionRender {
            session_name,
            cameras: resolved_cameras,
            draws,
        });
    }

    camera_dynamic_uniform.write_pods(device, queue, &camera_transforms);
    dynamic_storage.write_pods(device, queue, &instances);
    line_buffers.write_buffers(device, queue, &all_line_vertices, &all_line_indices);

    prepared_sessions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_camera_viewport_clamps_to_render_target_and_depth_bounds() {
        let mut cam = bones::Camera {
            viewport: bones::Set(bones::Viewport {
                position: UVec2::new(600, 400),
                size: UVec2::new(500, 500),
                ..Default::default()
            }),
            ..Default::default()
        };

        // Partially overflowing viewport clamps width/height to remaining window area and defaults
        // (0.0, 0.0) depth to (0.0, 1.0).
        let vp = resolve_camera_viewport(&cam, UVec2::new(800, 600)).unwrap();
        assert_eq!(vp.position, Vec2::new(600.0, 400.0));
        assert_eq!(vp.size, Vec2::new(200.0, 200.0));
        assert_eq!((vp.depth_min, vp.depth_max), (0.0, 1.0));
        assert!(vp.is_custom);

        // Completely off-screen viewport returns None instead of triggering wgpu validation error.
        assert!(resolve_camera_viewport(&cam, UVec2::new(500, 300)).is_none());

        // Reversed / out-of-range explicit depth bounds are clamped and ordered.
        cam.viewport = bones::Set(bones::Viewport {
            position: UVec2::new(0, 0),
            size: UVec2::new(100, 100),
            depth_min: 1.5,
            depth_max: -0.2,
        });
        let vp2 = resolve_camera_viewport(&cam, UVec2::new(800, 600)).unwrap();
        assert_eq!((vp2.depth_min, vp2.depth_max), (0.0, 1.0));
    }

    #[test]
    fn camera_view_size_handles_hidpi_window_scaling_and_invalid_inputs() {
        let win_cam = bones::Camera {
            size: bones::CameraSize::Window {
                pixels_per_unit: 2.0,
            },
            ..Default::default()
        };
        // 2560x1440 physical at 2.0 scale_factor => 1280x720 logical => 640x360 world units.
        assert_eq!(
            camera_view_size(&win_cam, Vec2::new(2560.0, 1440.0), 2.0),
            Some((640.0, 360.0))
        );

        let zero_cam = bones::Camera {
            size: bones::CameraSize::FixedHeight(0.0),
            ..Default::default()
        };
        assert_eq!(
            camera_view_size(&zero_cam, Vec2::new(800.0, 600.0), 1.0),
            None
        );
    }

    #[test]
    fn tile_uniform_with_zero_grid_width_does_not_divide_by_zero() {
        let tile = bones::Tile::default();
        let atlas = bones::Atlas {
            tile_size: Vec2::new(16.0, 16.0),
            columns: 4,
            rows: 4,
            ..Default::default()
        };
        let layer = bones::TileLayer {
            map: bones::EntityGridMap {
                tiles: Vec::new(),
                grid_size: UVec2::ZERO,
            },
            atlas: bones::Handle::default(),
        };
        let uniform = AtlasSpriteUniform::from_tile(
            &tile,
            &atlas,
            &bones::Transform::default(),
            ([0.0, 0.0], [1.0, 1.0]),
            Vec2::new(64.0, 64.0),
            0,
            &layer,
        );
        assert_eq!(uniform.entity_type, 1);
    }
}
