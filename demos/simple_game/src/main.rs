use bones_bevy_renderer::BonesBevyRenderer;
use bones_framework::prelude::*;

fn main() {
    // Create a bones bevy renderer and pass it our game and input system.
    BonesBevyRenderer::new(game_init()).app().run()
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
    session
        .stages
        .add_system_to_stage(CoreStage::Update, menu_system);
}

#[derive(HasSchema, Default, Clone, Debug, Deref, DerefMut)]
#[repr(C)]
struct ShowHello(pub bool);

fn menu_system(
    mut hello: ResMut<ShowHello>,
    keyboard_input: Res<KeyboardInputs>,
    egui_ctx: ResMut<EguiCtx>,
) {
    for event in &keyboard_input.keys {
        if event.key_code == Some(KeyCode::Space) {
            if event.button_state == ButtonState::Pressed {
                **hello = true;
            } else if event.button_state == ButtonState::Released {
                **hello = false;
            }
        }
    }

    egui::CentralPanel::default().show(&egui_ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);
            if **hello {
                ui.label("Hello World!");
            } else {
                ui.label("...");
            }
        });
    });
}
