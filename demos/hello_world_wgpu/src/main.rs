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
    sprite2: Handle<Image>,
    atlas: Handle<Atlas>,
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
        .install_plugin(DefaultSessionPlugin)
        // Add our menu system to the update stage
        //.add_system_to_stage(Update, menu_system)
        .add_system_to_stage(Update, test);

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
    mut atlas_sprites: CompMut<AtlasSprite>,
    mut transforms: CompMut<Transform>,
    mut cameras: CompMut<Camera>,
    meta: Root<GameMeta>,
) {
    spawn_default_camera(&mut entities, &mut transforms, &mut cameras);
    let second_camera = Camera {
        priority: 1,
        viewport: Maybe::Set(Viewport {
            position: UVec2::new(10, 10),
            size: UVec2::new(100, 100),
            ..Default::default()
        }),
        background_color: Set(Color::ORANGE),
        ..Default::default()
    };
    let camera_ent = entities.create();
    transforms.insert(camera_ent, default());
    cameras.insert(camera_ent, second_camera);

    let sprite_ent = entities.create();
    transforms.insert(sprite_ent, default());
    sprites.insert(
        sprite_ent,
        Sprite {
            image: meta.sprite,
            ..default()
        },
    );
    let sprite_ent = entities.create();
    transforms.insert(
        sprite_ent,
        Transform::from_translation(Vec3::new(0.5, 0.5, 0.0)),
    );
    sprites.insert(
        sprite_ent,
        Sprite {
            image: meta.sprite2,
            flip_x: true,
            ..default()
        },
    );

    let atlas_ent = entities.create();
    transforms.insert(
        atlas_ent,
        Transform::from_translation(Vec3::new(1.5, 1.5, 0.0)),
    );
    atlas_sprites.insert(
        atlas_ent,
        AtlasSprite {
            atlas: meta.atlas,
            index: 0,
            flip_x: false,
            flip_y: false,
            ..default()
        },
    );
}

fn move_sprite(
    entities: Res<Entities>,
    mut atlases: CompMut<AtlasSprite>,
    mut sprites: CompMut<Sprite>,
    mut transforms: CompMut<Transform>,
    input: Res<KeyboardInputs>,
    input_mouse: Res<MouseInputs>,
    gamepad: Res<GamepadInputs>,
    camera: Comp<Camera>,
    //mouse_position: Res<MouseScreenPosition>
) {
    let mut left = false;
    let mut right = false;
    let mut up = false;
    let mut down = false;
    let mut rotate_left = false;
    let mut rotate_right = false;
    let mut idx_up = false;
    let mut idx_down = false;
    let mut flip_x = false;
    let mut flip_y = false;

    for input in &input.key_events {
        match input.key_code {
            Set(KeyCode::D) => right = true,
            Set(KeyCode::A) => left = true,
            Set(KeyCode::W) => up = true,
            Set(KeyCode::S) => down = true,
            Set(KeyCode::Q) => rotate_left = true,
            Set(KeyCode::E) => rotate_right = true,
            Set(KeyCode::P) => idx_up = true,
            Set(KeyCode::O) => idx_down = true,
            Set(KeyCode::L) => flip_y = true,
            Set(KeyCode::K) => flip_x = true,
            _ => (),
        }
    }

    for (_, atlas) in entities.iter_with(&mut atlases) {
        if idx_up {
            atlas.index += 1;
        }
        if idx_down {
            atlas.index -= 1;
        }
        if flip_x {
            atlas.flip_x = !atlas.flip_x;
        }
        if flip_y {
            atlas.flip_y = !atlas.flip_y;
        }
    }

    for (_, sprite) in entities.iter_with(&mut sprites) {
        if flip_x {
            sprite.flip_x = !sprite.flip_x;
        }
        if flip_y {
            sprite.flip_y = !sprite.flip_y;
        }
    }

    let mut i = 0;
    for (_ent, (_sprite, transform)) in entities.iter_with((&camera, &mut transforms)) {
        if i == 0 {
            //mouse and keyboard
            if left {
                transform.translation.x -= 0.1;
            }
            if right {
                transform.translation.x += 0.1;
            }
            if up {
                transform.translation.y += 0.1;
            }
            if down {
                transform.translation.y -= 0.1;
            }
            if rotate_left {
                transform.rotation.z -= 0.1;
            }
            if rotate_right {
                transform.rotation.z += 0.1;
            }

            for event in &input_mouse.wheel_events {
                transform.scale += event.movement.y * 0.1;
            }
            i += 1;
            continue;
        }
        //gamepad
        for event in gamepad.gamepad_events.iter() {
            match event {
                GamepadEvent::Axis(axis) => match axis.axis {
                    GamepadAxis::LeftStickX => {
                        transform.translation.x += axis.value * 0.1;
                    }
                    GamepadAxis::LeftStickY => {
                        transform.translation.y += axis.value * 0.1;
                    }
                    GamepadAxis::RightStickX => {
                        transform.translation.x += axis.value * 0.1;
                    }
                    GamepadAxis::RightStickY => {
                        transform.translation.y += axis.value * 0.1;
                    }
                    _ => (),
                },
                GamepadEvent::Button(button) => {
                    if button.button == GamepadButton::LeftTrigger {
                        transform.rotation.z += 0.1;
                    }
                    if button.button == GamepadButton::RightTrigger {
                        transform.rotation.z -= 0.1;
                    }
                    if button.button == GamepadButton::LeftTrigger2 {
                        transform.scale -= button.value * 0.05;
                    }
                    if button.button == GamepadButton::RightTrigger2 {
                        transform.scale += button.value * 0.05;
                    }
                }
                _ => (),
            }
        }
    }
}

/// System to render the home menu.
fn menu_system(ctx: Res<EguiCtx>) {
    egui::CentralPanel::default().show(&ctx, |ui| {
        ui.label("Hello World");
    });
}

fn test(ctx: Res<EguiCtx>) {
    egui::Window::new("winit + egui + wgpu + bones :0")
        .resizable(true)
        .vscroll(true)
        .default_open(false)
        .show(&ctx, |ui| {
            ui.label("Label!");

            if ui.button("Button!").clicked() {
                println!("boom!")
            }

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("-").clicked() {
                    println!("Sei la");
                }
                if ui.button("+").clicked() {
                    println!("Sei la");
                }
            });
        });
}
