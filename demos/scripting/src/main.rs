use bones_bevy_renderer::BonesBevyRenderer;
use bones_framework::prelude::*;

#[derive(HasSchema, Default, Clone)]
#[type_data(metadata_asset("game"))]
#[repr(C)]
struct GameMeta {
    plugins: SVec<Handle<LuaPlugin>>,
    version: u32,
    sprite: Handle<Image>,
    info: Handle<GameInfoMeta>,
}

#[derive(HasSchema, Default, Clone)]
#[repr(C)]
#[type_data(metadata_asset("info"))]
struct GameInfoMeta {
    name: String,
    gravity: f32,
}

#[derive(HasSchema, Default, Clone)]
#[repr(C)]
struct DemoData {
    name: String,
    age: f32,
    favorite_things: SVec<String>,
    attributes: SMap<String, f32>,
    best_friend: Maybe<String>,
    state: DemoState,
}

#[derive(HasSchema, Default, Clone)]
#[repr(C, u8)]
pub enum DemoState {
    #[default]
    Ready,
    Thinking(f32),
    Finished {
        score: u32,
    },
}

fn main() {
    let mut game = Game::new();
    game.install_plugin(DefaultGamePlugin);
    GameMeta::register_schema();
    DemoData::register_schema();

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
            meta.plugins.iter().copied().collect(),
        ))
        .add_startup_system(game_startup);

    game_session.world.insert_resource(DemoData {
        name: "default name".into(),
        age: 10.0,
        favorite_things: ["candy".into(), "rain".into()].into_iter().collect(),
        attributes: [("coolness".into(), 50.0), ("friendliness".into(), 10.57)]
            .into_iter()
            .collect(),
        best_friend: Some("Jane".into()).into(),
        state: DemoState::Thinking(20.),
    });
}

fn game_startup(
    mut entities: ResMut<Entities>,
    mut transforms: CompMut<Transform>,
    mut cameras: CompMut<Camera>,
) {
    spawn_default_camera(&mut entities, &mut transforms, &mut cameras);
}
