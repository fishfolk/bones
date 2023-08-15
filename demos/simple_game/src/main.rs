#![allow(clippy::too_many_arguments)]

use bones_bevy_renderer::BonesBevyRenderer;
use bones_framework::prelude::*;

/// Create our root asset type.
///
/// The path to our root asset file is specified in `assets/pack.yaml`.
#[derive(HasSchema, Default, Clone)]
#[repr(C)]
// Allow asset to be loaded from "game.yaml" assets.
#[type_data(metadata_asset("game"))]
struct GameMeta {
    /// The title shown on the menu.
    title: String,
    /// Character information that will be loaded from a separate asset file.
    character: Handle<CharacterMeta>,
    /// A sprite that will be shown on the menu
    menu_sprite: Handle<Image>,
}

/// Character information.
#[derive(HasSchema, Default, Clone)]
#[repr(C)]
#[type_data(metadata_asset("character"))]
struct CharacterMeta {
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

fn main() {
    // Create a bones bevy renderer from our bones game
    BonesBevyRenderer::new(create_game())
        // Get a bevy app for running our game
        .app()
        // Run the bevy app
        .run()
}

// Initialize the game.
pub fn create_game() -> Game {
    // Create an empty game
    let mut game = Game::new();

    // Configure the asset server
    game.asset_server()
        // Register the default asset types
        .register_default_assets()
        // Register our custom asset types
        .register_asset::<GameMeta>()
        .register_asset::<CharacterMeta>();

    // Create our menu session
    game.sessions.create("menu").install_plugin(menu_plugin);

    game
}

/// Menu plugin
pub fn menu_plugin(session: &mut Session) {
    // Register our menu system
    session
        // Install the default plugins for this session
        .install_plugin(DefaultPlugin)
        // And add our systems.
        .stages
        .add_system_to_stage(CoreStage::Update, main_menu)
        .add_startup_system(menu_startup);
}

/// Setup the main menu
fn menu_startup(
    mut egui_settings: ResMutInit<EguiSettings>,
    mut entities: ResMut<Entities>,
    mut transforms: CompMut<Transform>,
    mut sprites: CompMut<Sprite>,
    mut cameras: CompMut<Camera>,
    mut clear_color: ResMutInit<ClearColor>,
    meta: Root<GameMeta>,
) {
    // Set the clear color
    **clear_color = Color::BLACK;

    egui_settings.scale = 2.0;

    // Spawn the camera
    let camera_ent = entities.create();
    transforms.insert(
        camera_ent,
        Transform::from_translation(Vec3::new(0., 0., 100.)),
    );
    cameras.insert(camera_ent, default());

    // Spawn a sprite for the menu
    let ent = entities.create();
    transforms.insert(ent, default());
    sprites.insert(
        ent,
        Sprite {
            image: meta.menu_sprite,
            ..default()
        },
    );
}

/// Our main menu system.
fn main_menu(
    egui_ctx: ResMut<EguiCtx>,
    mut sessions: ResMut<Sessions>,
    mut session_options: ResMut<SessionOptions>,
    // Get the root asset with the `Root` system param.
    game_meta: Root<GameMeta>,
) {
    // Render the menu.
    egui::CentralPanel::default()
        .frame(egui::Frame::none())
        .show(&egui_ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                ui.heading(&game_meta.title);
                ui.add_space(20.0);
                if ui.button("Start Game").clicked() {
                    // Delete the menu session
                    session_options.delete = true;

                    // Create a session for the match
                    sessions
                        .create("match")
                        .install_plugin(DefaultPlugin)
                        .install_plugin(match_plugin);
                }
            });
        });
}

fn match_plugin(session: &mut Session) {
    session
        .install_plugin(DefaultPlugin)
        .stages
        .add_startup_system(match_startup)
        .add_system_to_stage(CoreStage::Update, match_ui);
}

/// System to startup the match.
fn match_startup(
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
    let camera_ent = entities.create();
    transforms.insert(
        camera_ent,
        Transform::from_translation(Vec3::new(0., 0., 100.)),
    );
    cameras.insert(
        camera_ent,
        Camera {
            height: 200.0,
            ..default()
        },
    );

    // Get the character metadata
    let character = assets.get(&meta.character);

    // Spawn the character sprite.
    let sprite_ent = entities.create();
    transforms.insert(sprite_ent, default());
    atlas_sprites.insert(
        sprite_ent,
        AtlasSprite {
            atlas: character.atlas,
            ..default()
        },
    );
    animated_sprites.insert(
        sprite_ent,
        AnimatedSprite {
            frames: character.animation.iter().copied().collect(),
            fps: character.fps,
            ..default()
        },
    );
}

fn match_ui(
    egui_ctx: ResMut<EguiCtx>,
    mut sessions: ResMut<Sessions>,
    mut session_options: ResMut<SessionOptions>,
) {
    egui::CentralPanel::default()
        .frame(egui::Frame::none())
        .show(&egui_ctx, |ui| {
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                ui.add_space(20.0);
                if ui.button("Back to Menu").clicked() {
                    session_options.delete = true;
                    sessions.create("menu").install_plugin(menu_plugin);
                }
            });
        });
}
