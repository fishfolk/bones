use bones_bevy_renderer::BonesBevyRenderer;
use bones_framework::prelude::*;

fn main() {
    // Create a bones bevy renderer and pass it our game and input system.
    BonesBevyRenderer::new(game_init(), input::system)
        .app()
        .run()
}

/// Our game input resource. This is returned by the [`input::system`].
#[derive(HasSchema, Default, Clone, Debug)]
#[repr(C)]
pub struct GameInput {
    player1: PlayerInput,
    player2: PlayerInput,
}

/// A player's individual input.
#[derive(HasSchema, Default, Clone, Debug)]
#[repr(C)]
pub struct PlayerInput {
    movement: Vec2,
    jump: bool,
    punch: bool,
}

/// A separate module for the input system, so we can scope include of the Bevy prelude to one
/// module.
mod input {
    use super::{GameInput, PlayerInput};
    use bones_bevy_renderer::bevy::prelude::*;

    /// The input system is a _bevy_ system, not a bones system, that gets user input from the Bevy
    /// world, and returns our [`GameInput`].
    pub fn system(key_input: Res<Input<KeyCode>>) -> GameInput {
        let mut p1 = PlayerInput::default();
        if key_input.pressed(KeyCode::A) {
            p1.movement += Vec2::NEG_X;
        }
        if key_input.pressed(KeyCode::D) {
            p1.movement += Vec2::X;
        }
        if key_input.just_pressed(KeyCode::W) {
            p1.jump = true;
        }
        if key_input.just_pressed(KeyCode::S) {
            p1.punch = true;
        }

        let mut p2 = PlayerInput::default();
        if key_input.pressed(KeyCode::Numpad4) {
            p2.movement += Vec2::NEG_X;
        }
        if key_input.pressed(KeyCode::Numpad6) {
            p2.movement += Vec2::X;
        }
        if key_input.just_pressed(KeyCode::Numpad8) {
            p2.jump = true;
        }
        if key_input.just_pressed(KeyCode::Numpad5) {
            p2.punch = true;
        }

        GameInput {
            player1: p1,
            player2: p2,
        }
    }
}

// Initialize the game.
pub fn game_init() -> Game {
    // Create an empty game
    let mut game = Game::new();

    // Create our menu session
    let menu_session = game.sessions.create("menu");

    // Install our menu plugin into the menu session
    menu_session.install_plugin(menu::plugin);

    game
}

mod menu {
    use super::*;

    pub fn plugin(session: &mut Session) {
        session
            .stages
            .add_system_to_stage(CoreStage::Update, menu_system);
    }

    fn menu_system(input: Res<GameInput>, egui_ctx: ResMut<EguiCtx>) {
        egui::CentralPanel::default().show(&egui_ctx, |ui| {
            ui.label(&format!("{:?}", *input));
        });
    }
}
