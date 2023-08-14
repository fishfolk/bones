use bones_bevy_renderer::BonesBevyRenderer;
use bones_framework::prelude::*;

/// Create our root asset type.
///
/// The path to our root asset file is specified in `assets/pack.yaml`.
#[derive(HasSchema, Default, Clone)]
#[repr(C)]
// Allow asset to be loaded from "game.yaml" assets.
#[type_data(metadata_asset("game"))]
struct GameMeta {
    /// The name of the game.
    name: String,
    /// The sprite to render
    sprite: Handle<Image>,
}

fn main() {
    // Create a bones bevy renderer
    BonesBevyRenderer::new(
        // Pass it our bones game
        game_init(),
        // Configure the asset server
        |asset_server| {
            // Register our game meta asset kind
            asset_server.register_asset::<GameMeta>();
        },
    )
    // Get a bevy app for running our game
    .app()
    // Run the bevy app
    .run()
}

// Initialize the game.
pub fn game_init() -> Game {
    // Create an empty game
    let mut game = Game::new();

    // Create our menu session
    let menu_session = game.sessions.create("menu");

    // Install our menu plugin into the menu session
    menu_session.install_plugin(menu_plugin);

    game
}

/// Menu plugin
pub fn menu_plugin(session: &mut Session) {
    // Register our menu system
    session
        .stages
        .add_system_to_stage(CoreStage::Update, menu_system)
        .add_system_to_stage(CoreStage::Update, move_sprite)
        .add_system_to_stage(CoreStage::Update, init_system);
}

/// Resource that stores whether or not we should say hello.
#[derive(HasSchema, Default, Clone, Debug, Deref, DerefMut)]
#[repr(C)]
struct ShowHello(pub bool);

#[derive(HasSchema, Debug, Default, Clone, Deref, DerefMut)]
#[repr(C)]
struct HasInit(pub bool);

#[derive(HasSchema, Default, Clone)]
#[repr(C)]
struct MySprite;

#[allow(clippy::too_many_arguments)]
fn init_system(
    mut has_init: ResMutInit<HasInit>,
    mut entities: ResMut<Entities>,
    mut transforms: CompMut<Transform>,
    mut cameras: CompMut<Camera>,
    mut sprites: CompMut<Sprite>,
    mut my_sprites: CompMut<MySprite>,
    mut clear_color: ResMutInit<ClearColor>,
    meta: Root<GameMeta>,
) {
    if !**has_init {
        **has_init = true;

        **clear_color = Color::BLACK;

        let camera_ent = entities.create();
        transforms.insert(
            camera_ent,
            Transform::from_translation(Vec3::new(0., 0., 100.)),
        );
        cameras.insert(camera_ent, default());

        let sprite_ent = entities.create();
        transforms.insert(sprite_ent, default());
        sprites.insert(
            sprite_ent,
            Sprite {
                image: meta.sprite,
                ..default()
            },
        );
        my_sprites.insert(sprite_ent, default());
    }
}

fn move_sprite(
    time: Res<Time>,
    entities: Res<Entities>,
    my_sprites: Comp<MySprite>,
    mut transforms: CompMut<Transform>,
) {
    for (_, (transform, _)) in entities.iter_with((&mut transforms, &my_sprites)) {
        transform.translation.x = time.elapsed_seconds().sin() * 200.0;
    }
}

/// Our main menu system.
fn menu_system(
    time: Res<Time>,
    mut hello: ResMutInit<ShowHello>,
    keyboard_input: Res<KeyboardInputs>,
    egui_ctx: ResMut<EguiCtx>,
    // Get the root asset with the `Root` system param.
    game_meta: Root<GameMeta>,
) {
    // Update the hello state based on keyboard events.
    for event in &keyboard_input.keys {
        if event.key_code == Some(KeyCode::Space) {
            if event.button_state == ButtonState::Pressed {
                **hello = true;
            } else if event.button_state == ButtonState::Released {
                **hello = false;
            }
        }
    }

    // Render the menu.
    egui::CentralPanel::default()
        .frame(egui::Frame::none())
        .show(&egui_ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                ui.heading(&game_meta.name);
                ui.add_space(20.0);
                ui.label(&format!("{:.0?}", time.elapsed()));
                ui.add_space(20.0);
                if **hello {
                    ui.label("Hello World!");
                } else {
                    ui.label("...");
                }
            });
        });
}
