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
    character: Handle<CharacterMeta>,
}

#[derive(HasSchema, Default, Clone)]
#[repr(C)]
#[type_data(metadata_asset("character"))]
struct CharacterMeta {
    pub fps: f32,
    pub atlas: Handle<Atlas>,
    pub animation: SVec<u32>,
}

fn main() {
    // Create a bones bevy renderer
    BonesBevyRenderer::new(
        // Pass it our bones game
        game_init(),
        // Configure the asset server
        |asset_server| {
            // Register our game meta asset kind
            asset_server.register_asset::<GameMeta>();
            asset_server.register_asset::<CharacterMeta>();
        },
    )
    // Get a bevy app for running our game
    .app()
    // Run the bevy app
    .run()
}

// Initialize the game.
pub fn game_init() -> Game {
    // Create an empty game
    let mut game = Game::new();

    // Create our menu session
    let menu_session = game.sessions.create("menu");

    // Install our menu plugin into the menu session
    menu_session.install_plugin(menu_plugin);

    game
}

/// Menu plugin
pub fn menu_plugin(session: &mut Session) {
    // Register our menu system
    session
        .install_plugin(DefaultPlugins)
        .stages
        .add_system_to_stage(CoreStage::Update, menu_system)
        .add_system_to_stage(CoreStage::Update, init_system);
}

#[derive(HasSchema, Debug, Default, Clone, Deref, DerefMut)]
#[repr(C)]
struct HasInit(pub bool);

#[allow(clippy::too_many_arguments)]
fn init_system(
    mut has_init: ResMutInit<HasInit>,
    mut entities: ResMut<Entities>,
    mut transforms: CompMut<Transform>,
    mut cameras: CompMut<Camera>,
    mut atlas_sprites: CompMut<AtlasSprite>,
    mut animated_sprites: CompMut<AnimatedSprite>,
    mut clear_color: ResMutInit<ClearColor>,
    meta: Root<GameMeta>,
    assets: Res<AssetServer>,
) {
    if !**has_init {
        **has_init = true;

        // Set the clear color
        **clear_color = Color::BLACK;

        // Spawn the camera
        let camera_ent = entities.create();
        transforms.insert(
            camera_ent,
            Transform::from_translation(Vec3::new(0., 0., 100.)),
        );
        cameras.insert(
            camera_ent,
            Camera {
                height: 250.0,
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
}

/// Resource that stores whether or not we should say hello.
#[derive(HasSchema, Default, Clone, Debug, Deref, DerefMut)]
#[repr(C)]
struct ShowHello(pub bool);

/// Our main menu system.
fn menu_system(
    time: Res<Time>,
    mut hello: ResMutInit<ShowHello>,
    keyboard_input: Res<KeyboardInputs>,
    egui_ctx: ResMut<EguiCtx>,
    // Get the root asset with the `Root` system param.
    game_meta: Root<GameMeta>,
) {
    // Update the hello state based on keyboard events.
    for event in &keyboard_input.keys {
        if event.key_code == Some(KeyCode::Space) {
            if event.button_state == ButtonState::Pressed {
                **hello = true;
            } else if event.button_state == ButtonState::Released {
                **hello = false;
            }
        }
    }

    // Render the menu.
    egui::CentralPanel::default()
        .frame(egui::Frame::none())
        .show(&egui_ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                ui.heading(&game_meta.title);
                ui.add_space(20.0);
                ui.label(&format!("{:.0?}", time.elapsed()));
                ui.add_space(20.0);
                if **hello {
                    ui.label("Hello World!");
                } else {
                    ui.label("...");
                }
            });
        });
}
