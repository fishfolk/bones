//! Bevy plugin for rendering Bones framework games.

#![warn(missing_docs)]
// This cfg_attr is needed because `rustdoc::all` includes lints not supported on stable
#![cfg_attr(doc, allow(unknown_lints))]
#![deny(rustdoc::all)]

use std::path::PathBuf;

pub use bevy;

use bevy::{
    input::{
        keyboard::KeyboardInput,
        mouse::{MouseButtonInput, MouseMotion, MouseWheel},
    },
    prelude::*,
    render::{
        camera::ScalingMode,
        view::{check_visibility, VisibilitySystems},
        Extract, RenderApp,
    },
    sprite::{extract_sprites, ExtractedSprite, ExtractedSprites, SpriteSystem},
    utils::HashMap,
};
use bevy_egui::EguiContext;
use bevy_prototype_lyon::prelude as lyon;

use bones_framework::prelude::{self as bones, BitSet};
use prelude::convert::{IntoBevy, IntoBones};

/// The prelude
pub mod prelude {
    pub use crate::*;
}

mod convert;

/// Marker component for entities that are rendered in Bevy for bones.
#[derive(Component)]
pub struct BevyBonesEntity;

/// Renderer for [`bones_framework`] [`Game`][bones::Game]s using Bevy.
#[derive(Resource)]
pub struct BonesBevyRenderer {
    /// Whether or not to use nearest-neighbor sampling for textures.
    pub pixel_art: bool,
    /// The bones game to run.
    pub game: bones::Game,
    /// The version of the game, used for the asset loader.
    pub game_version: bones::Version,
    /// The path to load assets from.
    pub asset_dir: PathBuf,
    /// The path to load asset packs from.
    pub packs_dir: PathBuf,
}

/// Resource containing the entity spawned for all of the bones game renderables.
#[derive(Resource)]
pub struct BonesGameEntity(pub Entity);
impl FromWorld for BonesGameEntity {
    fn from_world(world: &mut World) -> Self {
        Self(world.spawn(VisibilityBundle::default()).id())
    }
}

/// Resource mapping bones image IDs to their bevy handles.
#[derive(Resource, Default, Debug, Deref, DerefMut)]
pub struct BonesImageIds(HashMap<u32, Handle<Image>>);

/// Bevy resource that contains the info for the bones game that is being rendered.
#[derive(Resource)]
pub struct BonesData {
    /// The bones game.
    pub game: bones::Game,
    /// The bones asset server cell.
    pub asset_server: bones::AtomicResource<bones::AssetServer>,
}

impl BonesBevyRenderer {
    // TODO: Create a better builder pattern struct for `BonesBevyRenderer`.
    /// Create a new [`BevyBonesRenderer`] for the provided game.
    pub fn new(game: bones::Game) -> Self {
        BonesBevyRenderer {
            pixel_art: true,
            game,
            game_version: bones::Version::new(0, 1, 0),
            asset_dir: PathBuf::from("assets"),
            packs_dir: PathBuf::from("packs"),
        }
    }

    /// Return a bevy [`App`] configured to run the bones game.
    pub fn app(self) -> App {
        let mut app = App::new();

        // Initialize Bevy plugins we use
        let mut plugins = DefaultPlugins.build();
        if self.pixel_art {
            plugins = plugins.set(ImagePlugin::default_nearest());
        }

        app.add_plugins(plugins)
            .add_plugins((
                bevy_simple_tilemap::plugin::SimpleTileMapPlugin,
                bevy_egui::EguiPlugin,
                lyon::ShapePlugin,
            ))
            .insert_resource({
                let mut egui_settings = bevy_egui::EguiSettings::default();
                if self.pixel_art {
                    egui_settings.use_nearest_descriptor();
                }
                egui_settings
            })
            .init_resource::<BonesImageIds>();

        {
            // Configure the AssetIO
            let io = bones::FileAssetIo {
                core_dir: self.asset_dir.clone(),
                packs_dir: self.packs_dir.clone(),
            };
            let mut asset_server = self.game.asset_server();
            asset_server.set_io(io);

            // Load the game assets
            asset_server
                .load_assets()
                .expect("Could not load game assets");

            // Take all loaded image assets and conver them to external images that reference bevy handles
            let mut bones_image_ids = BonesImageIds::default();
            let mut bevy_images = app.world.resource_mut::<Assets<Image>>();
            let mut next_id = 0;
            for asset in asset_server.store.assets.values_mut() {
                if let Ok(image) = asset.data.try_cast_mut::<bones::Image>() {
                    let mut taken_image = bones::Image::External(0); // Dummy value temporarily
                    std::mem::swap(image, &mut taken_image);
                    if let bones::Image::Data(data) = taken_image {
                        let handle = bevy_images.add(Image::from_dynamic(data, true));
                        bones_image_ids.insert(next_id, handle);
                        *image = bones::Image::External(next_id);
                        next_id += 1;
                    }
                }
            }
            app.insert_resource(bones_image_ids);
        }

        // Insert the bones data
        app.insert_resource(BonesData {
            asset_server: self.game.asset_server.clone_cell(),
            game: self.game,
        })
        .init_resource::<BonesGameEntity>();

        // Add the world sync systems
        app.add_systems(
            Update,
            (
                // Collect input and run world simulation
                get_bones_input.pipe(step_bones_game),
                // Synchronize bones render components with the Bevy world.
                (sync_egui_settings, sync_clear_color, sync_cameras),
            )
                .chain(),
        )
        .add_systems(
            PostUpdate,
            set_renderable_visibility
                .in_set(VisibilitySystems::CheckVisibility)
                .after(check_visibility),
        );

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.add_systems(
                ExtractSchedule,
                extract_bones_sprites
                    .in_set(SpriteSystem::ExtractSprites)
                    .after(extract_sprites),
            );
        }

        app
    }
}

fn set_renderable_visibility(
    renderable: Res<BonesGameEntity>,
    mut computed_visibilities: Query<&mut ComputedVisibility>,
) {
    let mut vis = computed_visibilities.get_mut(renderable.0).unwrap();
    vis.set_visible_in_view();
}

fn get_bones_input(
    mut mouse_button_input_events: EventReader<MouseButtonInput>,
    mut mouse_motion_events: EventReader<MouseMotion>,
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mut keyboard_events: EventReader<KeyboardInput>,
) -> (bones::MouseInputs, bones::KeyboardInputs) {
    // TODO: investigate possible ways to avoid allocating vectors every frame for event lists.
    (
        bones::MouseInputs {
            movement: mouse_motion_events
                .iter()
                .last()
                .map(|x| x.delta)
                .unwrap_or_default(),
            wheel_events: mouse_wheel_events
                .iter()
                .map(|event| bones::MouseScrollInput {
                    unit: event.unit.into_bones(),
                    movement: Vec2::new(event.x, event.y),
                })
                .collect(),
            button_events: mouse_button_input_events
                .iter()
                .map(|event| bones::MouseButtonInput {
                    button: event.button.into_bones(),
                    state: event.state.into_bones(),
                })
                .collect(),
        },
        bones::KeyboardInputs {
            keys: keyboard_events
                .iter()
                .map(|event| bones::KeyboardInput {
                    scan_code: event.scan_code,
                    key_code: event.key_code.map(|x| x.into_bones()),
                    button_state: event.state.into_bones(),
                })
                .collect(),
        },
    )
}

/// System to step the bones simulation.
fn step_bones_game(
    In((mouse_inputs, keyboard_inputs)): In<(bones::MouseInputs, bones::KeyboardInputs)>,
    world: &mut World,
) {
    world.resource_scope(|world: &mut World, mut data: Mut<BonesData>| {
        let egui_ctx = {
            let mut egui_query = world.query_filtered::<&mut EguiContext, With<Window>>();
            let mut egui_ctx = egui_query.get_single_mut(world).unwrap();
            egui_ctx.get_mut().clone()
        };
        let BonesData { game, asset_server } = &mut *data;
        let bevy_time = world.resource::<Time>();

        let mouse_inputs = bones::AtomicResource::new(mouse_inputs);
        let keyboard_inputs = bones::AtomicResource::new(keyboard_inputs);

        // Step the game simulation
        game.step(|bones_world| {
            // Insert egui context if not present
            if !bones_world
                .resources
                .contains::<bones_framework::render::ui::EguiCtx>()
            {
                bones_world
                    .resources
                    .insert(bones_framework::render::ui::EguiCtx(egui_ctx.clone()));
            }

            // Update bones time
            {
                // Initialize the time resource if it doesn't exist.
                if !bones_world.resources.contains::<bones::Time>() {
                    bones_world.init_resource::<bones::Time>();
                }

                let mut time = bones_world.resource_mut::<bones::Time>();

                // Use the Bevy time if it's available, otherwise use the default time.
                if let Some(instant) = bevy_time.last_update() {
                    time.update_with_instant(instant);
                } else {
                    time.update();
                }
            }

            // Insert asset server if not present
            if !bones_world.resources.contains::<bones::AssetServer>() {
                bones_world.resources.insert_cell(asset_server.clone_cell());
            }

            // Update the inputs.
            bones_world.resources.insert_cell(mouse_inputs.clone_cell());
            bones_world
                .resources
                .insert_cell(keyboard_inputs.clone_cell());
        });
    });
}

fn sync_clear_color(mut clear_color: ResMut<ClearColor>, mut data: ResMut<BonesData>) {
    let game = &mut data.game;

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

fn sync_egui_settings(
    data: Res<BonesData>,
    mut bevy_egui_settings: ResMut<bevy_egui::EguiSettings>,
) {
    let game = &data.game;

    for session_name in &game.sorted_session_keys {
        let session = game.sessions.get(*session_name).unwrap();
        let world = &session.world;

        if let Some(settings) = world.get_resource::<bones::EguiSettings>() {
            bevy_egui_settings.scale_factor = settings.scale;
        }
    }
}

/// Sync bones cameras with Bevy
fn sync_cameras(
    mut commands: Commands,
    data: Res<BonesData>,
    mut bevy_bones_cameras: Query<
        (
            Entity,
            &mut Camera,
            &mut OrthographicProjection,
            &mut Transform,
        ),
        With<BevyBonesEntity>,
    >,
) {
    let game = &data.game;

    // let bones_cameras = game.sorted_session_keys
    //     .iter()
    //     .map(|name| game.sessions.get(*name).unwrap())
    //     .filter(|session| session.visible)
    //     .filter(|session| {
    //         let world = &session.world;
    //         (world.components.try_get_cell::<bones::Transform>().is_ok()
    //             && world.components.try_get_cell::<bones::Camera>().is_ok())
    //     })
    //     .map(|session| {
    //         let world = &session.world;
    //         let entities = world.resource::<bones::Entities>();
    //         let transforms = world.components.get_cell::<bones::Transform>();
    //         let cameras = world.components.get_cell::<bones::Camera>();
    //         let cameras = cameras.borrow();

    //         entities
    //     });

    for session_name in &game.sorted_session_keys {
        let session = game.sessions.get(*session_name).unwrap();

        let world = &session.world;

        // Skip worlds without cameras and transforms
        if !(world.components.get_cell::<bones::Transform>().is_ok()
            && world.components.get_cell::<bones::Camera>().is_ok())
        {
            continue;
        }

        let entities = world.resource::<bones::Entities>();
        let transforms = world.components.get::<bones::Transform>().unwrap();
        let cameras = world.components.get::<bones::Camera>().unwrap();

        // Sync cameras
        let mut cameras_bitset = cameras.bitset().clone();
        cameras_bitset.bit_and(transforms.bitset());
        let mut bones_camera_entity_iter = entities.iter_with_bitset(&cameras_bitset);
        for (bevy_ent, mut camera, mut projection, mut transform) in &mut bevy_bones_cameras {
            if let Some(bones_ent) = bones_camera_entity_iter.next() {
                let bones_camera = cameras.get(bones_ent).unwrap();
                let bones_transform = transforms.get(bones_ent).unwrap();

                camera.is_active = bones_camera.active;
                match projection.scaling_mode {
                    ScalingMode::FixedVertical(height) if height != bones_camera.height => {
                        projection.scaling_mode = ScalingMode::FixedVertical(bones_camera.height)
                    }
                    _ => (),
                }
                camera.viewport = bones_camera
                    .viewport
                    .map(|x| bevy::render::camera::Viewport {
                        physical_position: x.position,
                        physical_size: x.size,
                        depth: x.depth_min..x.depth_max,
                    });

                *transform = bones_transform.into_bevy();
            } else {
                commands.entity(bevy_ent).despawn();
            }
        }
        for bones_ent in bones_camera_entity_iter {
            let bones_camera = cameras.get(bones_ent).unwrap();
            let bones_transform = transforms.get(bones_ent).unwrap();

            commands.spawn((
                Camera2dBundle {
                    camera: Camera {
                        is_active: bones_camera.active,
                        ..default()
                    },
                    projection: OrthographicProjection {
                        scaling_mode: ScalingMode::FixedVertical(bones_camera.height),
                        ..default()
                    },
                    transform: bones_transform.into_bevy(),
                    ..default()
                },
                BevyBonesEntity,
            ));
        }
    }
}

fn extract_bones_sprites(
    mut extracted_sprites: ResMut<ExtractedSprites>,
    data: Extract<Res<BonesData>>,
    bones_image_ids: Extract<Res<BonesImageIds>>,
    bones_renderable_entity: Extract<Res<BonesGameEntity>>,
) {
    let game = &data.game;
    let bones_assets = data.asset_server.borrow();

    for session_name in &game.sorted_session_keys {
        let session = game.sessions.get(*session_name).unwrap();

        let world = &session.world;

        // Skip worlds without cameras and transforms
        if !(world.components.get_cell::<bones::Transform>().is_ok()
            && world.components.get_cell::<bones::Camera>().is_ok()
            && (world.components.get_cell::<bones::Sprite>().is_ok()
                || world.components.get_cell::<bones::AtlasSprite>().is_ok()))
        {
            continue;
        }

        let entities = world.resource::<bones::Entities>();
        let transforms = world.components.get::<bones::Transform>().unwrap();

        // Extract normal sprites
        if let Ok(sprites) = world.components.get::<bones::Sprite>() {
            for (_, (sprite, transform)) in entities.iter_with((&sprites, &transforms)) {
                let sprite_image = bones_assets.get(&sprite.image);
                let image_id = if let bones::Image::External(id) = sprite_image {
                    *id
                } else {
                    panic!(
                        "Images added at runtime not supported yet, \
                please open an issue."
                    );
                };
                extracted_sprites.sprites.push(ExtractedSprite {
                    entity: bones_renderable_entity.0,
                    transform: transform.into_bevy().into(),
                    color: sprite.color.into_bevy(),
                    rect: None,
                    custom_size: None,
                    image_handle_id: bones_image_ids.get(&image_id).unwrap().id(),
                    flip_x: sprite.flip_x,
                    flip_y: sprite.flip_y,
                    anchor: Vec2::ZERO,
                })
            }
        }

        // Extract atlas sprites
        if let Ok(atlas_sprites) = world.components.get_cell::<bones::AtlasSprite>() {
            let atlas_sprites = atlas_sprites.borrow();
            for (_, (atlas_sprite, transform)) in entities.iter_with((&atlas_sprites, &transforms))
            {
                let atlas = bones_assets.get(&atlas_sprite.atlas);
                let atlas_image = bones_assets.get(&atlas.image);
                let image_id = if let bones::Image::External(id) = atlas_image {
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
                    anchor: Vec2::ZERO,
                })
            }
        }
    }
}

// fn sync_tilemaps<W: HasBonesRenderer>(
//     mut commands: Commands,
//     world_resource: Option<ResMut<W>>,
//     mut bevy_bones_tile_layers: Query<
//         (
//             Entity,
//             &mut TileMap,
//             &mut Handle<TextureAtlas>,
//             &mut Transform,
//         ),
//         With<BevyBonesEntity>,
//     >,
//     atlas_assets: Res<Assets<TextureAtlas>>,
// ) {
//     let Some(mut world_resource) = world_resource else {
//         bevy_bones_tile_layers.for_each(|(e, ..)| commands.entity(e).despawn());
//         return;
//     };

//     let world = world_resource.game();

//     world.components.init::<bones::Tile>();
//     world.components.init::<bones::TileLayer>();

//     let entities = world.resource::<bones::Entities>();
//     let entities = entities.borrow();
//     let tiles = world.components.get::<bones::Tile>();
//     let tiles = tiles.borrow();
//     let tile_layers = world.components.get::<bones::TileLayer>();
//     let tile_layers = tile_layers.borrow();
//     let transforms = world.components.get::<bones::Transform>();
//     let transforms = transforms.borrow();

//     // Sync tile layers
//     let mut tile_layers_bitset = tile_layers.bitset().clone();
//     tile_layers_bitset.bit_and(transforms.bitset());

//     let mut bones_tile_layer_entity_iter = entities.iter_with_bitset(&tile_layers_bitset);
//     for (bevy_ent, mut tile_map, mut atlas, mut transform) in &mut bevy_bones_tile_layers {
//         if let Some(bones_ent) = bones_tile_layer_entity_iter.next() {
//             let bones_tile_layer = tile_layers.get(bones_ent).unwrap();
//             let bones_transform = transforms.get(bones_ent).unwrap();

//             *atlas = bones_tile_layer.atlas.get_bevy_handle_untyped().typed();
//             *transform = bones_transform.into_bevy();
//             transform.translation += bones_tile_layer.tile_size.extend(0.0) / 2.0;

//             let Some(texture_atlas) = atlas_assets.get(&atlas) else { continue; };
//             let atlas_grid_size = texture_atlas.size / texture_atlas.textures[0].size();
//             let max_tile_idx = (atlas_grid_size.x * atlas_grid_size.y) as u32 - 1;

//             let grid_size = bones_tile_layer.grid_size;
//             let tile_iter = bones_tile_layer
//                 .tiles
//                 .iter()
//                 .enumerate()
//                 .map(|(idx, entity)| {
//                     let y = idx as u32 / grid_size.x;
//                     let x = idx as u32 - (y * grid_size.x);
//                     let tile = entity
//                         .map(|e| {
//                             let tile = tiles.get(e)?;
//                             Some(Tile {
//                                 sprite_index: (tile.idx as u32).min(max_tile_idx),
//                                 color: default(),
//                                 flags: if tile.flip_x {
//                                     TileFlags::FLIP_X
//                                 } else {
//                                     TileFlags::empty()
//                                 } | if tile.flip_y {
//                                     TileFlags::FLIP_Y
//                                 } else {
//                                     TileFlags::empty()
//                                 },
//                             })
//                         })
//                         .flatten();
//                     (IVec3::new(x as i32, y as i32, 0), tile)
//                 });

//             tile_map.clear();
//             tile_map.set_tiles(tile_iter);

//             // This is maybe a bug in bevy_simple_tilemap. If the tilemap atlas has been changed,
//             // and one of the tiles in the map had a tile index greater than the max tile count in
//             // the new atlas, the map renderer will panic.
//             //
//             // This shouldn't happen because we made sure to `clear()` the tiles and ensured that
//             // all the new tile indexes are clamped, but apparently the chunks are updated a frame
//             // late or otherwise just evaluated before our tile changes take effect, so we must
//             // clamp the tiles indexes directly on the chunks as well.
//             tile_map.chunks.iter_mut().for_each(|(_, chunk)| {
//                 chunk
//                     .tiles
//                     .iter_mut()
//                     .flatten()
//                     .for_each(|x| x.sprite_index = x.sprite_index.min(max_tile_idx))
//             });
//         } else {
//             commands.entity(bevy_ent).despawn();
//         }
//     }
//     for bones_ent in bones_tile_layer_entity_iter {
//         let bones_tile_layer = tile_layers.get(bones_ent).unwrap();
//         let bones_transform = transforms.get(bones_ent).unwrap();

//         let mut tile_map = TileMap::default();

//         let grid_size = bones_tile_layer.grid_size;
//         let tile_iter = bones_tile_layer
//             .tiles
//             .iter()
//             .enumerate()
//             .map(|(idx, entity)| {
//                 let y = idx as u32 / grid_size.x;
//                 let x = idx as u32 - (y * grid_size.x);
//                 let tile = entity
//                     .map(|e| {
//                         let tile = tiles.get(e)?;
//                         Some(Tile {
//                             sprite_index: tile.idx as _,
//                             color: default(),
//                             flags: if tile.flip_x {
//                                 TileFlags::FLIP_X
//                             } else {
//                                 TileFlags::empty()
//                             } | if tile.flip_y {
//                                 TileFlags::FLIP_Y
//                             } else {
//                                 TileFlags::empty()
//                             },
//                         })
//                     })
//                     .flatten();
//                 (IVec3::new(x as i32, y as i32, 0), tile)
//             });

//         tile_map.set_tiles(tile_iter);

//         let mut transform = bones_transform.into_bevy();
//         transform.translation += bones_tile_layer.tile_size.extend(0.0) / 2.0;
//         commands.spawn((
//             TileMapBundle {
//                 tilemap: tile_map,
//                 transform,
//                 ..default()
//             },
//             BevyBonesEntity,
//         ));
//     }
// }

// /// The system that renders the bones world.
// fn sync_path2ds<W: HasBonesRenderer>(
//     mut commands: Commands,
//     world_resource: Option<ResMut<W>>,
//     mut bevy_bones_path2ds: Query<
//         (Entity, &mut lyon::Path, &mut lyon::Stroke, &mut Transform),
//         With<BevyBonesEntity>,
//     >,
// ) {
//     let Some(mut world_resource) = world_resource else {
//         bevy_bones_path2ds.for_each(|(e, ..)| commands.entity(e).despawn());
//         return;
//     };

//     let world = world_resource.game();

//     world.components.init::<bones::Path2d>();
//     world.components.init::<bones::Transform>();

//     let entities = world.resource::<bones::Entities>();
//     let entities = entities.borrow();
//     let path2ds = world.components.get::<bones::Path2d>();
//     let path2ds = path2ds.borrow();
//     let transforms = world.components.get::<bones::Transform>();
//     let transforms = transforms.borrow();

//     fn get_bevy_components(
//         bones_path2d: &bones::Path2d,
//         bones_transform: &bones::Transform,
//     ) -> (lyon::Stroke, lyon::Path, Transform) {
//         let stroke = lyon::Stroke::new(bones_path2d.color.into_bevy(), bones_path2d.thickness);
//         let new_path = bones_path2d
//             .points
//             .iter()
//             .copied()
//             .enumerate()
//             .fold(lyon::PathBuilder::new(), |mut builder, (i, point)| {
//                 if i > 0 && !bones_path2d.line_breaks.contains(&i) {
//                     builder.line_to(point);
//                 }
//                 builder.move_to(point);

//                 builder
//             })
//             .build();

//         let mut transform = bones_transform.into_bevy();
//         // Offset the path towards the camera slightly to make sure it renders on top of a
//         // sprite/etc. if it is applied to an entity with both a sprite and a path.
//         transform.translation.z += 0.0001;
//         (stroke, new_path, transform)
//     }

//     // Sync paths
//     let mut path2ds_bitset = path2ds.bitset().clone();
//     path2ds_bitset.bit_and(transforms.bitset());
//     let mut bones_sprite_entity_iter = entities.iter_with_bitset(&path2ds_bitset);
//     for (bevy_ent, mut path, mut draw_mode, mut transform) in &mut bevy_bones_path2ds {
//         if let Some(bones_ent) = bones_sprite_entity_iter.next() {
//             let bones_path2d = path2ds.get(bones_ent).unwrap();
//             let bones_transform = transforms.get(bones_ent).unwrap();

//             (*draw_mode, *path, *transform) = get_bevy_components(bones_path2d, bones_transform);
//         } else {
//             commands.entity(bevy_ent).despawn();
//         }
//     }
//     for bones_ent in bones_sprite_entity_iter {
//         let bones_path2d = path2ds.get(bones_ent).unwrap();
//         let bones_transform = transforms.get(bones_ent).unwrap();

//         let (stroke, path, transform) = get_bevy_components(bones_path2d, bones_transform);

//         commands.spawn((
//             lyon::ShapeBundle {
//                 path,
//                 transform,
//                 ..default()
//             },
//             stroke,
//             BevyBonesEntity,
//         ));
//     }
// }
