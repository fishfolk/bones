#![allow(clippy::too_many_arguments)]

use bones_bevy_renderer::{
    bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    BonesBevyRenderer,
};
use bones_framework::prelude::*;

/// Create our root asset type.
///
/// The path to our root asset file is specified in `assets/pack.yaml`.
#[derive(HasSchema, Default, Clone)]
#[repr(C)]
// Allow asset to be loaded from "game.yaml" assets.
#[type_data(metadata_asset("game"))]
struct GameMeta {
    /// The image displayed on the menu.
    menu_image: Handle<Image>,
    /// The image for the sprite demo
    sprite_demo: Handle<Image>,
    /// Character information that will be loaded from a separate asset file.
    atlas_demo: Handle<AtlasDemoMeta>,
    /// The tilemap demo metadata.
    tilemap_demo: Handle<TilemapDemoMeta>,
    /// The color the debug lines in the debug line demo.
    path2d_color: Color,
    /// Localization asset
    localization: Handle<LocalizationAsset>,
    /// The font to use for the demo title.
    title_font: FontMeta,
    /// The list of font files to load for the UI.
    fonts: SVec<Handle<Font>>,
    /// The border to use the for main menu.
    menu_border: BorderImageMeta,
    /// The style to use for buttons.
    button_style: ButtonThemeMeta,
}

/// Atlas information.
#[derive(HasSchema, Default, Clone)]
#[repr(C)]
#[type_data(metadata_asset("atlas-demo"))]
struct AtlasDemoMeta {
    /// The sprite atlas for the player.
    pub atlas: Handle<Atlas>,
    /// The frames-per-second of the animation.
    pub fps: f32,
    /// The frames of the animation.
    ///
    /// Note: We use an [`SVec`] here because it implements [`HasSchema`], allowing it to be loaded
    /// in a metadata asset.
    pub animation: SVec<u32>,
}

/// Tilemap info.
#[derive(HasSchema, Default, Clone)]
#[repr(C)]
#[type_data(metadata_asset("tilemap"))]
struct TilemapDemoMeta {
    /// The atlas that will be used for the tilemap.
    pub atlas: Handle<Atlas>,
    /// The size of the tile map in tiles.
    pub map_size: UVec2,
    /// The information about each tile in the tilemap.
    pub tiles: SVec<TileMeta>,
}

/// Tile info.
#[derive(HasSchema, Default, Clone)]
#[repr(C)]
struct TileMeta {
    /// The tile position.
    pos: UVec2,
    /// The index of the tile in the atlas.
    idx: u32,
}

fn main() {
    assert!(Color::schema()
        .type_data
        .get::<SchemaDeserialize>()
        .is_some());

    // Create a bones bevy renderer from our bones game
    BonesBevyRenderer::new(create_game())
        // Get a bevy app for running our game
        .app()
        // We can add our own bevy plugins now
        .add_plugins((FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin::default()))
        // And run the bevy app
        .run()
}

// Initialize the game.
pub fn create_game() -> Game {
    // Create an empty game
    let mut game = Game::new();

    // Configure the asset server
    game.init_shared_resource::<AssetServer>()
        // Register the default asset types
        .register_default_assets()
        // Register our custom asset types
        .register_asset::<GameMeta>()
        .register_asset::<AtlasDemoMeta>()
        .register_asset::<TilemapDemoMeta>();

    // Create our menu session
    game.sessions.create("menu").install_plugin(menu_plugin);

    game
}

/// Menu plugin
pub fn menu_plugin(session: &mut Session) {
    // Register our menu system
    session
        // Install the bones_framework default plugins for this session
        .install_plugin(DefaultPlugin)
        // And add our systems.
        .add_system_to_stage(Update, menu_system)
        .add_startup_system(menu_startup);
}

/// Setup the main menu.
fn menu_startup(
    mut egui_settings: ResMutInit<EguiSettings>,
    mut clear_color: ResMutInit<ClearColor>,
) {
    // Set the clear color
    **clear_color = Color::BLACK;
    // Set the egui scale
    egui_settings.scale = 2.0;
}

/// Our main menu system.
fn menu_system(
    meta: Root<GameMeta>,
    egui_ctx: ResMut<EguiCtx>,
    mut sessions: ResMut<Sessions>,
    mut session_options: ResMut<SessionOptions>,
    egui_textures: Res<EguiTextures>,
    // Get the localization field from our `GameMeta`
    localization: Localization<GameMeta>,
) {
    // Render the menu.
    egui::CentralPanel::default()
        .frame(egui::Frame::none())
        .show(&egui_ctx, |ui| {
            BorderedFrame::new(&meta.menu_border).show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(20.0);
                    ui.label(meta.title_font.rich(localization.get("title")));
                    ui.add_space(20.0);

                    if BorderedButton::themed(&meta.button_style, localization.get("sprite-demo"))
                        .show(ui)
                        .clicked()
                    {
                        // Delete the menu world
                        session_options.delete = true;

                        // Create a session for the match
                        sessions
                            .create("sprite_demo")
                            .install_plugin(sprite_demo_plugin);
                    }

                    if BorderedButton::themed(&meta.button_style, localization.get("atlas-demo"))
                        .show(ui)
                        .clicked()
                    {
                        // Delete the menu world
                        session_options.delete = true;

                        // Create a session for the match
                        sessions
                            .create("atlas_demo")
                            .install_plugin(atlas_demo_plugin);
                    }

                    if BorderedButton::themed(&meta.button_style, localization.get("tilemap-demo"))
                        .show(ui)
                        .clicked()
                    {
                        // Delete the menu world
                        session_options.delete = true;

                        // Create a session for the match
                        sessions
                            .create("tilemap_demo")
                            .install_plugin(tilemap_demo_plugin);
                    }

                    if BorderedButton::themed(&meta.button_style, localization.get("path2d-demo"))
                        .show(ui)
                        .clicked()
                    {
                        // Delete the menu world
                        session_options.delete = true;

                        // Create a session for the match
                        sessions
                            .create("path2d_demo")
                            .install_plugin(path2d_demo_plugin);
                    }

                    ui.add_space(30.0);

                    // When using a bones image in egui, we have to get it's corresponding egui texture
                    // from the egui textures resource.
                    ui.image(egui_textures.get(meta.menu_image), [100., 100.]);

                    ui.add_space(30.0);
                });
            })
        });
}

/// Plugin for running the sprite demo.
fn sprite_demo_plugin(session: &mut Session) {
    session
        .install_plugin(DefaultPlugin)
        .add_startup_system(sprite_demo_startup)
        .add_system_to_stage(Update, back_to_menu_ui);
}

/// System that spawns the sprite demo.
fn sprite_demo_startup(
    mut entities: ResMut<Entities>,
    mut sprites: CompMut<Sprite>,
    mut transforms: CompMut<Transform>,
    mut cameras: CompMut<Camera>,
    meta: Root<GameMeta>,
) {
    spawn_default_camera(&mut entities, &mut transforms, &mut cameras);

    let sprite_ent = entities.create();
    transforms.insert(sprite_ent, default());
    sprites.insert(
        sprite_ent,
        Sprite {
            image: meta.sprite_demo,
            ..default()
        },
    );
}

/// Plugin for running the tilemap demo.
fn tilemap_demo_plugin(session: &mut Session) {
    session
        .install_plugin(DefaultPlugin)
        .add_startup_system(tilemap_startup_system)
        .add_system_to_stage(Update, back_to_menu_ui);
}

/// System for starting up the tilemap demo.
fn tilemap_startup_system(
    mut entities: ResMut<Entities>,
    mut transforms: CompMut<Transform>,
    mut tile_layers: CompMut<TileLayer>,
    mut cameras: CompMut<Camera>,
    mut tiles: CompMut<Tile>,
    meta: Root<GameMeta>,
    assets: Res<AssetServer>,
) {
    spawn_default_camera(&mut entities, &mut transforms, &mut cameras);

    // Load our map and atlas info
    let map_info = assets.get(meta.tilemap_demo);
    let atlas = assets.get(map_info.atlas);

    // Create a new map layer
    let mut layer = TileLayer::new(map_info.map_size, atlas.tile_size, map_info.atlas);

    // Load the layer up with the tiles from our metadata
    for tile in &map_info.tiles {
        let tile_ent = entities.create();
        tiles.insert(
            tile_ent,
            Tile {
                idx: tile.idx,
                ..default()
            },
        );
        layer.set(tile.pos, Some(tile_ent))
    }

    // Spawn the layer
    let layer_ent = entities.create();
    tile_layers.insert(layer_ent, layer);
    transforms.insert(layer_ent, default());
}

/// Plugin for running the atlas demo.
fn atlas_demo_plugin(session: &mut Session) {
    session
        .install_plugin(DefaultPlugin)
        .add_startup_system(atlas_demo_startup)
        .add_system_to_stage(Update, back_to_menu_ui);
}

/// System to startup the atlas demo.
fn atlas_demo_startup(
    mut entities: ResMut<Entities>,
    mut transforms: CompMut<Transform>,
    mut cameras: CompMut<Camera>,
    mut atlas_sprites: CompMut<AtlasSprite>,
    mut animated_sprites: CompMut<AnimatedSprite>,
    mut clear_color: ResMutInit<ClearColor>,
    meta: Root<GameMeta>,
    assets: Res<AssetServer>,
) {
    // Set the clear color
    **clear_color = Color::GRAY;

    // Spawn the camera
    spawn_default_camera(&mut entities, &mut transforms, &mut cameras);

    // Get the atlas metadata
    let demo = assets.get(meta.atlas_demo);

    // Spawn the character sprite.
    let sprite_ent = entities.create();
    transforms.insert(sprite_ent, default());
    atlas_sprites.insert(
        sprite_ent,
        AtlasSprite {
            atlas: demo.atlas,
            ..default()
        },
    );
    animated_sprites.insert(
        sprite_ent,
        AnimatedSprite {
            frames: demo.animation.iter().copied().collect(),
            fps: demo.fps,
            ..default()
        },
    );
}

fn path2d_demo_plugin(session: &mut Session) {
    session
        .install_plugin(DefaultPlugin)
        .add_startup_system(path2d_demo_startup)
        .add_system_to_stage(Update, back_to_menu_ui);
}

fn path2d_demo_startup(
    meta: Root<GameMeta>,
    mut entities: ResMut<Entities>,
    mut transforms: CompMut<Transform>,
    mut cameras: CompMut<Camera>,
    mut path2ds: CompMut<Path2d>,
) {
    spawn_default_camera(&mut entities, &mut transforms, &mut cameras);

    let ent = entities.create();
    transforms.insert(ent, default());
    const SIZE: f32 = 40.;
    path2ds.insert(
        ent,
        Path2d {
            color: meta.path2d_color,
            points: vec![
                vec2(-SIZE, 0.),
                vec2(0., SIZE),
                vec2(SIZE, 0.),
                vec2(-SIZE, 0.),
            ],
            thickness: 2.0,
            ..default()
        },
    );
}

/// Simple UI system that shows a button at the bottom of the screen to delete the current session
/// and  go back to the main menu.
fn back_to_menu_ui(
    egui_ctx: ResMut<EguiCtx>,
    mut sessions: ResMut<Sessions>,
    mut session_options: ResMut<SessionOptions>,
    localization: Localization<GameMeta>,
) {
    egui::CentralPanel::default()
        .frame(egui::Frame::none())
        .show(&egui_ctx, |ui| {
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                ui.add_space(20.0);
                if ui.button(localization.get("back-to-menu")).clicked() {
                    session_options.delete = true;
                    sessions.create("menu").install_plugin(menu_plugin);
                }
            });
        });
}
