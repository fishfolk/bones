use super::*;

use bevy::render::{camera::ScalingMode, Extract};
use bevy::sprite::{Anchor, ExtractedSprite, ExtractedSprites};
use bevy::utils::HashMap;
use bevy::window::WindowMode;
use bevy_prototype_lyon::prelude as lyon;
use bones::{BitSet, ComponentIterBitset};
use glam::*;

/// Marker component for entities that are rendered in Bevy for bones.
#[derive(Component)]
pub struct BevyBonesEntity;

/// Resource containing the entity spawned for all of the bones game renderables.
#[derive(Resource)]
pub struct BonesGameEntity(pub Entity);
impl FromWorld for BonesGameEntity {
    fn from_world(world: &mut World) -> Self {
        Self(world.spawn(VisibilityBundle::default()).id())
    }
}

/// Resource mapping bones image IDs to their bevy handles.
#[derive(Resource, Debug, Deref, DerefMut)]
pub struct BonesImageIds {
    #[deref]
    map: HashMap<u32, Handle<Image>>,
    next_id: u32,
}
impl Default for BonesImageIds {
    fn default() -> Self {
        Self {
            map: Default::default(),
            next_id: 1,
        }
    }
}

impl BonesImageIds {
    /// Load all bones images into bevy.
    pub fn load_bones_images(
        &mut self,
        bones_assets: &bones::AssetServer,
        bones_egui_textures: &mut bones::EguiTextures,
        bevy_images: &mut Assets<Image>,
        bevy_egui_textures: &mut bevy_egui::EguiUserTextures,
    ) {
        for entry in bones_assets.store.asset_ids.iter() {
            let handle: &bones::UntypedHandle = entry.key();
            let cid = entry.value();
            let mut asset = bones_assets.store.assets.get_mut(cid).unwrap();
            if let Ok(image) = asset.data.try_cast_mut::<bones::Image>() {
                self.load_bones_image(
                    handle.typed(),
                    image,
                    bones_egui_textures,
                    bevy_images,
                    bevy_egui_textures,
                )
            }
        }
    }

    /// Load a bones image into bevy.
    pub fn load_bones_image(
        &mut self,
        bones_handle: bones::Handle<bones::Image>,
        image: &mut bones::Image,
        bones_egui_textures: &mut bones::EguiTextures,
        bevy_images: &mut Assets<Image>,
        bevy_egui_textures: &mut bevy_egui::EguiUserTextures,
    ) {
        let Self { map, next_id } = self;
        let mut taken_image = bones::Image::External(0); // Dummy value temporarily
        std::mem::swap(image, &mut taken_image);
        if let bones::Image::Data(data) = taken_image {
            let handle = bevy_images.add(Image::from_dynamic(data, true));
            let egui_texture = bevy_egui_textures.add_image(handle.clone());
            // Broke when updating bones egui to 0.30
            //bones_egui_textures.insert(bones_handle, egui_texture);
            map.insert(*next_id, handle);
            *image = bones::Image::External(*next_id);
            *next_id += 1;

        // The image has already been loaded. This may happen if multiple asset handles use the same
        // image data. We will end up visiting the same data twice.
        } else {
            // Swap the image back to it's previous value.
            std::mem::swap(image, &mut taken_image);
        }
    }
}

pub fn sync_clear_color(mut clear_color: ResMut<ClearColor>, game: Res<BonesGame>) {
    for name in &game.sorted_session_keys {
        let session = game.sessions.get(*name).unwrap();
        if !session.visible {
            continue;
        }
        if let Some(bones_clear_color) = session.world.get_resource::<bones::ClearColor>() {
            clear_color.0 = bones_clear_color.0.into_bevy();
        }
    }
}

/// Syncs elements of the bones window
pub fn sync_bones_window(mut game: ResMut<BonesGame>, mut window_query: Query<&mut Window>) {
    let mut window = window_query.get_single_mut().unwrap();
    let bones_window = match game.shared_resource_cell::<bones::Window>() {
        Some(w) => w,
        None => {
            game.insert_shared_resource(bones::Window {
                size: vec2(window.width(), window.height()),
                fullscreen: matches!(&window.mode, WindowMode::BorderlessFullscreen),
            });
            game.shared_resource_cell().unwrap()
        }
    };
    let bones_window = bones_window.borrow().unwrap();

    let is_fullscreen = matches!(&window.mode, WindowMode::BorderlessFullscreen);
    if is_fullscreen != bones_window.fullscreen {
        window.mode = if bones_window.fullscreen {
            WindowMode::BorderlessFullscreen
        } else {
            WindowMode::Windowed
        };
    }
}

/// Sync bones cameras with Bevy
pub fn sync_cameras(
    game: Res<BonesGame>,
    mut commands: Commands,
    mut bevy_bones_cameras: Query<Entity, (With<BevyBonesEntity>, With<Camera>)>,
) {
    // Collect the bevy cameras that we've created for the bones game
    let mut bevy_bones_cameras = bevy_bones_cameras.iter_mut();

    // Create a helper callback to add/update a bones camera into the bevy world
    let mut add_bones_camera = |bones_camera: &bones::Camera,
                                bones_transform: &bones::Transform| {
        // Get or spawn an entity for the camera
        let mut camera_ent = match bevy_bones_cameras.next() {
            Some(ent) => commands.entity(ent),
            None => commands.spawn((Camera2dBundle::default(), BevyBonesEntity)),
        };

        // Insert our updated components on the camera
        camera_ent.insert((
            Camera {
                is_active: bones_camera.active,
                viewport: bones_camera.viewport.option().map(|x| x.into_bevy()),
                order: bones_camera.priority as isize,
                ..default()
            },
            OrthographicProjection {
                scaling_mode: match bones_camera.size {
                    bones::CameraSize::FixedHeight(h) => ScalingMode::FixedVertical(h),
                    bones::CameraSize::FixedWidth(w) => ScalingMode::FixedHorizontal(w),
                },
                ..default()
            },
            bones_transform.into_bevy(),
        ));
    };

    // Loop through all sessions
    for session_name in &game.sorted_session_keys {
        let session = game.sessions.get(*session_name).unwrap();
        if !session.visible {
            continue;
        }

        let world = &session.world;

        // Skip worlds without cameras and transforms
        if !(world
            .components
            .get::<bones::Transform>()
            .borrow()
            .bitset()
            .bit_any()
            && world
                .components
                .get::<bones::Camera>()
                .borrow()
                .bitset()
                .bit_any())
        {
            continue;
        }

        let entities = world.resource::<bones::Entities>();
        let transforms = world.components.get::<bones::Transform>().borrow();
        let cameras = world.components.get::<bones::Camera>().borrow();

        // Sync cameras
        for (_ent, (transform, camera)) in entities.iter_with((&transforms, &cameras)) {
            // Add each camera to the bevy world
            add_bones_camera(camera, transform)
        }
    }

    // Delete any remaining bevy cameras that don't have bones equivalents.
    for remaining_ent in bevy_bones_cameras {
        commands.entity(remaining_ent).despawn()
    }
}

pub fn extract_bones_sprites(
    mut extracted_sprites: ResMut<ExtractedSprites>,
    game: Extract<Res<BonesGame>>,
    bones_image_ids: Extract<Res<BonesImageIds>>,
    bones_renderable_entity: Extract<Res<BonesGameEntity>>,
) {
    let Some(bones_assets) = &game.asset_server() else {
        return;
    };

    for session_name in &game.sorted_session_keys {
        let session = game.sessions.get(*session_name).unwrap();
        if !session.visible {
            continue;
        }

        let world = &session.world;

        // Skip worlds without cameras and transforms
        if !(world
            .components
            .get::<bones::Transform>()
            .borrow()
            .bitset()
            .bit_any()
            && world
                .components
                .get::<bones::Camera>()
                .borrow()
                .bitset()
                .bit_any()
            && (world
                .components
                .get::<bones::Sprite>()
                .borrow()
                .bitset()
                .bit_any()
                || world
                    .components
                    .get::<bones::AtlasSprite>()
                    .borrow()
                    .bitset()
                    .bit_any()))
        {
            continue;
        }

        let entities = world.resource::<bones::Entities>();
        let transforms = world.components.get::<bones::Transform>().borrow();
        let sprites = world.components.get::<bones::Sprite>().borrow();
        let atlas_sprites = world.components.get::<bones::AtlasSprite>().borrow();

        // Extract normal sprites
        let mut z_offset = 0.0;
        for (_, (sprite, transform)) in entities.iter_with((&sprites, &transforms)) {
            let sprite_image = match bones_assets.try_get(sprite.image) {
                Some(Ok(image)) => image,
                Some(Err(err)) => {
                    warn!("Sprite {:?} has invalid handle: {err:?}", sprite.image);
                    continue;
                }
                None => {
                    warn!("Sprite not loaded: {:?}", sprite.image);
                    continue;
                }
            };

            let image_id = if let bones::Image::External(id) = &*sprite_image {
                *id
            } else {
                panic!(
                    "Images added at runtime not supported yet, \
                please open an issue."
                );
            };
            extracted_sprites.sprites.push(ExtractedSprite {
                entity: bones_renderable_entity.0,
                transform: {
                    let mut t: Transform = transform.into_bevy();
                    // Add tiny z offset to enforce a consistent z-sort
                    t.translation.z += z_offset;
                    z_offset += 0.00001;
                    t.into()
                },
                color: sprite.color.into_bevy(),
                rect: None,
                custom_size: None,
                image_handle_id: bones_image_ids.get(&image_id).unwrap().id(),
                flip_x: sprite.flip_x,
                flip_y: sprite.flip_y,
                anchor: Anchor::Center.as_vec(),
            });
        }

        // Extract atlas sprites
        for (_, (atlas_sprite, transform)) in entities.iter_with((&atlas_sprites, &transforms)) {
            let atlas = bones_assets.get(atlas_sprite.atlas);
            let atlas_image = bones_assets.get(atlas.image);
            let image_id = if let bones::Image::External(id) = &*atlas_image {
                *id
            } else {
                panic!(
                    "Images added at runtime not supported yet, \
                        please open an issue."
                );
            };
            let index = atlas_sprite.index;
            let y = index / atlas.columns;
            let x = index - (y * atlas.columns);
            let cell = Vec2::new(x as f32, y as f32);
            let current_padding = atlas.padding
                * Vec2::new(if x > 0 { 1.0 } else { 0.0 }, if y > 0 { 1.0 } else { 0.0 });
            let min = (atlas.tile_size + current_padding) * cell + atlas.offset;
            let rect = Rect {
                min,
                max: min + atlas.tile_size,
            };
            extracted_sprites.sprites.push(ExtractedSprite {
                entity: bones_renderable_entity.0,
                transform: transform.into_bevy().into(),
                color: atlas_sprite.color.into_bevy(),
                rect: Some(rect),
                custom_size: None,
                image_handle_id: bones_image_ids.get(&image_id).unwrap().id(),
                flip_x: atlas_sprite.flip_x,
                flip_y: atlas_sprite.flip_y,
                anchor: Anchor::Center.as_vec(),
            });
        }
    }
}

pub fn extract_bones_tilemaps(
    mut extracted_sprites: ResMut<ExtractedSprites>,
    game: Extract<Res<BonesGame>>,
    bones_image_ids: Extract<Res<BonesImageIds>>,
    bones_renderable_entity: Extract<Res<BonesGameEntity>>,
) {
    let Some(bones_assets) = &game.asset_server() else {
        return;
    };

    for session_name in &game.sorted_session_keys {
        let session = game.sessions.get(*session_name).unwrap();
        if !session.visible {
            continue;
        }

        let world = &session.world;

        // Skip worlds without cameras renderable tile layers
        if !(world
            .components
            .get::<bones::Transform>()
            .borrow()
            .bitset()
            .bit_any()
            && world
                .components
                .get::<bones::Camera>()
                .borrow()
                .bitset()
                .bit_any()
            && world
                .components
                .get::<bones::TileLayer>()
                .borrow()
                .bitset()
                .bit_any())
        {
            continue;
        }

        let entities = world.resource::<bones::Entities>();
        let transforms = world.components.get::<bones::Transform>().borrow();
        let tile_layers = world.components.get::<bones::TileLayer>().borrow();
        let tiles = world.components.get::<bones::Tile>().borrow();

        // Extract tiles as sprites
        for (_, (tile_layer, transform)) in entities.iter_with((&tile_layers, &transforms)) {
            let atlas = bones_assets.get(tile_layer.atlas);
            let atlas_image = bones_assets.get(atlas.image);
            let image_id = if let bones::Image::External(id) = &*atlas_image {
                *id
            } else {
                panic!(
                    "Images added at runtime not supported yet, \
                        please open an issue."
                );
            };

            for (tile_pos_idx, tile_ent) in tile_layer.tiles.iter().enumerate() {
                let Some(tile_ent) = tile_ent else { continue };
                let tile = tiles.get(*tile_ent).unwrap();

                let tile_pos = tile_layer.pos(tile_pos_idx as u32);
                let tile_offset = tile_pos.as_vec2() * tile_layer.tile_size;

                let sprite_idx = tile.idx;
                let y = sprite_idx / atlas.columns;
                let x = sprite_idx - (y * atlas.columns);
                let cell = Vec2::new(x as f32, y as f32);
                let current_padding = atlas.padding
                    * Vec2::new(if x > 0 { 1.0 } else { 0.0 }, if y > 0 { 1.0 } else { 0.0 });
                let min = (atlas.tile_size + current_padding) * cell + atlas.offset;
                let rect = Rect {
                    min,
                    max: min + atlas.tile_size,
                };
                let mut transform = transform.into_bevy();
                transform.translation += tile_offset.extend(0.0);
                // Scale up slightly to avoid bleeding between tiles.
                // TODO: Improve tile rendering
                // Currently we do a small hack here, scaling up the tiles a little bit, to prevent
                // visible gaps between tiles. This solution isn't perfect and we probably need to
                // create a proper tile renderer. That can render multiple tiles on one quad instead
                // of using a separate quad for each tile.
                transform.scale += Vec3::new(0.01, 0.01, 0.0);
                extracted_sprites.sprites.push(ExtractedSprite {
                    entity: bones_renderable_entity.0,
                    transform: transform.into(),
                    color: Color::WHITE,
                    rect: Some(rect),
                    custom_size: None,
                    image_handle_id: bones_image_ids.get(&image_id).unwrap().id(),
                    flip_x: tile.flip_x,
                    flip_y: tile.flip_y,
                    anchor: Anchor::BottomLeft.as_vec(),
                });
            }
        }
    }
}

pub fn sync_bones_path2ds(
    game: Res<BonesGame>,
    mut commands: Commands,
    mut bevy_bones_path2ds: Query<
        (Entity, &mut lyon::Path, &mut lyon::Stroke, &mut Transform),
        With<BevyBonesEntity>,
    >,
) {
    // Collect the bevy path2ds that we've created for the bones game
    let mut bevy_bones_path2ds = bevy_bones_path2ds.iter_mut();

    // Create a helper callback to add/update a bones path2d into the bevy world
    let mut add_bones_path2d = |bones_path2d: &bones::Path2d,
                                bones_transform: &bones::Transform| {
        // Get or create components for the entity
        let mut new_components = None;
        let mut existing_components;
        let (path, stroke, transform) = match bevy_bones_path2ds.next() {
            Some((_ent, path, stroke, transform)) => {
                existing_components = (path, stroke, transform);
                let (path, stroke, transform) = &mut existing_components;
                (&mut **path, &mut **stroke, &mut **transform)
            }
            None => {
                let bundle = lyon::ShapeBundle::default();
                new_components = Some((
                    bundle.path,
                    lyon::Stroke::new(Color::default(), 1.0),
                    Transform::default(),
                ));
                let (path, stroke, transform) = new_components.as_mut().unwrap();
                (path, stroke, transform)
            }
        };

        // Update the components
        *stroke = lyon::Stroke::new(bones_path2d.color.into_bevy(), bones_path2d.thickness);
        *path = bones_path2d
            .points
            .iter()
            .copied()
            .enumerate()
            .fold(lyon::PathBuilder::new(), |mut builder, (i, point)| {
                if i > 0 && !bones_path2d.line_breaks.contains(&i) {
                    builder.line_to(point);
                }
                builder.move_to(point);

                builder
            })
            .build();
        *transform = bones_transform.into_bevy();
        // Offset the path towards the camera slightly to make sure it renders on top of a
        // sprite/etc. if it is applied to an entity with both a sprite and a path.
        transform.translation.z += 0.0001;

        // Spawn the shape if it doesn't already exist
        if let Some((path, stroke, transform)) = new_components {
            commands
                .spawn(lyon::ShapeBundle { path, ..default() })
                .insert(transform)
                .insert(stroke)
                .insert(BevyBonesEntity);
        }
    };

    for session_name in &game.sorted_session_keys {
        let session = game.sessions.get(*session_name).unwrap();
        if !session.visible {
            continue;
        }

        let world = &session.world;

        // Skip worlds without cameras renderable tile layers
        if !(world
            .components
            .get::<bones::Transform>()
            .borrow()
            .bitset()
            .bit_any()
            && world
                .components
                .get::<bones::Camera>()
                .borrow()
                .bitset()
                .bit_any()
            && world
                .components
                .get::<bones::Path2d>()
                .borrow()
                .bitset()
                .bit_any())
        {
            continue;
        }

        let entities = world.resource::<bones::Entities>();
        let transforms = world.components.get::<bones::Transform>().borrow();
        let path2ds = world.components.get::<bones::Path2d>().borrow();

        // Extract tiles as sprites
        for (_, (path2d, transform)) in entities.iter_with((&path2ds, &transforms)) {
            add_bones_path2d(path2d, transform);
        }
    }

    // Despawn extra path 2ds
    for (ent, ..) in bevy_bones_path2ds {
        commands.entity(ent).despawn()
    }
}
