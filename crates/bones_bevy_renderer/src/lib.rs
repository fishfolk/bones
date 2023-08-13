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
};
use bevy_egui::EguiContext;
use bevy_prototype_lyon::prelude as lyon;

use bones_framework::prelude as bones;
use prelude::convert::IntoBones;

/// The prelude
pub mod prelude {
    pub use crate::*;
}

mod convert;

/// Marker component for entities that are rendered in Bevy for bones.
#[derive(Component)]
pub struct BevyBonesEntity;

/// [`SystemSet`] marker for sets added by bones to the Bevy world.
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
#[system_set(base)]
pub enum BonesStage {
    /// This stage is run after [`CoreSet::First`] to synchronize the bevy `Time` resource with
    /// the bones one.
    SyncTime,
    /// This is the stage where the plugin reads the bones world adds bevy sprites, tiles, etc. to
    /// be rendered.
    SyncRender,
}

/// Renderer for [`bones_framework`] [`Game`][bones::Game]s using Bevy.
#[derive(Resource)]
pub struct BonesBevyRenderer {
    /// The bones game to run.
    pub game: bones::Game,
    /// The version of the game, used for the asset loader.
    pub game_version: bones::Version,
    /// The path to load assets from.
    pub asset_dir: PathBuf,
    /// The path to load asset packs from.
    pub packs_dir: PathBuf,
    /// Function to configure the bones asset server.
    pub asset_server_config_fn: Box<dyn FnOnce(&mut bones::AssetServer) + Sync + Send + 'static>,
}

/// Bevy resource that contains the info for the bones game that is being rendered.
#[derive(Resource)]
pub struct BonesData {
    /// The bones game.
    pub game: bones::Game,
    /// The bones asset server.
    pub asset_server: bones::AtomicResource<bones::AssetServer>,
}

impl BonesBevyRenderer {
    /// Create a new [`BevyBonesRenderer`] for the provided game.
    pub fn new<F: FnOnce(&mut bones::AssetServer) + Sync + Send + 'static>(
        game: bones::Game,
        configure_asset_server: F,
    ) -> Self {
        BonesBevyRenderer {
            game,
            game_version: bones::Version::new(0, 1, 0),
            asset_dir: PathBuf::from("assets"),
            packs_dir: PathBuf::from("packs"),
            asset_server_config_fn: Box::new(configure_asset_server),
        }
    }

    /// Return a bevy [`App`] configured to run the bones game.
    pub fn app(self) -> App {
        let mut app = App::new();

        // Initialize Bevy plugins we use
        app.add_plugins(DefaultPlugins)
            .add_plugin(bevy_simple_tilemap::plugin::SimpleTileMapPlugin)
            .add_plugin(bevy_egui::EguiPlugin)
            .add_plugin(lyon::ShapePlugin);

        // Create the asset server
        let mut asset_server = bones::AssetServer::new(
            bones::FileAssetIo {
                core_dir: self.asset_dir.clone(),
                packs_dir: self.packs_dir.clone(),
            },
            self.game_version,
        );
        // Configure the asset server
        (self.asset_server_config_fn)(&mut asset_server);

        // Load the game assets
        asset_server
            .load_assets()
            .expect("Could not load game assets");

        // Insert the bones data
        app.insert_resource(BonesData {
            game: self.game,
            asset_server: bones::AtomicResource::new(asset_server),
        });

        // Configure the bones stages
        app.configure_set(BonesStage::SyncTime.after(CoreSet::First))
            .configure_set(BonesStage::SyncRender.before(CoreSet::Update));

        // Add the world sync systems
        app.add_system(get_bones_input.pipe(step_bones_game));

        // .add_systems(
        //     (
        //         sync_sprites,
        //         sync_cameras,
        //         sync_path2ds,
        //         sync_tilemaps,
        //         sync_clear_color,
        //         sync_atlas_sprites,
        //     )
        //         .in_base_set(BonesStage::SyncRender),
        // );

        // app.add_system(sync_time.in_base_set(BonesStage::SyncTime));

        app
    }
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

fn sync_clear_color(mut clear_color: ResMut<ClearColor>, data: ResMut<BonesData>) {
    // let game = &mut data.game;
    // game.init_resource::<bones::ClearColor>();

    // let bones_clear_color = game.resource::<bones::ClearColor>();

    // clear_color.0 = bones_clear_color.0.into_bevy()
}

// /// The system that renders the bones world.
// fn sync_sprites<W: HasBonesRenderer>(
//     mut commands: Commands,
//     world_resource: Option<ResMut<W>>,
//     mut bevy_bones_sprites: Query<
//         (Entity, &mut Handle<Image>, &mut Sprite, &mut Transform),
//         With<BevyBonesEntity>,
//     >,
// ) {
//     let Some(mut world_resource) = world_resource else {
//         bevy_bones_sprites.for_each(|(e, ..)| commands.entity(e).despawn());
//         return;
//     };

//     let world = world_resource.game();

//     // TODO: Evaluate cost of initializing bones render components every frame.
//     world.components.init::<bones::Sprite>();
//     world.components.init::<bones::Transform>();

//     let entities = world.resource::<bones::Entities>();
//     let entities = entities.borrow();
//     let sprites = world.components.get::<bones::Sprite>();
//     let sprites = sprites.borrow();
//     let transforms = world.components.get::<bones::Transform>();
//     let transforms = transforms.borrow();

//     // Sync sprites
//     let mut sprites_bitset = sprites.bitset().clone();
//     sprites_bitset.bit_and(transforms.bitset());
//     let mut bones_sprite_entity_iter = entities.iter_with_bitset(&sprites_bitset);
//     for (bevy_ent, mut image, mut sprite, mut transform) in &mut bevy_bones_sprites {
//         if let Some(bones_ent) = bones_sprite_entity_iter.next() {
//             let bones_sprite = sprites.get(bones_ent).unwrap();
//             let bones_transform = transforms.get(bones_ent).unwrap();

//             sprite.flip_x = bones_sprite.flip_x;
//             sprite.flip_y = bones_sprite.flip_y;
//             sprite.color = bones_sprite.color.into_bevy();
//             *image = bones_sprite.image.get_bevy_handle_untyped().typed();
//             *transform = bones_transform.into_bevy();
//         } else {
//             commands.entity(bevy_ent).despawn();
//         }
//     }
//     for bones_ent in bones_sprite_entity_iter {
//         let bones_sprite = sprites.get(bones_ent).unwrap();
//         let bones_transform = transforms.get(bones_ent).unwrap();

//         commands.spawn((
//             SpriteBundle {
//                 texture: bones_sprite.image.get_bevy_handle_untyped().typed(),
//                 transform: bones_transform.into_bevy(),
//                 ..default()
//             },
//             BevyBonesEntity,
//         ));
//     }
// }

// /// The system that renders the bones world.
// fn sync_atlas_sprites<W: HasBonesRenderer>(
//     mut commands: Commands,
//     world_resource: Option<ResMut<W>>,
//     mut bevy_bones_atlases: Query<
//         (
//             Entity,
//             &mut Handle<TextureAtlas>,
//             &mut TextureAtlasSprite,
//             &mut Transform,
//         ),
//         With<BevyBonesEntity>,
//     >,
// ) {
//     let Some(mut world_resource) = world_resource else {
//         bevy_bones_atlases.for_each(|(e, ..)| commands.entity(e).despawn());
//         return;
//     };

//     let world = world_resource.game();

//     world.components.init::<bones::AtlasSprite>();
//     world.components.init::<bones::Transform>();

//     let entities = world.resource::<bones::Entities>();
//     let entities = entities.borrow();
//     let atlas_sprites = world.components.get::<bones::AtlasSprite>();
//     let atlas_sprites = atlas_sprites.borrow();
//     let transforms = world.components.get::<bones::Transform>();
//     let transforms = transforms.borrow();

//     // Sync atlas sprites
//     let mut atlas_bitset = atlas_sprites.bitset().clone();
//     atlas_bitset.bit_and(transforms.bitset());
//     let mut bones_atlas_sprite_entity_iter = entities.iter_with_bitset(&atlas_bitset);
//     for (bevy_ent, mut image, mut atlas_sprite, mut transform) in &mut bevy_bones_atlases {
//         if let Some(bones_ent) = bones_atlas_sprite_entity_iter.next() {
//             let bones_atlas = atlas_sprites.get(bones_ent).unwrap();
//             let bones_transform = transforms.get(bones_ent).unwrap();

//             *image = bones_atlas.atlas.get_bevy_handle_untyped().typed();
//             *transform = bones_transform.into_bevy();

//             atlas_sprite.index = bones_atlas.index;
//             atlas_sprite.flip_x = bones_atlas.flip_x;
//             atlas_sprite.flip_y = bones_atlas.flip_y;
//             atlas_sprite.color = bones_atlas.color.into_bevy();
//         } else {
//             commands.entity(bevy_ent).despawn();
//         }
//     }
//     for bones_ent in bones_atlas_sprite_entity_iter {
//         let bones_atlas = atlas_sprites.get(bones_ent).unwrap();
//         let bones_transform = transforms.get(bones_ent).unwrap();

//         commands.spawn((
//             SpriteSheetBundle {
//                 texture_atlas: bones_atlas.atlas.get_bevy_handle_untyped().typed(),
//                 transform: bones_transform.into_bevy(),
//                 ..default()
//             },
//             BevyBonesEntity,
//         ));
//     }
// }

// /// The system that renders the bones world.
// fn sync_cameras<W: HasBonesRenderer>(
//     mut commands: Commands,
//     world_resource: Option<ResMut<W>>,
//     mut bevy_bones_cameras: Query<
//         (
//             Entity,
//             &mut Camera,
//             &mut OrthographicProjection,
//             &mut Transform,
//         ),
//         With<BevyBonesEntity>,
//     >,
// ) {
//     let Some(mut world_resource) = world_resource else {
//         bevy_bones_cameras.for_each(|(e, ..)| commands.entity(e).despawn());
//         return;
//     };

//     let world = world_resource.game();

//     world.components.init::<bones::Transform>();
//     world.components.init::<bones::Camera>();

//     let entities = world.resource::<bones::Entities>();
//     let entities = entities.borrow();
//     let transforms = world.components.get::<bones::Transform>();
//     let transforms = transforms.borrow();
//     let cameras = world.components.get::<bones::Camera>();
//     let cameras = cameras.borrow();

//     // Sync cameras
//     let mut cameras_bitset = cameras.bitset().clone();
//     cameras_bitset.bit_and(transforms.bitset());
//     let mut bones_camera_entity_iter = entities.iter_with_bitset(&cameras_bitset);
//     for (bevy_ent, mut camera, mut projection, mut transform) in &mut bevy_bones_cameras {
//         if let Some(bones_ent) = bones_camera_entity_iter.next() {
//             let bones_camera = cameras.get(bones_ent).unwrap();
//             let bones_transform = transforms.get(bones_ent).unwrap();

//             camera.is_active = bones_camera.active;
//             match projection.scaling_mode {
//                 ScalingMode::FixedVertical(height) if height != bones_camera.height => {
//                     projection.scaling_mode = ScalingMode::FixedVertical(bones_camera.height)
//                 }
//                 _ => (),
//             }
//             camera.viewport = bones_camera
//                 .viewport
//                 .map(|x| bevy::render::camera::Viewport {
//                     physical_position: x.position,
//                     physical_size: x.size,
//                     depth: x.depth_min..x.depth_max,
//                 });

//             *transform = bones_transform.into_bevy();
//         } else {
//             commands.entity(bevy_ent).despawn();
//         }
//     }
//     for bones_ent in bones_camera_entity_iter {
//         let bones_camera = cameras.get(bones_ent).unwrap();
//         let bones_transform = transforms.get(bones_ent).unwrap();

//         commands.spawn((
//             Camera2dBundle {
//                 camera: Camera {
//                     is_active: bones_camera.active,
//                     ..default()
//                 },
//                 projection: OrthographicProjection {
//                     scaling_mode: ScalingMode::FixedVertical(bones_camera.height),
//                     ..default()
//                 },
//                 transform: bones_transform.into_bevy(),
//                 ..default()
//             },
//             BevyBonesEntity,
//         ));
//     }
// }

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

// /// The system that renders the bones world.
// fn sync_time<W: HasBonesRenderer>(
//     world_resource: Option<ResMut<W>>,
//     bevy_time: Res<bevy::prelude::Time>,
// ) {
//     let Some(mut world_resource) = world_resource else {
//         return;
//     };
//     let world = world_resource.game();

//     // Initialize the time resource if it doesn't exist.
//     if world
//         .get_atomic_resource::<bones_lib::prelude::Time>()
//         .is_none()
//     {
//         world.init_resource::<bones_lib::prelude::Time>();
//     }

//     let time = world.resource::<bones_lib::prelude::Time>();
//     let mut time = time.borrow_mut();

//     // Use the Bevy time if it's available, otherwise use the default time.
//     if let Some(instant) = bevy_time.last_update() {
//         time.update_with_instant(instant);
//     } else {
//         time.update();
//     }
// }
