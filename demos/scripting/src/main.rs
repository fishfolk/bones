use std::sync::Arc;

use bones_bevy_renderer::BonesBevyRenderer;
use bones_framework::prelude::*;

#[derive(HasSchema, Default, Clone)]
#[type_data(metadata_asset("game"))]
#[repr(C)]
struct GameMeta {
    plugins: SVec<Handle<LuaPlugin>>,
    data: Handle<SchemaBox>,
}

fn main() {
    let mut game = Game::new();
    game.install_plugin(DefaultGamePlugin);
    game.shared_resource_mut::<AssetServer>()
        .unwrap()
        .register_default_assets();
    GameMeta::register_schema();

    game.sessions
        .create("launch")
        .add_startup_system(launch_game_session);

    let mut renderer = BonesBevyRenderer::new(game);
    renderer.app_namespace = (
        "org".into(),
        "fishfolk".into(),
        "bones.demo_scripting".into(),
    );
    renderer.app().run();
}

fn launch_game_session(
    meta: Root<GameMeta>,
    mut sessions: ResMut<Sessions>,
    mut session_ops: ResMut<SessionOptions>,
) {
    session_ops.delete = true;
    let game_session = sessions.create("game");
    game_session
        .install_plugin(DefaultSessionPlugin)
        // Install the plugin that will load our lua plugins and run them in the game session
        .install_plugin(LuaPluginLoaderSessionPlugin(
            // Tell it to install the lua plugins specified in our game meta
            Arc::new(meta.plugins.iter().copied().collect()),
        ))
        .add_startup_system(game_startup);
}

fn game_startup(
    mut entities: ResMut<Entities>,
    mut transforms: CompMut<Transform>,
    mut cameras: CompMut<Camera>,
) {
    spawn_default_camera(&mut entities, &mut transforms, &mut cameras);
}
