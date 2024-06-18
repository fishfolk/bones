//! Bevy plugin for rendering Bones framework games.

#![warn(missing_docs)]
// This cfg_attr is needed because `rustdoc::all` includes lints not supported on stable
#![cfg_attr(doc, allow(unknown_lints))]
#![deny(rustdoc::all)]

use std::path::PathBuf;

pub use bevy;

use bevy::{
    input::InputSystem,
    prelude::*,
    render::RenderApp,
    sprite::{extract_sprites, SpriteSystem},
    tasks::IoTaskPool,
    utils::{HashMap, Instant},
    window::WindowMode,
};
use glam::*;

use bones_framework::prelude::{self as bones, EguiCtx, SchemaBox, SCHEMA_REGISTRY};
use prelude::convert::{IntoBevy, IntoBones};
use serde::{de::Visitor, Deserialize, Serialize};

/// The prelude
pub mod prelude {
    pub use crate::*;
}

mod convert;
mod debug;
mod storage;

mod input;
use input::*;
mod lyon;
use lyon::*;
mod render;
use render::*;
mod ui;
use ui::*;

/// Marker component for entities that are rendered in Bevy for bones.
#[derive(Component)]
pub struct BevyBonesEntity;

/// Renderer for [`bones_framework`] [`Game`][bones::Game]s using Bevy.
pub struct BonesBevyRenderer {
    /// Skip the default loading screen and run the bones game immediately, so that you can
    /// implement your own loading screen.
    pub custom_load_progress: Option<LoadingFunction>,
    /// Whether or not to use nearest-neighbor sampling for textures.
    pub pixel_art: bool,
    /// The bones game to run.
    pub game: bones::Game,
    /// The version of the game, used for the asset loader.
    pub game_version: bones::Version,
    /// The (qualifier, organization, application) that will be used to pick a persistent storage
    /// location for the game.
    ///
    /// For example: `("org", "fishfolk", "jumpy")`
    pub app_namespace: (String, String, String),
    /// The path to load assets from.
    pub asset_dir: PathBuf,
    /// The path to load asset packs from.
    pub packs_dir: PathBuf,
}

/// Bevy resource containing the [`bones::Game`]
#[derive(Resource, Deref, DerefMut)]
pub struct BonesGame(pub bones::Game);
impl BonesGame {
    fn asset_server(&self) -> Option<bones::Ref<bones::AssetServer>> {
        self.0.shared_resource()
    }
}

#[derive(Resource, Deref, DerefMut)]
struct LoadingContext(pub Option<LoadingFunction>);
type LoadingFunction =
    Box<dyn FnMut(&bones::AssetServer, &bevy_egui::egui::Context) + Sync + Send + 'static>;

impl BonesBevyRenderer {
    // TODO: Create a better builder pattern struct for `BonesBevyRenderer`.
    // We want to use a nice builder-pattern struct for `BonesBevyRenderer` so that it is easier
    // to set options like the `pixel_art` flag or the `game_version`.
    /// Create a new [`BonesBevyRenderer`] for the provided game.
    pub fn new(game: bones::Game) -> Self {
        BonesBevyRenderer {
            pixel_art: true,
            custom_load_progress: None,
            game,
            game_version: bones::Version::new(0, 1, 0),
            app_namespace: ("org".into(), "fishfolk".into(), "bones_demo_game".into()),
            asset_dir: PathBuf::from("assets"),
            packs_dir: PathBuf::from("packs"),
        }
    }

    /// Return a bevy [`App`] configured to run the bones game.
    pub fn app(mut self) -> App {
        let mut app = App::new();

        // Initialize Bevy plugins we use
        let mut plugins = DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    fit_canvas_to_parent: true,
                    ..default()
                }),
                ..default()
            })
            .build();
        if self.pixel_art {
            plugins = plugins.set(ImagePlugin::default_nearest());
        }

        app.add_plugins(plugins)
            .add_plugins((
                bevy_egui::EguiPlugin,
                lyon::ShapePlugin,
                debug::BevyDebugPlugin,
            ))
            .insert_resource({
                let mut egui_settings = bevy_egui::EguiSettings::default();
                if self.pixel_art {
                    egui_settings.use_nearest_descriptor();
                }
                egui_settings
            })
            .init_resource::<BonesImageIds>();

        'asset_load: {
            let Some(mut asset_server) = self.game.shared_resource_mut::<bones::AssetServer>()
            else {
                break 'asset_load;
            };
            asset_server.set_game_version(self.game_version);

            // Configure the AssetIO implementation
            #[cfg(not(target_arch = "wasm32"))]
            {
                let io = bones::FileAssetIo::new(&self.asset_dir, &self.packs_dir);
                asset_server.set_io(io);
            }
            #[cfg(target_arch = "wasm32")]
            {
                let window = web_sys::window().unwrap();
                let path = window.location().pathname().unwrap();
                let base = path.rsplit_once('/').map(|x| x.0).unwrap_or(&path);
                let io = bones::WebAssetIo::new(&format!("{base}/assets"));
                asset_server.set_io(io);
            }

            // Spawn the task to load game assets
            let s = asset_server.clone();
            IoTaskPool::get()
                .spawn(async move {
                    s.load_assets().await.unwrap();
                })
                .detach();

            // Enable asset hot reload.
            asset_server.watch_for_changes();
        }

        // Configure and load the persitent storage
        let mut storage = bones::Storage::with_backend(Box::new(storage::StorageBackend::new(
            &self.app_namespace.0,
            &self.app_namespace.1,
            &self.app_namespace.2,
        )));
        storage.load();
        self.game.insert_shared_resource(storage);

        self.game
            .insert_shared_resource(bones::EguiTextures::default());
        app.insert_resource(BonesImageIds::default());

        // Insert empty inputs that will be updated by the `insert_bones_input` system later.
        self.game.init_shared_resource::<bones::KeyboardInputs>();
        self.game.init_shared_resource::<bones::MouseInputs>();
        self.game.init_shared_resource::<bones::GamepadInputs>();

        // Insert the bones data
        app.insert_resource(BonesGame(self.game))
            .insert_resource(LoadingContext(self.custom_load_progress))
            .init_resource::<BonesGameEntity>();

        let assets_are_loaded = |game: Res<BonesGame>| {
            // Game is not required to have AssetServer, so default to true.
            game.asset_server()
                .as_ref()
                .map(|x| x.load_progress.is_finished())
                .unwrap_or(true)
        };
        let assets_not_loaded = |game: Res<BonesGame>| {
            game.asset_server()
                .as_ref()
                .map(|x| !x.load_progress.is_finished())
                .unwrap_or(true)
        };
        let egui_ctx_initialized =
            |game: Res<BonesGame>| game.shared_resource::<EguiCtx>().is_some();

        // Add the world sync systems
        app.add_systems(
            PreUpdate,
            (
                setup_egui,
                get_bones_input.pipe(insert_bones_input).after(InputSystem),
                egui_input_hook,
            )
                .chain()
                .run_if(assets_are_loaded)
                .after(bevy_egui::EguiSet::ProcessInput)
                .before(bevy_egui::EguiSet::BeginFrame),
        )
        .add_systems(Update, asset_load_status.run_if(assets_not_loaded))
        .add_systems(
            Update,
            (
                load_egui_textures,
                // Run world simulation
                step_bones_game,
                // Synchronize bones render components with the Bevy world.
                (
                    sync_egui_settings,
                    sync_clear_color,
                    sync_cameras,
                    sync_bones_path2ds,
                ),
            )
                .chain()
                .run_if(assets_are_loaded)
                .run_if(egui_ctx_initialized),
        );

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.add_systems(
                ExtractSchedule,
                (extract_bones_sprites, extract_bones_tilemaps)
                    .in_set(SpriteSystem::ExtractSprites)
                    .after(extract_sprites),
            );
        }

        app
    }
}

fn asset_load_status(
    game: Res<BonesGame>,
    mut custom_load_context: ResMut<LoadingContext>,
    mut egui_query: Query<&mut bevy_egui::EguiContext, With<Window>>,
) {
    let Some(asset_server) = &game.asset_server() else {
        return;
    };

    let mut ctx = egui_query.single_mut();
    if let Some(function) = &mut **custom_load_context {
        (function)(asset_server, ctx.get_mut());
    } else {
        default_load_progress(asset_server, ctx.get_mut());
    }
}

fn load_egui_textures(
    mut has_initialized: Local<bool>,
    game: ResMut<BonesGame>,
    mut bones_image_ids: ResMut<BonesImageIds>,
    mut bevy_images: ResMut<Assets<Image>>,
    mut bevy_egui_textures: ResMut<bevy_egui::EguiUserTextures>,
) {
    if !*has_initialized {
        *has_initialized = true;
    } else {
        return;
    }
    if let Some(asset_server) = &game.asset_server() {
        let bones_egui_textures_cell = game.shared_resource_cell::<bones::EguiTextures>().unwrap();
        // TODO: Avoid doing this every frame when there have been no assets loaded.
        // We should should be able to use the asset load progress event listener to detect newly
        // loaded assets that will need to be handled.
        let mut bones_egui_textures = bones_egui_textures_cell.borrow_mut().unwrap();
        // Take all loaded image assets and conver them to external images that reference bevy handles
        bones_image_ids.load_bones_images(
            asset_server,
            &mut bones_egui_textures,
            &mut bevy_images,
            &mut bevy_egui_textures,
        );
    }
}

/// System to step the bones simulation.
fn step_bones_game(world: &mut World) {
    let mut game = world.remove_resource::<BonesGame>().unwrap();
    let mut bones_image_ids = world.remove_resource::<BonesImageIds>().unwrap();
    let mut bevy_egui_textures = world
        .remove_resource::<bevy_egui::EguiUserTextures>()
        .unwrap();
    let mut bevy_images = world.remove_resource::<Assets<Image>>().unwrap();

    let mut winow_query = world.query::<&mut Window>();
    let mut window = winow_query.get_single_mut(world).unwrap();
    let bones_window = match game.shared_resource_cell::<bones::Window>() {
        Some(w) => w,
        None => {
            game.insert_shared_resource(bones::Window {
                size: vec2(window.width(), window.height()),
                fullscreen: matches!(&window.mode, WindowMode::BorderlessFullscreen),
            });
            game.shared_resource_cell().unwrap()
        }
    };
    let bones_window = bones_window.borrow_mut().unwrap();

    let is_fullscreen = matches!(&window.mode, WindowMode::BorderlessFullscreen);
    if is_fullscreen != bones_window.fullscreen {
        window.mode = if bones_window.fullscreen {
            WindowMode::BorderlessFullscreen
        } else {
            WindowMode::Windowed
        };
    }
    drop(bones_window);

    let bevy_time = world.resource::<Time>();

    // Reload assets if necessary
    if let Some(mut asset_server) = game.shared_resource_mut::<bones::AssetServer>() {
        asset_server.handle_asset_changes(|asset_server, handle| {
            let mut bones_egui_textures =
                game.shared_resource_mut::<bones::EguiTextures>().unwrap();
            let Some(mut asset) = asset_server.get_asset_untyped_mut(handle) else {
                // There was an issue loading the asset. The error will have been logged.
                return;
            };

            // TODO: hot reload changed fonts.

            if let Ok(image) = asset.data.try_cast_mut::<bones::Image>() {
                bones_image_ids.load_bones_image(
                    handle.typed(),
                    image,
                    &mut bones_egui_textures,
                    &mut bevy_images,
                    &mut bevy_egui_textures,
                );
            }
        })
    }

    // Step the game simulation
    game.step(bevy_time.last_update().unwrap_or_else(Instant::now));

    world.insert_resource(game);
    world.insert_resource(bones_image_ids);
    world.insert_resource(bevy_egui_textures);
    world.insert_resource(bevy_images);
}
