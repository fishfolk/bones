pub mod lua;

use bones_lib::prelude::*;

/// The prelude.
pub mod prelude {
    pub use super::{lua::*, ScriptingGamePlugin};
    pub(crate) use bones_asset::prelude::*;
    pub(crate) use bones_lib::prelude::*;
}

/// Scripting plugin for the bones framework.
pub struct ScriptingGamePlugin {
    pub enable_lua: bool,
}

impl Default for ScriptingGamePlugin {
    fn default() -> Self {
        Self { enable_lua: true }
    }
}

impl GamePlugin for ScriptingGamePlugin {
    fn install(self, game: &mut Game) {
        if self.enable_lua {
            game.install_plugin(lua::lua_game_plugin);
        }
    }
}
