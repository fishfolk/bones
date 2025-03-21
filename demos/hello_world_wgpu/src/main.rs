use bones_framework::prelude::*;
use bones_wgpu_renderer::BonesWgpuRenderer;

//
// NOTE: You must run this example from within the `demos/hello_world_wgpu` folder. Also, be sure to
// look at the `demos/hello_world_wgpu` folder to see the asset files for this example.
//

/// Create our "root" asset type.
#[derive(HasSchema, Clone, Default)]
#[repr(C)]
// We must mark this as a metadata asset, and we set the type to "game".
//
// This means that any files with names like `game.yaml`, `game.yml`, `game.json`, `name.game.yaml`,
// etc. will be loaded as a `GameMeta` asset.
#[type_data(metadata_asset("game"))]
struct GameMeta {
    title: String,
    sprite: Handle<Image>,
}

fn main() {
    // Setup logging
    setup_logs!();

    // First create bones game.
    let mut game = Game::new();

    game
        // We initialize the asset server.
        .init_shared_resource::<AssetServer>();

    // We must register all of our asset types before they can be loaded by the asset server. This
    // may be done by calling schema() on each of our types, to register them with the schema
    // registry.
    GameMeta::register_schema();

    // Create a new session for the game world. Each session is it's own bones world with it's own
    // plugins, systems, and entities.
    let world_session = game
        .sessions
        .create("world")
        .install_plugin(sprite_demo_plugin);
    world_session
        // Install the default bones_framework plugin for this session
        .install_plugin(DefaultSessionPlugin);
    // Add our menu system to the update stage
    //.add_system_to_stage(Update, menu_system);

    BonesWgpuRenderer::new(game).run();
}

/// Plugin for running the sprite demo.
fn sprite_demo_plugin(session: &mut Session) {
    session
        .install_plugin(DefaultSessionPlugin)
        .add_startup_system(sprite_demo_startup)
        .add_system_to_stage(Update, move_sprite);
}

/// System that spawns the sprite demo.
fn sprite_demo_startup(
    mut entities: ResMut<Entities>,
    mut sprites: CompMut<Sprite>,
    mut transforms: CompMut<Transform>,
    mut cameras: CompMut<Camera>,
    meta: Root<GameMeta>,
) {
    spawn_default_camera(&mut entities, &mut transforms, &mut cameras);

    let sprite_ent = entities.create();
    transforms.insert(sprite_ent, default());
    sprites.insert(
        sprite_ent,
        Sprite {
            image: meta.sprite,
            ..default()
        },
    );
}

fn move_sprite(
    entities: Res<Entities>,
    sprite: Comp<Sprite>,
    mut transforms: CompMut<Transform>,
    input: Res<KeyboardInputs>,
) {
    let mut left = false;
    let mut right = false;

    for input in &input.key_events {
        match input.key_code {
            Set(KeyCode::Right) => right = true,
            Set(KeyCode::Left) => left = true,
            _ => (),
        }
    }

    for (_ent, (_sprite, transform)) in entities.iter_with((&sprite, &mut transforms)) {
        if left {
            transform.translation.x -= 2.0;
        }
        if right {
            transform.translation.x += 2.0;
        }
    }
}

/// System to render the home menu.
fn _menu_system(ctx: Res<EguiCtx>) {
    egui::CentralPanel::default().show(&ctx, |ui| {
        ui.label("Hello World");
    });
}
