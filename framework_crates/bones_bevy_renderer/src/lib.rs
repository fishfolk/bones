//! Bevy plugin for rendering Bones framework games.

#![warn(missing_docs)]
// This cfg_attr is needed because `rustdoc::all` includes lints not supported on stable
#![cfg_attr(doc, allow(unknown_lints))]
#![deny(rustdoc::all)]

pub use bevy;

/// The prelude
pub mod prelude {
    pub use crate::*;
}

mod debug;
mod storage;

mod convert;
use convert::*;
mod input;
use input::*;
mod render;
use render::*;
mod ui;
use ui::*;

use bevy::prelude::*;
use bones_framework::prelude as bones;

use bevy::{
    input::InputSystem,
    render::RenderApp,
    sprite::{extract_sprites, SpriteSystem},
    tasks::IoTaskPool,
    utils::Instant,
};
use std::path::{Path, PathBuf};

/// Renderer for [`bones_framework`] [`Game`][bones::Game]s using Bevy.
pub struct BonesBevyRenderer {
    /// Whether or not to load all assets on startup with a loading screen,
    /// or skip straight to running the bones game immedietally.
    pub preload: bool,
    /// Optional field to implement your own loading screen. Does nothing if [`Self::preload`] = false
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
    /// Shorthand for [`bones::AssetServer`] typed access to the shared resource
    pub fn asset_server(&self) -> Option<bones::Ref<bones::AssetServer>> {
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
            preload: true,
            pixel_art: true,
            custom_load_progress: None,
            game,
            game_version: bones::Version::new(0, 1, 0),
            app_namespace: ("local".into(), "developer".into(), "bones_demo_game".into()),
            asset_dir: PathBuf::from("assets"),
            packs_dir: PathBuf::from("packs"),
        }
    }
    /// Whether or not to load all assets on startup with a loading screen,
    /// or skip straight to running the bones game immedietally.
    pub fn preload(self, preload: bool) -> Self {
        Self { preload, ..self }
    }
    /// Insert a custom loading screen function that will be used in place of the default
    pub fn loading_screen(mut self, function: LoadingFunction) -> Self {
        self.custom_load_progress = Some(function);
        self
    }
    /// Whether or not to use nearest-neighbor sampling for textures.
    pub fn pixel_art(self, pixel_art: bool) -> Self {
        Self { pixel_art, ..self }
    }
    /// The (qualifier, organization, application) that will be used to pick a persistent storage
    /// location for the game.
    ///
    /// For example: `("org", "fishfolk", "jumpy")`
    pub fn namespace(mut self, (qualifier, organization, application): (&str, &str, &str)) -> Self {
        self.app_namespace = (qualifier.into(), organization.into(), application.into());
        self
    }
    /// The path to load assets from.
    pub fn asset_dir(self, asset_dir: PathBuf) -> Self {
        Self { asset_dir, ..self }
    }
    /// The path to load asset packs from.
    pub fn packs_dir(self, packs_dir: PathBuf) -> Self {
        Self { packs_dir, ..self }
    }
    /// Set the version of the game, used for the asset loader.
    pub fn version(self, game_version: bones::Version) -> Self {
        Self {
            game_version,
            ..self
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

        app.add_plugins(plugins).add_plugins((
            bevy_egui::EguiPlugin,
            bevy_prototype_lyon::plugin::ShapePlugin,
            debug::BevyDebugPlugin,
        ));
        if self.pixel_art {
            app.insert_resource({
                let mut egui_settings = bevy_egui::EguiSettings::default();
                egui_settings.use_nearest_descriptor();
                egui_settings
            });
        }
        app.init_resource::<BonesImageIds>();

        if let Some(mut asset_server) = self.game.shared_resource_mut::<bones::AssetServer>() {
            asset_server.set_game_version(self.game_version);
            asset_server.set_io(asset_io(&self.asset_dir, &self.packs_dir));

            if self.preload {
                // Spawn the task to load game assets
                let s = asset_server.clone();
                IoTaskPool::get()
                    .spawn(async move {
                        s.load_assets().await.unwrap();
                    })
                    .detach();
            }

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

        // Insert empty inputs that will be updated by the `insert_bones_input` system later.
        self.game.init_shared_resource::<bones::KeyboardInputs>();
        self.game.init_shared_resource::<bones::MouseInputs>();
        self.game.init_shared_resource::<bones::GamepadInputs>();

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.game.init_shared_resource::<bones::ExitBones>();
            app.add_systems(Update, handle_exits);
        }

        // Insert the bones data
        app.insert_resource(BonesGame(self.game))
            .insert_resource(LoadingContext(self.custom_load_progress))
            .init_resource::<BonesGameEntity>();

        // Add the world sync systems
        app.add_systems(
            PreUpdate,
            (
                setup_egui,
                get_bones_input.pipe(insert_bones_input).after(InputSystem),
                egui_input_hook,
            )
                .chain()
                .run_if(assets_are_loaded.or_else(move || !self.preload))
                .after(bevy_egui::EguiSet::ProcessInput)
                .before(bevy_egui::EguiSet::BeginFrame),
        );
        if self.preload {
            app.add_systems(Update, asset_load_status.run_if(assets_not_loaded));
        }
        app.add_systems(
            Update,
            (
                load_egui_textures,
                sync_bones_window,
                handle_asset_changes,
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
                .run_if(assets_are_loaded.or_else(move || !self.preload))
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

fn egui_ctx_initialized(game: Res<BonesGame>) -> bool {
    game.shared_resource::<bones::EguiCtx>().is_some()
}

fn assets_are_loaded(game: Res<BonesGame>) -> bool {
    // Game is not required to have AssetServer, so default to true.
    game.asset_server()
        .as_ref()
        .map(|x| x.load_progress.is_finished())
        .unwrap_or(true)
}

fn assets_not_loaded(game: Res<BonesGame>) -> bool {
    game.asset_server()
        .as_ref()
        .map(|x| !x.load_progress.is_finished())
        .unwrap_or(true)
}

/// A [`bones::AssetIo`] configured for web and local file access
pub fn asset_io(asset_dir: &Path, packs_dir: &Path) -> impl bones::AssetIo + 'static {
    #[cfg(not(target_arch = "wasm32"))]
    {
        bones::FileAssetIo::new(asset_dir, packs_dir)
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = asset_dir;
        let _ = packs_dir;
        let window = web_sys::window().unwrap();
        let path = window.location().pathname().unwrap();
        let base = path.rsplit_once('/').map(|x| x.0).unwrap_or(&path);
        bones::WebAssetIo::new(&format!("{base}/assets"))
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
    world.resource_scope(|world: &mut World, mut game: Mut<BonesGame>| {
        let time = world.get_resource::<Time>().unwrap();
        game.step(time.last_update().unwrap_or_else(Instant::now));
    });
}

/// System for handling asset changes in the bones asset server
pub fn handle_asset_changes(
    game: ResMut<BonesGame>,
    mut bevy_images: ResMut<Assets<Image>>,
    mut bevy_egui_textures: ResMut<bevy_egui::EguiUserTextures>,
    mut bones_image_ids: ResMut<BonesImageIds>,
) {
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
}

#[cfg(not(target_arch = "wasm32"))]
fn handle_exits(game: Res<BonesGame>, mut exits: EventWriter<bevy::app::AppExit>) {
    if **game.shared_resource::<bones::ExitBones>().unwrap() {
        exits.send(bevy::app::AppExit);
    }
}
