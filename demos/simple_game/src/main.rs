use bones_lib::prelude::*;
use glam::*;

#[derive(HasSchema, Clone, Debug, Default)]
#[repr(C)]
struct GameMeta;

#[derive(HasSchema, Default, Clone, Debug)]
#[repr(C)]
pub struct Input {
    player1: PlayerInput,
    player2: PlayerInput,
    player3: PlayerInput,
    player4: PlayerInput,
}

#[derive(HasSchema, Default, Clone, Debug)]
#[repr(C)]
pub struct PlayerInput {
    movement: Vec2,
    jump: bool,
    punch: bool,
}

fn main() {
    // Create an empty game
    let mut game = Game::new();

    // Create our menu session
    let menu_session = game.sessions.create("menu");

    // Install our menu plugin into the menu session
    menu_session.install_plugin(menu::plugin);
}

mod menu {
    use super::*;

    pub fn plugin(session: &mut Session) {
        session
            .stages
            .add_system_to_stage(CoreStage::Update, menu_system);
    }

    fn menu_system() {}
}
