use bones_bevy_renderer::BonesBevyRenderer;
use bones_framework::prelude::*;

/// Create our game metadata asset.
#[derive(HasSchema, Default, Clone)]
#[repr(C)]
// Allow asset to be loaded from "game.yaml" assets.
#[type_data(metadata_asset("game"))]
struct GameMeta {
    /// The name of the game.
    name: String,
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
        .stages
        .add_system_to_stage(CoreStage::Update, menu_system);
}

/// Resource that stores whether or not we should say hello.
#[derive(HasSchema, Default, Clone, Debug, Deref, DerefMut)]
#[repr(C)]
struct ShowHello(pub bool);

/// Our main menu system.
fn menu_system(
    mut hello: ResMutInit<ShowHello>,
    keyboard_input: Res<KeyboardInputs>,
    egui_ctx: ResMut<EguiCtx>,
    asset_server: Res<AssetServer>,
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

    // Get the root asset data from the asset server
    let game_meta: &GameMeta = asset_server.root();

    // Render the menu.
    egui::CentralPanel::default().show(&egui_ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);
            ui.heading(&game_meta.name);
            ui.add_space(20.0);
            if **hello {
                ui.label("Hello World!");
            } else {
                ui.label("...");
            }
        });
    });
}
