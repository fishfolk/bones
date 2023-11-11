use bones_bevy_renderer::BonesBevyRenderer;
use bones_framework::prelude::*;

#[derive(HasSchema, Default, Clone)]
#[type_data(metadata_asset("game"))]
#[repr(C)]
struct GameMeta {
    startup: Handle<LuaScript>,
    update: Handle<LuaScript>,
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

    let default_session = game.sessions.create("default");
    default_session
        .install_plugin(DefaultSessionPlugin)
        .add_startup_system(startup)
        .add_startup_system(startup_lua)
        .add_system_to_stage(Update, update_script);
    default_session.world.insert_resource(DemoData {
        name: "default name".into(),
        age: 10.0,
        favorite_things: ["candy".into(), "rain".into()].into_iter().collect(),
        attributes: [("coolness".into(), 50.0), ("friendliness".into(), 10.57)]
            .into_iter()
            .collect(),
        best_friend: Some("Jane".into()).into(),
        state: DemoState::Thinking(20.),
    });

    let mut renderer = BonesBevyRenderer::new(game);
    renderer.app_namespace = (
        "org".into(),
        "fishfolk".into(),
        "bones.demo_scripting".into(),
    );
    renderer.app().run();
}

fn startup(
    mut entities: ResMut<Entities>,
    mut transforms: CompMut<Transform>,
    mut cameras: CompMut<Camera>,
) {
    spawn_default_camera(&mut entities, &mut transforms, &mut cameras);
}

fn startup_lua(world: &World, lua_engine: Res<LuaEngine>, meta: Root<GameMeta>) {
    lua_engine.run_script_system(world, meta.startup);
}

fn update_script(world: &World, lua_engine: Res<LuaEngine>, meta: Root<GameMeta>) {
    lua_engine.run_script_system(world, meta.update);
}
