use bones_bevy_renderer::BonesBevyRenderer;
use bones_framework::prelude::*;

#[derive(HasSchema, Default, Clone)]
#[type_data(metadata_asset("game"))]
#[repr(C)]
struct GameMeta {
    update: Handle<LuaScript>,
}

#[derive(HasSchema, Default, Clone)]
#[repr(C)]
struct DemoData {
    name: String,
    age: f32,
}

fn main() {
    let mut game = Game::new();
    game.install_plugin(DefaultGamePlugin);
    GameMeta::schema();
    DemoData::schema();

    let default_session = game.sessions.create("default");
    default_session
        .install_plugin(DefaultSessionPlugin)
        .add_system_to_stage(Update, update_script);
    default_session.world.insert_resource(DemoData {
        name: "default name".into(),
        age: 10.0,
    });

    BonesBevyRenderer::new(game).app().run();
}

fn update_script(world: &World, lua_engine: Res<LuaEngine>, meta: Root<GameMeta>) {
    lua_engine.run_script_system(world, meta.update);
}
