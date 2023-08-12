use bones_framework::prelude::*;

fn main() {
    let game = game::init();
}

mod game {
    use super::*;

    #[derive(HasSchema, Default, Clone, Debug)]
    #[repr(C)]
    pub struct Input {
        player1: PlayerInput,
        player2: PlayerInput,
    }

    #[derive(HasSchema, Default, Clone, Debug)]
    #[repr(C)]
    pub struct PlayerInput {
        movement: Vec2,
        jump: bool,
        punch: bool,
    }

    // Initialize the game.
    pub fn init() -> Game {
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

        fn menu_system() {}
    }
}
