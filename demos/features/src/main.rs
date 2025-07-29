#![allow(clippy::too_many_arguments)]

use bones_wgpu_renderer::BonesWgpuRenderer;
use bones_framework::prelude::*;

/// Create our root asset type.
///
/// The path to our root asset file is specified in `assets/pack.yaml`.
#[derive(HasSchema, Default, Clone)]
#[repr(C)]
// Allow asset to be loaded from "game.yaml" assets.
#[type_data(metadata_asset("game"))]
struct GameMeta {
    /// A lua script that will be run every frame on the menu.
    menu_script: Handle<LuaScript>,
    /// The image displayed on the menu.
    menu_image: Handle<Image>,
    /// The image for the sprite demo
    sprite_demo: Handle<Image>,
    /// Character information that will be loaded from a separate asset file.
    atlas_demo: Handle<AtlasDemoMeta>,
    /// The tilemap demo metadata.
    tilemap_demo: Handle<TilemapDemoMeta>,
    /// Audio track for the audio demo.
    audio_demo: Handle<AudioSource>,
    /// The color the debug lines in the debug line demo.
    path2d_color: Color,
    /// Localization asset
    localization: Handle<LocalizationAsset>,
    /// The font to use for the demo title.
    title_font: FontMeta,
    /// The list of font files to load for the UI.
    fonts: SVec<Handle<Font>>,
    /// The border to use the for main menu.
    menu_border: BorderImageMeta,
    /// The style to use for buttons.
    button_style: ButtonThemeMeta,
}

/// Atlas information.
#[derive(HasSchema, Default, Clone)]
#[repr(C)]
#[type_data(metadata_asset("atlas-demo"))]
struct AtlasDemoMeta {
    /// The size of the camera.
    camera_size: CameraSize,
    /// The sprite atlas for the player.
    pub atlas: Handle<Atlas>,
    /// The frames-per-second of the animation.
    pub fps: f32,
    /// The frames of the animation.
    ///
    /// Note: We use an [`SVec`] here because it implements [`HasSchema`], allowing it to be loaded
    /// in a metadata asset.
    pub animation: SVec<u32>,
}

/// Tilemap info.
#[derive(HasSchema, Default, Clone)]
#[repr(C)]
#[type_data(metadata_asset("tilemap"))]
struct TilemapDemoMeta {
    /// The atlas that will be used for the tilemap.
    pub atlas: Handle<Atlas>,
    /// The size of the tile map in tiles.
    pub map_size: UVec2,
    /// The information about each tile in the tilemap.
    pub tiles: SVec<TileMeta>,
}

/// Tile info.
#[derive(HasSchema, Default, Clone)]
#[repr(C)]
struct TileMeta {
    /// The tile position.
    pos: UVec2,
    /// The index of the tile in the atlas.
    idx: u32,
}

/// Struct containing data that will be persisted with the storage API.
#[derive(HasSchema, Default, Clone)]
#[repr(C)]
struct PersistedTextData(String);

fn main() {
    // Setup logging
    setup_logs!();

    // Register persistent data's schema so that it can be loaded by the storage loader.
    PersistedTextData::register_schema();

    // Create a bones bevy renderer from our bones game
    let mut renderer = BonesWgpuRenderer::new(create_game());
    // Set the app namespace which will be used by the renderer to decide where to put
    // persistent storage files.
    renderer.app_namespace = (
        "org".into(),
        "fishfolk".into(),
        "bones.demo_features".into(),
    );
    // Get a bevy app for running our game
    renderer
        //.app()
        // We can add our own bevy plugins now
        //.add_plugins(LogDiagnosticsPlugin::default())
        // And run the bevy app
        .run()
}

// Initialize the game.
pub fn create_game() -> Game {
    // Create an empty game
    let mut game = Game::new();

    // Configure the asset server
    game.install_plugin(DefaultGamePlugin)
        .init_shared_resource::<AssetServer>()
        // Register the default asset types
        .register_default_assets();

    // Register our custom asset types
    GameMeta::register_schema();
    AtlasDemoMeta::register_schema();
    TilemapDemoMeta::register_schema();

    // Create our menu session
    game.sessions.create_with("menu", menu_plugin);

    game
}

/// Resource containing data that we will access from our menu lua script.
#[derive(HasSchema, Default, Clone)]
#[repr(C)]
struct MenuData {
    /// The index of the frame that we are on.
    pub frame: u32,
}

/// Menu plugin
pub fn menu_plugin(session: &mut SessionBuilder) {
    // Register our menu system
    session
        // Install the bones_framework default plugins for this session
        .install_plugin(DefaultSessionPlugin)
        // Initialize our menu data resource
        .init_resource::<MenuData>();

    // And add our systems.
    session
        .add_system_to_stage(Update, menu_system)
        .add_startup_system(menu_startup);
}

/// Setup the main menu.
fn menu_startup(
    mut egui_settings: ResMutInit<EguiSettings>,
    mut clear_color: ResMutInit<ClearColor>,
) {
    // Set the clear color
    **clear_color = Color::BLACK;
    // Set the egui scale
    egui_settings.scale = 2.0;
}

/// Our main menu system.
fn menu_system(
    meta: Root<GameMeta>,
    ctx: Res<EguiCtx>,
    mut sessions: ResMut<Sessions>,
    mut session_options: ResMut<SessionOptions>,
    mut exit_bones: Option<ResMut<ExitBones>>,
    // Get the localization field from our `GameMeta`
    localization: Localization<GameMeta>,
    world: &World,
    lua_engine: Res<LuaEngine>,
) {
    // Run our menu script.
    lua_engine.run_script_system(world, meta.menu_script);

    // Render the menu.
    egui::CentralPanel::default()
        .frame(egui::Frame::none())
        .show(&ctx, |ui| {
            BorderedFrame::new(&meta.menu_border).show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(20.0);
                    ui.label(meta.title_font.rich(localization.get("title")));
                    ui.add_space(20.0);

                    if BorderedButton::themed(&meta.button_style, localization.get("sprite-demo"))
                        .show(ui)
                        .clicked()
                    {
                        // Delete the menu world
                        session_options.delete = true;

                        // Create a session for the match
                        sessions.create_with("sprite_demo", sprite_demo_plugin);
                    }

                    if BorderedButton::themed(&meta.button_style, localization.get("atlas-demo"))
                        .show(ui)
                        .clicked()
                    {
                        // Delete the menu world
                        session_options.delete = true;

                        // Create a session for the match
                        sessions.create_with("atlas_demo", atlas_demo_plugin);
                    }

                    if BorderedButton::themed(&meta.button_style, localization.get("tilemap-demo"))
                        .show(ui)
                        .clicked()
                    {
                        // Delete the menu world
                        session_options.delete = true;

                        // Create a session for the match
                        sessions.create_with("tilemap_demo", tilemap_demo_plugin);
                    }

                    if BorderedButton::themed(&meta.button_style, localization.get("audio-demo"))
                        .show(ui)
                        .clicked()
                    {
                        // Delete the menu world
                        session_options.delete = true;

                        // Create a session for the match
                        sessions.create_with("audio_demo", audio_demo_plugin);
                    }

                    if BorderedButton::themed(&meta.button_style, localization.get("storage-demo"))
                        .show(ui)
                        .clicked()
                    {
                        // Delete the menu world
                        session_options.delete = true;

                        // Create a session for the match
                        sessions.create_with("storage_demo", storage_demo_plugin);
                    }

                    if BorderedButton::themed(&meta.button_style, localization.get("path2d-demo"))
                        .show(ui)
                        .clicked()
                    {
                        // Delete the menu world
                        session_options.delete = true;

                        // Create a session for the match
                        sessions.create_with("path2d_demo", path2d_demo_plugin);
                    }

                    if let Some(exit_bones) = &mut exit_bones {
                        if BorderedButton::themed(&meta.button_style, localization.get("quit"))
                            .show(ui)
                            .clicked()
                        {
                            ***exit_bones = true;
                        }
                    }

                    ui.add_space(10.0);

                    // We can use the `&World` parameter to access the world and run systems to act
                    // as egui widgets.
                    //
                    // This makes it easier to compose widgets that have differing access to the
                    // bones world.
                    world.run_system(demo_widget, ui);

                    ui.add_space(30.0);
                });
            })
        });
}

/// Plugin for running the sprite demo.
fn sprite_demo_plugin(session: &mut SessionBuilder) {
    session
        .install_plugin(DefaultSessionPlugin)
        .add_startup_system(sprite_demo_startup)
        .add_system_to_stage(Update, back_to_menu_ui)
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
            image: meta.sprite_demo,
            ..default()
        },
    );
}

fn move_sprite(
    entities: Res<Entities>,
    sprite: Comp<Sprite>,
    mut transforms: CompMut<Transform>,
    input: Res<KeyboardInputs>,
    ctx: Res<EguiCtx>,
) {
    egui::CentralPanel::default()
        .frame(egui::Frame::none())
        .show(&ctx, |ui| {
            ui.label("Press left and right arrow keys to move sprite");
        });

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

/// Plugin for running the tilemap demo.
fn tilemap_demo_plugin(session: &mut SessionBuilder) {
    session
        .install_plugin(DefaultSessionPlugin)
        .add_startup_system(tilemap_startup_system)
        .add_system_to_stage(Update, back_to_menu_ui);
}

/// System for starting up the tilemap demo.
fn tilemap_startup_system(
    mut entities: ResMut<Entities>,
    mut transforms: CompMut<Transform>,
    mut tile_layers: CompMut<TileLayer>,
    mut cameras: CompMut<Camera>,
    mut tiles: CompMut<Tile>,
    meta: Root<GameMeta>,
    assets: Res<AssetServer>,
) {
    spawn_default_camera(&mut entities, &mut transforms, &mut cameras);

    // Load our map and atlas info
    let map_info = assets.get(meta.tilemap_demo);
    let atlas = assets.get(map_info.atlas);

    // Create a new map layer
    let mut layer = TileLayer::new(map_info.map_size, atlas.tile_size, map_info.atlas);

    // Load the layer up with the tiles from our metadata
    for tile in &map_info.tiles {
        let tile_ent = entities.create();
        tiles.insert(
            tile_ent,
            Tile {
                idx: tile.idx,
                ..default()
            },
        );
        layer.set(tile.pos, Some(tile_ent))
    }

    // Spawn the layer
    let layer_ent = entities.create();
    tile_layers.insert(layer_ent, layer);
    transforms.insert(layer_ent, default());
}

/// Plugin for running the atlas demo.
fn atlas_demo_plugin(session: &mut SessionBuilder) {
    session
        .install_plugin(DefaultSessionPlugin)
        .add_startup_system(atlas_demo_startup)
        .add_system_to_stage(Update, back_to_menu_ui);
}

/// System to startup the atlas demo.
fn atlas_demo_startup(
    mut entities: ResMut<Entities>,
    mut transforms: CompMut<Transform>,
    mut cameras: CompMut<Camera>,
    mut atlas_sprites: CompMut<AtlasSprite>,
    mut animated_sprites: CompMut<AnimatedSprite>,
    mut clear_color: ResMutInit<ClearColor>,
    meta: Root<GameMeta>,
    assets: Res<AssetServer>,
) {
    // Set the clear color
    **clear_color = Color::GRAY;

    // Get the atlas metadata
    let demo = assets.get(meta.atlas_demo);

    // Spawn the camera
    let camera_ent = spawn_default_camera(&mut entities, &mut transforms, &mut cameras);
    cameras.get_mut(camera_ent).unwrap().size = demo.camera_size;

    // Spawn the character sprite.
    let sprite_ent = entities.create();
    transforms.insert(sprite_ent, default());
    atlas_sprites.insert(
        sprite_ent,
        AtlasSprite {
            atlas: demo.atlas,
            ..default()
        },
    );
    animated_sprites.insert(
        sprite_ent,
        AnimatedSprite {
            frames: demo.animation.iter().copied().collect(),
            fps: demo.fps,
            ..default()
        },
    );
}

fn audio_demo_plugin(session: &mut SessionBuilder) {
    session
        .install_plugin(DefaultSessionPlugin)
        .add_system_to_stage(Update, back_to_menu_ui)
        .add_system_to_stage(Update, audio_demo_ui);
}

fn audio_demo_ui(
    ctx: Res<EguiCtx>,
    localization: Localization<GameMeta>,
    mut audio: ResMut<AudioManager>,
    meta: Root<GameMeta>,
    assets: Res<AssetServer>,
) {
    egui::CentralPanel::default()
        .frame(egui::Frame::none())
        .show(&ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                if ui.button(localization.get("play-sound")).clicked() {
                    audio.play(&*assets.get(meta.audio_demo)).unwrap();
                }
            })
        });
}

fn storage_demo_plugin(session: &mut SessionBuilder) {
    session
        .install_plugin(DefaultSessionPlugin)
        .add_system_to_stage(Update, storage_demo_ui)
        .add_system_to_stage(Update, back_to_menu_ui);
}

fn storage_demo_ui(
    ctx: Res<EguiCtx>,
    mut storage: ResMut<Storage>,
    localization: Localization<GameMeta>,
) {
    egui::CentralPanel::default().show(&ctx, |ui| {
        ui.add_space(20.0);

        ui.vertical_centered(|ui| {
            ui.set_width(300.0);
            {
                let data = storage.get_or_insert_default_mut::<PersistedTextData>();
                egui::TextEdit::singleline(&mut data.0)
                    .hint_text(localization.get("persisted-text-box-content"))
                    .show(ui);
            }
            if ui.button(localization.get("save")).clicked() {
                storage.save()
            }
        });
    });
}

fn path2d_demo_plugin(session: &mut SessionBuilder) {
    session
        .install_plugin(DefaultSessionPlugin)
        .add_startup_system(path2d_demo_startup)
        .add_system_to_stage(Update, back_to_menu_ui);
}

fn path2d_demo_startup(
    meta: Root<GameMeta>,
    mut entities: ResMut<Entities>,
    mut transforms: CompMut<Transform>,
    mut cameras: CompMut<Camera>,
    mut path2ds: CompMut<Path2d>,
) {
    spawn_default_camera(&mut entities, &mut transforms, &mut cameras);

    let ent = entities.create();
    transforms.insert(ent, default());
    const SIZE: f32 = 40.;
    path2ds.insert(
        ent,
        Path2d {
            color: meta.path2d_color,
            points: vec![
                vec2(-SIZE, 0.),
                vec2(0., SIZE),
                vec2(SIZE, 0.),
                vec2(-SIZE, 0.),
            ],
            thickness: 2.0,
            ..default()
        },
    );
}

/// Simple UI system that shows a button at the bottom of the screen to delete the current session
/// and  go back to the main menu.
fn back_to_menu_ui(
    ctx: Res<EguiCtx>,
    mut sessions: ResMut<Sessions>,
    mut session_options: ResMut<SessionOptions>,
    localization: Localization<GameMeta>,
) {
    egui::TopBottomPanel::bottom("back-to-menu")
        .frame(egui::Frame::none())
        .show_separator_line(false)
        .show(&ctx, |ui| {
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                ui.add_space(20.0);
                if ui.button(localization.get("back-to-menu")).clicked() {
                    session_options.delete = true;
                    sessions.create_with("menu", menu_plugin);
                }
            });
        });
}

/// This is an example widget system.
fn demo_widget(
    // Widget systems must have an `In<&mut egui::Ui>` parameter as their first argument.
    mut ui: In<&mut egui::Ui>,
    // They can have any normal bones system parameters
    meta: Root<GameMeta>,
    egui_textures: Res<EguiTextures>,
    // And they may return an `egui::Response` or any other value.
) -> egui::Response {
    ui.label("Demo Widget");
    // When using a bones image in egui, we have to get it's corresponding egui texture
    // from the egui textures resource.
    ui.image(egui::load::SizedTexture::new(
        egui_textures.get(meta.menu_image),
        [50., 50.],
    ))
}
