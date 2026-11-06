//! Renderer for [`bones_framework`] [`Game`][bones::Game]s using [`wgpu`] and [`winit`].

use bevy_tasks::{IoTaskPool, TaskPool};
use convert::IntoBones;
use image::RgbaImage;
use pollster::FutureExt;
use std::{
    collections::{HashSet, VecDeque},
    path::{Path, PathBuf},
    sync::Arc,
};
use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Fullscreen, Window, WindowId},
};

pub use wgpu;
pub use winit;

use bones_framework::{
    glam::*,
    input::gilrs::process_gamepad_events,
    prelude::{self as bones},
};

use egui_wgpu::ScreenDescriptor;

mod atlas_pool;
mod convert;
mod dynamic_storage;
mod line;
mod rumble;
mod sprite;
mod storage;
mod ui;

use dynamic_storage::DynamicBuffer;
use sprite::*;
use ui::{default_load_progress, EguiRenderer};

/// The prelude
pub mod prelude {
    pub use crate::*;
}

//Wgpu utils Bones types

#[derive(bones_schema::HasSchema, Default, Clone)]
#[repr(C)]
#[schema(opaque)]
struct WgpuDevice(Option<Arc<wgpu::Device>>);

#[derive(bones_schema::HasSchema, Default, Clone)]
#[repr(C)]
struct PixelArt(bool);

#[derive(bones_schema::HasSchema, Default, bones::Deref, bones::DerefMut)]
#[schema(no_clone)]
struct LoadingContext(pub Option<LoadingFunction>);
/// Custom loading screen callback type used when [`BonesWgpuRenderer::preload`] is enabled.
pub type LoadingFunction =
    Box<dyn FnMut(&bones::AssetServer, &egui::Context) + Sync + Send + 'static>;

#[derive(bones_schema::HasSchema, Default, Clone)]
struct LoadedEguiAssets {
    cids: std::collections::HashMap<bones::UntypedHandle, bones::Cid>,
}

/// Renderer for [`bones_framework`] [`Game`][bones::Game]s using wgpu.
pub struct BonesWgpuRenderer {
    /// Whether or not to load all assets on startup with a loading screen,
    /// or skip straight to running the bones game immediately.
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
    /// The title of the window.
    pub window_title: String,
    /// The initial logical size of the window in pixels.
    pub window_size: Vec2,
    /// Optional minimum logical size of the window in pixels.
    pub min_window_size: Option<Vec2>,
    /// Whether or not the window can be resized by the user.
    pub window_resizable: bool,
    /// Whether or not the window should start in borderless fullscreen mode.
    pub fullscreen: bool,
    /// Optional icon for the window.
    pub window_icon: Option<winit::window::Icon>,
    /// Present mode used when configuring the swapchain surface (controls VSync).
    pub present_mode: wgpu::PresentMode,
}

impl Default for BonesWgpuRenderer {
    fn default() -> Self {
        Self::new(bones::Game::default())
    }
}

impl BonesWgpuRenderer {
    /// Create a new [`BonesWgpuRenderer`] for the provided game.
    pub fn new(game: bones::Game) -> Self {
        BonesWgpuRenderer {
            preload: true,
            custom_load_progress: None,
            pixel_art: true,
            game,
            game_version: bones::Version::new(0, 1, 0),
            app_namespace: ("local".into(), "developer".into(), "bones_demo_game".into()),
            asset_dir: PathBuf::from("assets"),
            packs_dir: PathBuf::from("packs"),
            window_title: "App".into(),
            window_size: Vec2::new(1280.0, 720.0),
            min_window_size: None,
            window_resizable: true,
            fullscreen: false,
            window_icon: None,
            present_mode: wgpu::PresentMode::AutoVsync,
        }
    }

    /// Whether or not to load all assets on startup with a loading screen,
    /// or skip straight to running the bones game immediately.
    pub fn preload(self, preload: bool) -> Self {
        Self { preload, ..self }
    }

    /// Insert a custom loading screen function that will be used in place of the default.
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

    /// Set the title of the window.
    pub fn window_title(mut self, title: impl Into<String>) -> Self {
        self.window_title = title.into();
        self
    }

    /// Alias for [`Self::window_title`].
    pub fn title(self, title: impl Into<String>) -> Self {
        self.window_title(title)
    }

    /// Set the initial logical size of the window in pixels.
    pub fn window_size(mut self, size: impl Into<Vec2>) -> Self {
        self.window_size = size.into();
        self
    }

    /// Set the minimum logical size of the window in pixels.
    pub fn min_window_size(mut self, min_size: impl Into<Vec2>) -> Self {
        self.min_window_size = Some(min_size.into());
        self
    }

    /// Set whether or not the window can be resized by the user.
    pub fn window_resizable(self, window_resizable: bool) -> Self {
        Self {
            window_resizable,
            ..self
        }
    }

    /// Set whether or not the window should start in borderless fullscreen mode.
    pub fn fullscreen(self, fullscreen: bool) -> Self {
        Self { fullscreen, ..self }
    }

    /// Set the window icon.
    pub fn window_icon(mut self, icon: winit::window::Icon) -> Self {
        self.window_icon = Some(icon);
        self
    }

    /// Set the swapchain present mode.
    pub fn present_mode(self, present_mode: wgpu::PresentMode) -> Self {
        Self {
            present_mode,
            ..self
        }
    }

    /// Enable or disable VSync (`AutoVsync` when `true`, `AutoNoVsync` when `false`).
    pub fn vsync(self, vsync: bool) -> Self {
        self.present_mode(if vsync {
            wgpu::PresentMode::AutoVsync
        } else {
            wgpu::PresentMode::AutoNoVsync
        })
    }

    pub fn run(mut self) {
        // wgpu uses `log` for all of its logging; initialize `env_logger` early as a fallback if
        // the caller did not already install a logger (e.g. via `setup_logs!()`).
        let _ = env_logger::try_init();

        //Start wgpu
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let (adapter, device, queue) = async {
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    ..Default::default()
                })
                .await
                .unwrap();

            let required_limits = wgpu::Limits::default().using_resolution(adapter.limits());
            let (device, queue) = adapter
                .request_device(
                    &wgpu::DeviceDescriptor {
                        required_limits,
                        ..Default::default()
                    },
                    None, // Trace path
                )
                .await
                .unwrap();
            (Arc::new(adapter), Arc::new(device), Arc::new(queue))
        }
        .block_on();

        // Texture bind group layout (matches @group(0) in WGSL)
        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    // This should match the filterable field of the textures
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("texture_bind_group_layout"),
        });

        // Storage buffer bind group layout (matches @group(1) in WGSL)
        let storage_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("storage_bind_group_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let camera_entry_size =
            std::num::NonZeroU64::new(std::mem::size_of::<CameraTransform>() as u64);
        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("camera_bind_group_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: true,
                    min_binding_size: camera_entry_size,
                },
                count: None,
            }],
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let instance = Arc::new(instance);
        let texture_layout = Arc::new(texture_layout);
        let storage_layout = Arc::new(storage_layout);
        let camera_layout = Arc::new(camera_layout);
        let vertex_buffer = Arc::new(vertex_buffer);
        let index_buffer = Arc::new(index_buffer);

        //This is used to store dynamically some rendering data, like transform, flip, etc
        let dynamic_storage =
            DynamicBuffer::new(&device, storage_layout, 1024, wgpu::BufferUsages::STORAGE);
        let camera_dynamic_uniform = DynamicBuffer::new_with_binding_size(
            &device,
            camera_layout,
            1024,
            wgpu::BufferUsages::STORAGE,
            camera_entry_size,
        );

        //Insert wgpu resources
        self.game
            .insert_shared_resource(WgpuDevice(Some(device.clone())));
        self.game.insert_shared_resource(PixelArt(self.pixel_art));
        self.game
            .insert_shared_resource(LoadingContext(self.custom_load_progress));

        //Deal with asset server
        IoTaskPool::init(TaskPool::default);
        if let Some(mut asset_server) = self.game.get_shared_resource_mut::<bones::AssetServer>() {
            asset_server.set_game_version(self.game_version.clone());
            asset_server.set_io(asset_io(&self.asset_dir, &self.packs_dir));

            if self.preload {
                // Load assets
                let s = asset_server.clone();
                log::info!("Loading assets...");

                // Spawn a task to load the assets
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
        self.game.init_shared_resource::<LoadedEguiAssets>();
        #[cfg(not(target_arch = "wasm32"))]
        self.game.init_shared_resource::<bones::ExitBones>();

        // Insert empty inputs that will be updated by the window/gamepad event handlers.
        self.game.init_shared_resource::<bones::KeyboardInputs>();
        self.game.init_shared_resource::<bones::MouseInputs>();
        self.game.init_shared_resource::<bones::GamepadInputs>();
        self.game.init_shared_resource::<bones::GamepadsRumble>();
        self.game
            .init_shared_resource::<bones::MouseScreenPosition>();
        self.game
            .init_shared_resource::<bones::MouseWorldPosition>();
        {
            let mut bones_window = self.game.init_shared_resource::<bones::Window>();
            if self.fullscreen {
                bones_window.fullscreen = true;
            } else {
                self.fullscreen = bones_window.fullscreen;
            }
            if bones_window.size == Vec2::ZERO {
                bones_window.size = self.window_size;
            }
        }
        self.game
            .init_shared_resource::<bones::debug::FrameDiagState>();

        let event_loop = EventLoop::builder().build().unwrap();

        // When the current loop iteration finishes, immediately begin a new
        // iteration regardless of whether or not new events are available to
        // process.
        event_loop.set_control_flow(ControlFlow::Poll);

        let bind_group_clone = texture_layout.clone();
        let device_clone = device.clone();

        let window_config = WindowConfig {
            title: self.window_title,
            size: self.window_size,
            min_size: self.min_window_size,
            resizable: self.window_resizable,
            fullscreen: self.fullscreen,
            icon: self.window_icon,
            present_mode: self.present_mode,
        };

        let mut app = App {
            state: None,
            game: self.game,
            atlas_pool: atlas_pool::AtlasPool::new(
                &device_clone,
                &bind_group_clone,
                (4096, 4096),
                64,
                self.pixel_art,
            ),
            storage_layout: dynamic_storage.layout.clone(),
            dynamic_storage: Some(dynamic_storage),
            camera_layout: camera_dynamic_uniform.layout.clone(),
            camera_dynamic_uniform: Some(camera_dynamic_uniform),
            texture_layout,
            vertex_buffer,
            index_buffer,
            rumble_manager: rumble::RumbleManager::default(),
            pressed_keys: HashSet::new(),
            pressed_mouse_buttons: HashSet::new(),
            preload_assets: self.preload,
            window_config,
            last_frame_instant: None,
            frame_time_history: VecDeque::with_capacity(20),
            queue,
            device,
            adapter,
            instance,
        };
        event_loop.run_app(&mut app).unwrap();
    }
}

fn asset_load_status(game: &mut bones::Game) {
    let asset_server = game.shared_resource::<bones::AssetServer>();
    let ctx = game.shared_resource::<bones::EguiCtx>();
    let mut function = game.shared_resource_mut::<LoadingContext>();

    if asset_server.load_progress.is_finished() {
        return;
    }

    if let Some(function) = &mut function.0 {
        (function)(&asset_server, &ctx);
    } else {
        default_load_progress(&asset_server, &ctx);
    }
}

/// Startup system to load egui fonts and textures.
pub fn setup_egui(game: &mut bones::Game, ctx: &egui::Context) {
    // Insert the egui context as a shared resource
    game.insert_shared_resource(bones::EguiCtx(ctx.clone()));

    if let Some(egui_textures) = game.shared_resource_cell::<bones::EguiTextures>() {
        ctx.data_mut(|map| {
            map.insert_temp(egui::Id::NULL, egui_textures);
        });
    }

    if let Some(bones_assets) = game.get_shared_resource::<bones::AssetServer>() {
        update_egui_fonts(ctx, &bones_assets);
    }
}

pub fn update_egui_fonts(ctx: &egui::Context, bones_assets: &bones::AssetServer) {
    let mut fonts = egui::FontDefinitions::default();

    for entry in bones_assets.store.assets.iter() {
        let asset = entry.value();
        if let Ok(font) = asset.try_cast_ref::<bones::Font>() {
            let previous = fonts
                .font_data
                .insert(font.family_name.to_string(), font.data.clone().into());
            if previous.is_some() {
                log::warn!(
                    "{} Found two fonts with the same family name, using \
                    only the latest one",
                    font.family_name
                );
            }
            fonts
                .families
                .entry(egui::FontFamily::Name(font.family_name.clone()))
                .or_default()
                .push(font.family_name.to_string());
        }
    }

    ctx.set_fonts(fonts);
}

/// System for handling asset changes in the bones asset server.
pub fn handle_asset_changes(game: &mut bones::Game) {
    if let Some(mut asset_server) = game.get_shared_resource_mut::<bones::AssetServer>() {
        asset_server.handle_asset_changes(|_asset_server, _handle| {});
    }
}

fn load_egui_textures(game: &mut bones::Game) {
    let Some(asset_server) = game.get_shared_resource::<bones::AssetServer>() else {
        return;
    };
    let ctx = game.shared_resource::<bones::EguiCtx>();
    let mut egui_textures = game.shared_resource_mut::<bones::EguiTextures>();
    let mut loaded_egui_assets = game.shared_resource_mut::<LoadedEguiAssets>();
    let pixel_art = game.shared_resource::<PixelArt>();

    // egui enforces its own max texture side (default is often 2048).
    // Also cap by the GPU/device limit so we don't upload textures the backend can't handle.
    let egui_max_texture_side = ctx.input(|i| i.max_texture_side);
    let device_max_texture_side = game
        .get_shared_resource::<WgpuDevice>()
        .and_then(|d| {
            d.0.as_ref()
                .map(|d| d.limits().max_texture_dimension_2d as usize)
        })
        .unwrap_or(usize::MAX);
    let max_texture_side = egui_max_texture_side.min(device_max_texture_side);

    let mut fonts_changed = false;

    for entry in asset_server.store.asset_ids.iter() {
        let untyped_handle = *entry.key();
        let cid = *entry.value();
        let id = untyped_handle.typed();

        if loaded_egui_assets.cids.get(&untyped_handle) == Some(&cid) {
            // We already loaded this exact content version.
            continue;
        }

        let Some(asset) = asset_server.store.assets.get(&cid) else {
            continue;
        };

        if let Ok(bones::Image::Data(data)) = asset.data.try_cast_ref::<bones::Image>() {
            let rgba: RgbaImage = data.to_rgba8();
            let (w, h) = (rgba.width() as usize, rgba.height() as usize);

            if w > max_texture_side || h > max_texture_side {
                log::warn!(
                    "Skipping egui texture upload for {:?} ({}x{} exceeds max texture side {})",
                    entry.key(),
                    w,
                    h,
                    max_texture_side
                );
                loaded_egui_assets.cids.insert(untyped_handle, cid);
                continue;
            }

            let raw = rgba.into_raw();

            let handle = ctx.load_texture(
                format!("Texture {:?}", entry.key()),
                egui::ColorImage::from_rgba_unmultiplied([w, h], &raw),
                egui::TextureOptions {
                    magnification: if pixel_art.0 {
                        egui::TextureFilter::Nearest
                    } else {
                        egui::TextureFilter::Linear
                    },
                    minification: if pixel_art.0 {
                        egui::TextureFilter::Nearest
                    } else {
                        egui::TextureFilter::Linear
                    },
                    ..Default::default()
                },
            );
            egui_textures.insert(id, handle);
            loaded_egui_assets.cids.insert(untyped_handle, cid);
        } else if asset.data.try_cast_ref::<bones::Font>().is_ok() {
            loaded_egui_assets.cids.insert(untyped_handle, cid);
            fonts_changed = true;
        }
    }

    if fonts_changed {
        update_egui_fonts(&ctx, &asset_server);
    }
}

struct WindowConfig {
    title: String,
    size: Vec2,
    min_size: Option<Vec2>,
    resizable: bool,
    fullscreen: bool,
    icon: Option<winit::window::Icon>,
    present_mode: wgpu::PresentMode,
}

impl WindowConfig {
    fn window_attributes(&self, want_fullscreen: bool) -> winit::window::WindowAttributes {
        let mut attrs = Window::default_attributes()
            .with_title(self.title.clone())
            .with_resizable(self.resizable)
            .with_window_icon(self.icon.clone());

        if self.size.x > 0.0 && self.size.y > 0.0 {
            attrs = attrs.with_inner_size(winit::dpi::LogicalSize::new(
                self.size.x as f64,
                self.size.y as f64,
            ));
        }

        if let Some(min_size) = self.min_size {
            if min_size.x > 0.0 && min_size.y > 0.0 {
                attrs = attrs.with_min_inner_size(winit::dpi::LogicalSize::new(
                    min_size.x as f64,
                    min_size.y as f64,
                ));
            }
        }

        if want_fullscreen {
            attrs = attrs.with_fullscreen(Some(Fullscreen::Borderless(None)));
        }

        attrs
    }
}

fn extract_scan_code(physical_key: &winit::keyboard::PhysicalKey) -> bones::Maybe<u32> {
    #[cfg(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    {
        use winit::platform::scancode::PhysicalKeyExtScancode;
        if let Some(scancode) = physical_key.to_scancode() {
            return bones::Set(scancode);
        }
    }

    match physical_key {
        winit::keyboard::PhysicalKey::Code(_) => bones::Unset,
        winit::keyboard::PhysicalKey::Unidentified(native_key_code) => match native_key_code {
            winit::keyboard::NativeKeyCode::Android(u) => bones::Set(*u),
            winit::keyboard::NativeKeyCode::MacOS(u) => bones::Set(*u as u32),
            winit::keyboard::NativeKeyCode::Windows(u) => bones::Set(*u as u32),
            winit::keyboard::NativeKeyCode::Xkb(u) => bones::Set(*u),
            winit::keyboard::NativeKeyCode::Unidentified => bones::Unset,
        },
    }
}

/// Top-level winit application state.
///
/// Fields are ordered so dependent GPU/window resources (`state`, `game`, `atlas_pool`, buffers,
/// layouts) drop before `queue`, `device`, `adapter`, and `instance`.
struct App {
    state: Option<State>,
    game: bones::Game,
    atlas_pool: atlas_pool::AtlasPool,
    dynamic_storage: Option<DynamicBuffer>,
    camera_dynamic_uniform: Option<DynamicBuffer>,
    storage_layout: Arc<wgpu::BindGroupLayout>,
    camera_layout: Arc<wgpu::BindGroupLayout>,
    texture_layout: Arc<wgpu::BindGroupLayout>,
    vertex_buffer: Arc<wgpu::Buffer>,
    index_buffer: Arc<wgpu::Buffer>,
    rumble_manager: rumble::RumbleManager,
    pressed_keys: HashSet<(Option<u32>, Option<bones::KeyCode>)>,
    pressed_mouse_buttons: HashSet<bones::MouseButton>,
    preload_assets: bool,
    window_config: WindowConfig,
    last_frame_instant: Option<bones::Instant>,
    frame_time_history: VecDeque<f64>,
    queue: Arc<wgpu::Queue>,
    device: Arc<wgpu::Device>,
    adapter: Arc<wgpu::Adapter>,
    instance: Arc<wgpu::Instance>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let want_fullscreen = self
            .game
            .get_shared_resource::<bones::Window>()
            .map(|w| w.fullscreen)
            .unwrap_or(self.window_config.fullscreen);

        // Create window object with configured title, size, icon, and fullscreen attributes
        let window = Arc::new(
            event_loop
                .create_window(self.window_config.window_attributes(want_fullscreen))
                .unwrap(),
        );

        if let Some(state) = &mut self.state {
            state.window = window.clone();
            state.size = window.inner_size();

            state.surface = self.instance.create_surface(window.clone()).unwrap();
            let cap = state.surface.get_capabilities(&self.adapter);
            let new_format = cap
                .formats
                .iter()
                .copied()
                .find(|f| f.is_srgb())
                .unwrap_or(cap.formats[0]);
            let format_changed =
                new_format.add_srgb_suffix() != state.surface_format.add_srgb_suffix();
            state.surface_format = new_format;
            state.supported_present_modes = cap.present_modes;

            if format_changed {
                let view_format = state.surface_format.add_srgb_suffix();
                let (sprite_pipeline, line_pipeline) = State::create_pipelines(
                    &self.device,
                    &self.texture_layout,
                    &state.dynamic_storage.layout,
                    &state.camera_dynamic_uniform.layout,
                    view_format,
                );
                state.sprite_render_pipeline = sprite_pipeline;
                state.line_render_pipeline = line_pipeline;
                state.egui_renderer =
                    EguiRenderer::new(&self.device, view_format, None, 1, &window);
                state.egui_fonts_loaded = false;
                if let Some(mut loaded) = self.game.get_shared_resource_mut::<LoadedEguiAssets>() {
                    loaded.cids.clear();
                }
                setup_egui(&mut self.game, &state.egui_renderer.context().clone());
            }

            state.configure_surface();
            state.sync_window_state(&mut self.game);
        } else {
            let state = State::new(
                window.clone(),
                self.device.clone(),
                self.queue.clone(),
                &self.instance,
                &self.adapter,
                self.texture_layout.clone(),
                self.vertex_buffer.clone(),
                self.index_buffer.clone(),
                self.dynamic_storage.take().unwrap_or_else(|| {
                    DynamicBuffer::new(
                        &self.device,
                        self.storage_layout.clone(),
                        1024,
                        wgpu::BufferUsages::STORAGE,
                    )
                }),
                self.camera_dynamic_uniform.take().unwrap_or_else(|| {
                    DynamicBuffer::new_with_binding_size(
                        &self.device,
                        self.camera_layout.clone(),
                        1024,
                        wgpu::BufferUsages::STORAGE,
                        std::num::NonZeroU64::new(std::mem::size_of::<CameraTransform>() as u64),
                    )
                }),
                self.preload_assets,
                self.window_config.present_mode,
            );

            setup_egui(&mut self.game, &state.egui_renderer.context().clone());
            state.sync_window_state(&mut self.game);

            self.state = Some(state);
        }

        window.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(state) = self.state.as_mut() else {
            // State may already be dropped during shutdown.
            return;
        };

        // Egui input handling (receives key repeats too so text editing works naturally)
        state
            .egui_renderer
            .handle_input(&state.get_window(), &event);

        match event {
            WindowEvent::CloseRequested => {
                self.state.take();
                event_loop.exit();
            }
            WindowEvent::Focused(focused) => {
                if let Some(mut bones_window) = self.game.get_shared_resource_mut::<bones::Window>()
                {
                    bones_window.focused = focused;
                }
                if !focused {
                    // Release any held keys and mouse buttons on focus loss so controls never stick
                    // when the user Alt-Tabs away while holding a button.
                    if !self.pressed_keys.is_empty() {
                        if let Some(mut keyboard_inputs) =
                            self.game.get_shared_resource_mut::<bones::KeyboardInputs>()
                        {
                            for (scan_code, key_code) in self.pressed_keys.drain() {
                                keyboard_inputs.key_events.push(bones::KeyboardEvent {
                                    scan_code: scan_code.into(),
                                    key_code: key_code.into(),
                                    button_state: bones::ButtonState::Released,
                                });
                            }
                        } else {
                            self.pressed_keys.clear();
                        }
                    }
                    if !self.pressed_mouse_buttons.is_empty() {
                        if let Some(mut mouse_inputs) =
                            self.game.get_shared_resource_mut::<bones::MouseInputs>()
                        {
                            for button in self.pressed_mouse_buttons.drain() {
                                mouse_inputs.button_events.push(bones::MouseButtonEvent {
                                    button,
                                    state: bones::ButtonState::Released,
                                });
                            }
                        } else {
                            self.pressed_mouse_buttons.clear();
                        }
                    }
                }
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                let size = state.window.inner_size();
                state.resize(size);
                if size.width > 0 && size.height > 0 {
                    if let Some(mut bones_window) =
                        self.game.get_shared_resource_mut::<bones::Window>()
                    {
                        let logical: winit::dpi::LogicalSize<f32> =
                            size.to_logical(state.window.scale_factor());
                        bones_window.size = Vec2::new(logical.width, logical.height);
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                #[cfg(not(target_arch = "wasm32"))]
                if self
                    .game
                    .get_shared_resource::<bones::ExitBones>()
                    .is_some_and(|exit| exit.0)
                {
                    self.state.take();
                    event_loop.exit();
                    return;
                }

                // Update frame time diagnostics (`FrameDiagState`).
                let now = bones::Instant::now();
                if let Some(last) = self.last_frame_instant.replace(now) {
                    let dt_secs = now.duration_since(last).as_secs_f64();
                    if dt_secs > 0.0 {
                        const MAX_HISTORY: usize = 20;
                        if self.frame_time_history.len() >= MAX_HISTORY {
                            self.frame_time_history.pop_front();
                        }
                        self.frame_time_history.push_back(dt_secs);

                        let avg_dt = self.frame_time_history.iter().sum::<f64>()
                            / self.frame_time_history.len() as f64;
                        let fps = 1.0 / dt_secs;
                        let fps_avg = if avg_dt > 0.0 { 1.0 / avg_dt } else { 0.0 };
                        let frame_time_ms = dt_secs * 1000.0;
                        let frame_time_avg_ms = avg_dt * 1000.0;

                        let mut diag = self
                            .game
                            .init_shared_resource::<bones::debug::FrameDiagState>();
                        diag.fps = fps;
                        diag.fps_avg = fps_avg;
                        if fps < diag.min_fps {
                            diag.min_fps = fps;
                        }
                        if fps > diag.max_fps {
                            diag.max_fps = fps;
                        }
                        diag.frame_time = frame_time_ms;
                        diag.frame_time_avg = frame_time_avg_ms;
                        if frame_time_ms < diag.min_frame_time {
                            diag.min_frame_time = frame_time_ms;
                        }
                        if frame_time_ms > diag.max_frame_time {
                            diag.max_frame_time = frame_time_ms;
                        }
                    }
                }

                // Poll gamepad inputs once per frame and update mouse world position.
                self.game.insert_shared_resource(process_gamepad_events());
                let scale_factor = (state.window.scale_factor() as f32).max(f32::EPSILON);
                let screen_pos = self
                    .game
                    .get_shared_resource::<bones::MouseScreenPosition>()
                    .and_then(|p| p.0);
                let world_pos = screen_pos.and_then(|logical_sp| {
                    screen_to_world_2d(
                        &self.game,
                        logical_sp * scale_factor,
                        UVec2::new(state.size.width, state.size.height),
                        scale_factor,
                    )
                });
                self.game
                    .insert_shared_resource(bones::MouseWorldPosition(world_pos));

                state.render(&mut self.game, &mut self.atlas_pool);

                // Process any gamepad rumble requests queued during this frame.
                if let Some(mut rumble) =
                    self.game.get_shared_resource_mut::<bones::GamepadsRumble>()
                {
                    self.rumble_manager.process(&mut rumble);
                }

                // Clear per-frame input events after the game has been stepped/rendered.
                if let Some(mut keyboard_inputs) =
                    self.game.get_shared_resource_mut::<bones::KeyboardInputs>()
                {
                    keyboard_inputs.key_events.clear();
                }
                if let Some(mut mouse_inputs) =
                    self.game.get_shared_resource_mut::<bones::MouseInputs>()
                {
                    mouse_inputs.wheel_events.clear();
                    mouse_inputs.button_events.clear();
                    mouse_inputs.movement = Vec2::ZERO;
                }

                // Emits a new redraw requested event.
                state.get_window().request_redraw();
            }
            WindowEvent::Resized(size) => {
                // Reconfigures the size of the surface. We do not re-render
                // here as this event is always followed up by redraw request.
                state.resize(size);
                if size.width > 0 && size.height > 0 {
                    if let Some(mut bones_window) =
                        self.game.get_shared_resource_mut::<bones::Window>()
                    {
                        let logical: winit::dpi::LogicalSize<f32> =
                            size.to_logical(state.window.scale_factor());
                        bones_window.size = Vec2::new(logical.width, logical.height);
                    }
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.repeat {
                    return;
                }
                let scan_code = extract_scan_code(&event.physical_key);
                let key_code = match event.physical_key {
                    winit::keyboard::PhysicalKey::Code(code) => code.into_bones(),
                    winit::keyboard::PhysicalKey::Unidentified(_) => bones::Unset,
                };
                let button_state: bones::ButtonState = event.state.into_bones();
                let key_id = (scan_code.option(), key_code.option());
                if button_state.pressed() {
                    self.pressed_keys.insert(key_id);
                } else {
                    self.pressed_keys.remove(&key_id);
                }
                let ev = bones::KeyboardEvent {
                    scan_code,
                    key_code,
                    button_state,
                };
                if let Some(mut keyboard_inputs) =
                    self.game.get_shared_resource_mut::<bones::KeyboardInputs>()
                {
                    keyboard_inputs.key_events.push(ev);
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let ev: bones::MouseScrollEvent = delta.into_bones();
                if let Some(mut mouse_inputs) =
                    self.game.get_shared_resource_mut::<bones::MouseInputs>()
                {
                    mouse_inputs.wheel_events.push(ev);
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let bones_button: bones::MouseButton = button.into_bones();
                let bones_state: bones::ButtonState = state.into_bones();
                if bones_state.pressed() {
                    self.pressed_mouse_buttons.insert(bones_button);
                } else {
                    self.pressed_mouse_buttons.remove(&bones_button);
                }
                let ev = bones::MouseButtonEvent {
                    button: bones_button,
                    state: bones_state,
                };
                if let Some(mut mouse_inputs) =
                    self.game.get_shared_resource_mut::<bones::MouseInputs>()
                {
                    mouse_inputs.button_events.push(ev);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let scale_factor = (state.window.scale_factor() as f32).max(f32::EPSILON);
                let physical_sp = Vec2::new(position.x as f32, position.y as f32);
                let logical_sp = physical_sp / scale_factor;
                let world_pos = screen_to_world_2d(
                    &self.game,
                    physical_sp,
                    UVec2::new(state.size.width, state.size.height),
                    scale_factor,
                );
                self.game
                    .insert_shared_resource(bones::MouseScreenPosition(Some(logical_sp)));
                self.game
                    .insert_shared_resource(bones::MouseWorldPosition(world_pos));
            }
            WindowEvent::CursorLeft { .. } => {
                self.game
                    .insert_shared_resource(bones::MouseScreenPosition(None));
                self.game
                    .insert_shared_resource(bones::MouseWorldPosition(None));
            }
            _ => (),
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        if let winit::event::DeviceEvent::MouseMotion { delta } = event {
            if self.state.as_ref().is_some_and(|s| s.window.has_focus()) {
                let delta = Vec2::new(delta.0 as f32, delta.1 as f32);
                if let Some(mut mouse_inputs) =
                    self.game.get_shared_resource_mut::<bones::MouseInputs>()
                {
                    mouse_inputs.movement += delta;
                }
            }
        }
    }
}

struct State {
    egui_renderer: EguiRenderer,
    sprite_render_pipeline: wgpu::RenderPipeline,
    line_render_pipeline: wgpu::RenderPipeline,
    line_buffers: line::LineBuffers,
    dynamic_storage: DynamicBuffer,
    camera_dynamic_uniform: DynamicBuffer,
    vertex_buffer: Arc<wgpu::Buffer>,
    index_buffer: Arc<wgpu::Buffer>,
    surface: wgpu::Surface<'static>,
    surface_format: wgpu::TextureFormat,
    present_mode: wgpu::PresentMode,
    supported_present_modes: Vec<wgpu::PresentMode>,
    size: winit::dpi::PhysicalSize<u32>,
    egui_scale_factor: f32,
    egui_fonts_loaded: bool,
    preload_assets: bool,
    /// When preloading assets, we pause all sessions to avoid running user startup systems before
    /// assets exist (which would create default handles with no CID and panic on access).
    saved_session_states: Option<Vec<(bones::Ustr, bool, bool)>>,
    queue: Arc<wgpu::Queue>,
    device: Arc<wgpu::Device>,
    window: Arc<Window>,
}

impl State {
    fn create_pipelines(
        device: &wgpu::Device,
        texture_layout: &wgpu::BindGroupLayout,
        storage_layout: &wgpu::BindGroupLayout,
        camera_layout: &wgpu::BindGroupLayout,
        view_format: wgpu::TextureFormat,
    ) -> (wgpu::RenderPipeline, wgpu::RenderPipeline) {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("atlas_sprite.wgsl").into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[texture_layout, storage_layout, camera_layout],
                push_constant_ranges: &[],
            });

        let sprite_render_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Sprite Render Pipeline"),
                layout: Some(&render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[Vertex::desc()],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: view_format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    // Requires Features::DEPTH_CLIP_CONTROL
                    unclipped_depth: false,
                    // Requires Features::CONSERVATIVE_RASTERIZATION
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
                cache: None,
            });

        let line_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Line Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("line.wgsl").into()),
        });

        let line_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Line Render Pipeline Layout"),
            bind_group_layouts: &[storage_layout, camera_layout],
            push_constant_ranges: &[],
        });

        let line_render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Line Render Pipeline"),
            layout: Some(&line_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &line_shader,
                entry_point: Some("vs_main"),
                buffers: &[line::Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &line_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: view_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        (sprite_render_pipeline, line_render_pipeline)
    }

    #[allow(clippy::too_many_arguments)]
    fn new(
        window: Arc<Window>,
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        instance: &wgpu::Instance,
        adapter: &wgpu::Adapter,
        texture_layout: Arc<wgpu::BindGroupLayout>,
        vertex_buffer: Arc<wgpu::Buffer>,
        index_buffer: Arc<wgpu::Buffer>,
        dynamic_storage: DynamicBuffer,
        camera_dynamic_uniform: DynamicBuffer,
        preload_assets: bool,
        present_mode: wgpu::PresentMode,
    ) -> Self {
        let size = window.inner_size();
        let surface = instance.create_surface(window.clone()).unwrap();
        let surface_caps = surface.get_capabilities(adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);
        let supported_present_modes = surface_caps.present_modes;
        let view_format = surface_format.add_srgb_suffix();

        let (sprite_render_pipeline, line_render_pipeline) = Self::create_pipelines(
            &device,
            &texture_layout,
            &dynamic_storage.layout,
            &camera_dynamic_uniform.layout,
            view_format,
        );
        let egui_renderer = EguiRenderer::new(&device, view_format, None, 1, &window);
        let line_buffers = line::LineBuffers::new(&device);

        let state = State {
            egui_renderer,
            sprite_render_pipeline,
            line_render_pipeline,
            line_buffers,
            dynamic_storage,
            camera_dynamic_uniform,
            vertex_buffer,
            index_buffer,
            surface,
            surface_format,
            present_mode,
            supported_present_modes,
            size,
            egui_scale_factor: 1.0,
            egui_fonts_loaded: false,
            preload_assets,
            saved_session_states: None,
            queue,
            device,
            window,
        };
        state.configure_surface();
        state
    }

    fn get_window(&self) -> Arc<Window> {
        self.window.clone()
    }

    fn resolved_present_mode(&self) -> wgpu::PresentMode {
        if matches!(
            self.present_mode,
            wgpu::PresentMode::AutoVsync | wgpu::PresentMode::AutoNoVsync
        ) || self.supported_present_modes.contains(&self.present_mode)
        {
            self.present_mode
        } else {
            log::warn!(
                "Requested present mode {:?} is not supported by surface; falling back to AutoVsync",
                self.present_mode
            );
            wgpu::PresentMode::AutoVsync
        }
    }

    fn configure_surface(&self) {
        if self.size.width == 0 || self.size.height == 0 {
            return;
        }
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: self.surface_format,
            // Request compatibility with the sRGB-format texture view we‘re going to create later.
            view_formats: vec![self.surface_format.add_srgb_suffix()],
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            width: self.size.width,
            height: self.size.height,
            desired_maximum_frame_latency: 2,
            present_mode: self.resolved_present_mode(),
        };
        self.surface.configure(&self.device, &surface_config);
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            self.size = new_size;
            return;
        }
        self.size = new_size;

        // reconfigure the surface
        self.configure_surface();
    }

    fn sync_window_state(&self, game: &mut bones::Game) {
        if let Some(mut bones_window) = game.get_shared_resource_mut::<bones::Window>() {
            let inner_size = self.window.inner_size();
            if inner_size.width > 0 && inner_size.height > 0 {
                let logical: winit::dpi::LogicalSize<f32> =
                    inner_size.to_logical(self.window.scale_factor());
                bones_window.size = Vec2::new(logical.width, logical.height);
            }
            bones_window.focused = self.window.has_focus();

            let want_fullscreen = bones_window.fullscreen;
            let is_fullscreen = self.window.fullscreen().is_some();
            if want_fullscreen && !is_fullscreen {
                self.window
                    .set_fullscreen(Some(Fullscreen::Borderless(self.window.current_monitor())));
            } else if !want_fullscreen && is_fullscreen {
                self.window.set_fullscreen(None);
            }
        }
    }

    fn sync_egui_scale_factor(&mut self, game: &bones::Game) {
        for session_name in sorted_visible_sessions(game) {
            if let Some(session) = game.sessions.get(session_name) {
                if let Some(settings) = session.world.get_resource::<bones::EguiSettings>() {
                    if settings.scale.is_finite() && settings.scale > 0.0 {
                        self.egui_scale_factor = settings.scale as f32;
                    }
                }
            }
        }
    }

    fn acquire_surface_texture(&mut self) -> Option<wgpu::SurfaceTexture> {
        match self.surface.get_current_texture() {
            Ok(texture) => Some(texture),
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.configure_surface();
                None
            }
            Err(wgpu::SurfaceError::Timeout) => {
                log::warn!("Timed out acquiring next swapchain texture");
                None
            }
            Err(wgpu::SurfaceError::OutOfMemory) => {
                panic!("Swapchain out of memory");
            }
            Err(wgpu::SurfaceError::Other) => {
                log::warn!("Failed to acquire swapchain texture (SurfaceError::Other)");
                None
            }
        }
    }

    fn render(&mut self, game: &mut bones::Game, atlas_pool: &mut atlas_pool::AtlasPool) {
        let is_minimized = self.size.width == 0 || self.size.height == 0;

        // Keep the Bones window resource (size, focus, fullscreen) synced with the winit window.
        self.sync_window_state(game);

        // Check whether assets have finished preloading before starting the egui pass so
        // `update_egui_fonts` can register loaded font definitions before `begin_pass`.
        let assets_loaded = game
            .get_shared_resource::<bones::AssetServer>()
            .map(|asset_server| asset_server.load_progress.is_finished())
            .unwrap_or(true);

        if (!self.preload_assets || assets_loaded) && !self.egui_fonts_loaded {
            if let Some(bones_assets) = game.get_shared_resource::<bones::AssetServer>() {
                update_egui_fonts(self.egui_renderer.context(), &bones_assets);
            }
            self.egui_fonts_loaded = true;
        }

        if self.preload_assets && !assets_loaded {
            // Save current session active/visible states once, then pause sessions.
            // (Even though we skip stepping below, this prevents any external callers from
            // observing sessions as "active" during preload.)
            if self.saved_session_states.is_none() {
                self.saved_session_states = Some(
                    game.sessions
                        .iter()
                        .map(|(name, session)| (*name, session.active, session.visible))
                        .collect(),
                );
            }
            for (_, session) in game.sessions.iter_mut() {
                session.active = false;
            }

            if is_minimized {
                return;
            }

            self.sync_egui_scale_factor(game);
            let pixels_per_point = self.window.scale_factor() as f32 * self.egui_scale_factor;
            self.egui_renderer.ppp(pixels_per_point);

            // Do not invoke `EguiInputHook` during preload (matching `bones_bevy_renderer`), as
            // user input hooks may expect preloaded assets to exist.
            let raw_input = self.egui_renderer.take_raw_input(&self.window);
            self.egui_renderer.begin_frame_with_raw_input(raw_input);

            // Render the loading UI (and do NOT step the game).
            asset_load_status(game);

            let Some(surface_texture) = self.acquire_surface_texture() else {
                self.egui_renderer
                    .end_frame_discard(&self.device, &self.queue, &self.window);
                return;
            };
            let texture_view = surface_texture
                .texture
                .create_view(&wgpu::TextureViewDescriptor {
                    format: Some(self.surface_format.add_srgb_suffix()),
                    ..Default::default()
                });

            let mut encoder = self.device.create_command_encoder(&Default::default());
            let screen_descriptor = ScreenDescriptor {
                size_in_pixels: [self.size.width, self.size.height],
                pixels_per_point,
            };

            self.egui_renderer.end_frame_and_draw(
                &self.device,
                &self.queue,
                &mut encoder,
                &self.window,
                &texture_view,
                screen_descriptor,
            );

            self.queue.submit([encoder.finish()]);
            self.window.pre_present_notify();
            surface_texture.present();
            return;
        }

        if self.preload_assets {
            if let Some(saved) = self.saved_session_states.take() {
                // Restore original session active/visible states.
                for (name, active, visible) in saved {
                    if let Some(session) = game.sessions.get_mut(name) {
                        session.active = active;
                        session.visible = visible;
                    }
                }
            }
        }

        // Process hot-reloaded assets and register any newly loaded/updated textures in EguiTextures before stepping.
        handle_asset_changes(game);
        load_egui_textures(game);

        // Apply current egui scale factor BEFORE `take_raw_input` / `begin_pass` so `begin_pass`
        // and `end_pass` use the exact same `pixels_per_point` throughout this frame.
        self.sync_egui_scale_factor(game);
        let pixels_per_point = self.window.scale_factor() as f32 * self.egui_scale_factor;
        self.egui_renderer.ppp(pixels_per_point);

        // Begin egui frame (and let the game mutate RawInput via EguiInputHook).
        let mut raw_input = self.egui_renderer.take_raw_input(&self.window);
        let input_hook = game
            .get_shared_resource::<bones::EguiInputHook>()
            .map(|h| h.0.clone());
        if let Some(input_hook) = input_hook {
            (input_hook)(game, &mut raw_input);
        }
        self.egui_renderer.begin_frame_with_raw_input(raw_input);

        // Step bones normally once assets are ready (or if preload is disabled).
        // Note: we still step when minimized (`0x0` window) so networked/GGRS sessions stay in sync.
        game.step(bones::Instant::now());

        // Sync egui scale factor for next frame and apply any fullscreen changes requested by the game.
        self.sync_egui_scale_factor(game);
        self.sync_window_state(game);

        if is_minimized {
            self.egui_renderer
                .end_frame_discard(&self.device, &self.queue, &self.window);
            return;
        }

        // Determine the frame clear color from the highest-priority visible session's `ClearColor`.
        let mut frame_clear_bones_color = bones::Color::BLACK;
        for session_name in sorted_visible_sessions(game).into_iter().rev() {
            if let Some(session) = game.sessions.get(session_name) {
                if let Some(clear_color) = session.world.get_resource::<bones::ClearColor>() {
                    frame_clear_bones_color = clear_color.0;
                    break;
                }
            }
        }
        let frame_clear_color = convert::color_to_wgpu(frame_clear_bones_color);

        let scale_factor = (self.window.scale_factor() as f32).max(f32::EPSILON);
        let target_size = UVec2::new(self.size.width, self.size.height);

        update_atlas_pool(game, atlas_pool, &self.queue);
        let prepared_sessions = prepare_render_frame(
            game,
            atlas_pool,
            &self.device,
            &self.queue,
            &mut self.camera_dynamic_uniform,
            &mut self.dynamic_storage,
            &mut self.line_buffers,
            target_size,
            scale_factor,
            frame_clear_bones_color,
        );

        // Acquire swapchain texture view.
        let Some(surface_texture) = self.acquire_surface_texture() else {
            self.egui_renderer
                .end_frame_discard(&self.device, &self.queue, &self.window);
            return;
        };
        let texture_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                // Without add_srgb_suffix() the image we will be working with
                // might not be "gamma correct".
                format: Some(self.surface_format.add_srgb_suffix()),
                ..Default::default()
            });

        // Create the command encoder.
        let mut encoder = self.device.create_command_encoder(&Default::default());

        {
            let _clear_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(frame_clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }

        #[derive(Clone, Copy, PartialEq, Eq)]
        enum ActivePipeline {
            None,
            Sprite,
            Line,
        }

        for prepared in &prepared_sessions {
            if log::log_enabled!(log::Level::Debug) {
                log::debug!(
                    "Render session {:?}: cameras={}, draw_cmds={}",
                    prepared.session_name,
                    prepared.cameras.len(),
                    prepared.draws.len(),
                );
            }

            for cam_draw in &prepared.cameras {
                let load_op = match cam_draw.load_clear_color {
                    Some(color) => wgpu::LoadOp::Clear(color),
                    None => wgpu::LoadOp::Load,
                };

                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &texture_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: load_op,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

                let mut active_pipeline = ActivePipeline::None;

                if cam_draw.viewport.is_custom {
                    pass.set_viewport(
                        cam_draw.viewport.position.x,
                        cam_draw.viewport.position.y,
                        cam_draw.viewport.size.x,
                        cam_draw.viewport.size.y,
                        cam_draw.viewport.depth_min,
                        cam_draw.viewport.depth_max,
                    );

                    // Clear sub-window viewport if camera.draw_background_color is true
                    if let Some(ref bg_draw) = cam_draw.viewport_bg_draw {
                        pass.set_pipeline(&self.line_render_pipeline);
                        pass.set_vertex_buffer(0, self.line_buffers.vertex_buffer.slice(..));
                        pass.set_index_buffer(
                            self.line_buffers.index_buffer.slice(..),
                            wgpu::IndexFormat::Uint32,
                        );
                        pass.set_bind_group(0, self.dynamic_storage.get_bind_group(), &[]);
                        pass.set_bind_group(
                            1,
                            self.camera_dynamic_uniform.get_bind_group(),
                            &[cam_draw.uniform_offset],
                        );
                        pass.draw_indexed(
                            bg_draw.index_range.clone(),
                            bg_draw.base_vertex,
                            bg_draw.instance_index..bg_draw.instance_index + 1,
                        );
                        active_pipeline = ActivePipeline::Line;
                    }
                }

                for draw_cmd in &prepared.draws {
                    match draw_cmd {
                        SessionDrawCmd::SpriteBatch {
                            atlas_id,
                            instances,
                        } => {
                            let Some(atlas) = atlas_pool.atlases.get(*atlas_id) else {
                                continue;
                            };
                            if active_pipeline != ActivePipeline::Sprite {
                                pass.set_pipeline(&self.sprite_render_pipeline);
                                pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                                pass.set_index_buffer(
                                    self.index_buffer.slice(..),
                                    wgpu::IndexFormat::Uint16,
                                );
                                pass.set_bind_group(1, self.dynamic_storage.get_bind_group(), &[]);
                                pass.set_bind_group(
                                    2,
                                    self.camera_dynamic_uniform.get_bind_group(),
                                    &[cam_draw.uniform_offset],
                                );
                                active_pipeline = ActivePipeline::Sprite;
                            }
                            pass.set_bind_group(0, &atlas.bind_group, &[]);
                            pass.draw_indexed(0..6, 0, instances.clone());
                        }
                        SessionDrawCmd::Line(draw) => {
                            if active_pipeline != ActivePipeline::Line {
                                pass.set_pipeline(&self.line_render_pipeline);
                                pass.set_vertex_buffer(
                                    0,
                                    self.line_buffers.vertex_buffer.slice(..),
                                );
                                pass.set_index_buffer(
                                    self.line_buffers.index_buffer.slice(..),
                                    wgpu::IndexFormat::Uint32,
                                );
                                pass.set_bind_group(0, self.dynamic_storage.get_bind_group(), &[]);
                                pass.set_bind_group(
                                    1,
                                    self.camera_dynamic_uniform.get_bind_group(),
                                    &[cam_draw.uniform_offset],
                                );
                                active_pipeline = ActivePipeline::Line;
                            }
                            pass.draw_indexed(
                                draw.index_range.clone(),
                                draw.base_vertex,
                                draw.instance_index..draw.instance_index + 1,
                            );
                        }
                    }
                }
            }
        }

        // Draw the egui UI
        {
            // F4 debug overlay to show atlas pool
            static DEBUG_ATLAS_IDX: std::sync::Mutex<Option<usize>> = std::sync::Mutex::new(None);
            let mut debug_idx = DEBUG_ATLAS_IDX.lock().unwrap();
            let f4_pressed = game
                .get_shared_resource::<bones::KeyboardInputs>()
                .is_some_and(|k| {
                    k.key_events.iter().any(|e| {
                        e.key_code.option() == Some(bones::KeyCode::F4) && !e.button_state.pressed()
                    })
                });
            if f4_pressed {
                let n = atlas_pool.atlases.len();
                *debug_idx = match *debug_idx {
                    None if n > 0 => Some(0),
                    Some(i) if i + 1 < n => Some(i + 1),
                    _ => None,
                };
            }
            let temp_tex_id = debug_idx
                .and_then(|i| atlas_pool.atlases.get(i).map(|a| (i, a)))
                .map(|(i, atlas)| {
                    let tex_id = self.egui_renderer.renderer.register_native_texture(
                        &self.device,
                        &atlas.view,
                        wgpu::FilterMode::Nearest,
                    );
                    egui::Area::new(egui::Id::new("debug_atlas_overlay"))
                        .order(egui::Order::Debug)
                        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                        .show(self.egui_renderer.context(), |ui| {
                            egui::Frame::popup(ui.style())
                                .fill(egui::Color32::from_black_alpha(220))
                                .show(ui, |ui| {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "Atlas #{} ({}/{}) {}x{} [{} allocs] — F4 to cycle",
                                            atlas.id,
                                            i + 1,
                                            atlas_pool.atlases.len(),
                                            atlas.size.0,
                                            atlas.size.1,
                                            atlas.allocated
                                        ))
                                        .color(egui::Color32::WHITE)
                                        .strong(),
                                    );
                                    let max_h = ui.ctx().screen_rect().height() * 0.85;
                                    let aspect = atlas.size.0 as f32 / atlas.size.1.max(1) as f32;
                                    ui.image(egui::load::SizedTexture::new(
                                        tex_id,
                                        [max_h * aspect, max_h],
                                    ));
                                });
                        });
                    tex_id
                });

            let screen_descriptor = ScreenDescriptor {
                size_in_pixels: [self.size.width, self.size.height],
                pixels_per_point,
            };

            self.egui_renderer.end_frame_and_draw(
                &self.device,
                &self.queue,
                &mut encoder,
                &self.window,
                &texture_view,
                screen_descriptor,
            );

            if let Some(tex_id) = temp_tex_id {
                self.egui_renderer.renderer.free_texture(&tex_id);
            }
        }

        // Submit the command queue.
        self.queue.submit([encoder.finish()]);
        self.window.pre_present_notify();
        surface_texture.present();
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
}

impl Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

const VERTICES: &[Vertex] = &[
    // Top-left vertex
    Vertex {
        position: [-0.5, 0.5, 0.0],
        tex_coords: [0.0, 0.0],
    },
    // Bottom-left vertex
    Vertex {
        position: [-0.5, -0.5, 0.0],
        tex_coords: [0.0, 1.0],
    },
    // Bottom-right vertex
    Vertex {
        position: [0.5, -0.5, 0.0],
        tex_coords: [1.0, 1.0],
    },
    // Top-right vertex
    Vertex {
        position: [0.5, 0.5, 0.0],
        tex_coords: [1.0, 0.0],
    },
];

const INDICES: &[u16] = &[
    0, 1, 2, // first triangle
    0, 2, 3, // second triangle
];

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renderer_builder_configures_window_options() {
        let renderer = BonesWgpuRenderer::new(bones::Game::new())
            .window_title("Jumpy")
            .window_size(Vec2::new(1920.0, 1080.0))
            .min_window_size(Vec2::new(640.0, 360.0))
            .window_resizable(false)
            .fullscreen(true)
            .vsync(false);

        assert_eq!(renderer.window_title, "Jumpy");
        assert_eq!(renderer.window_size, Vec2::new(1920.0, 1080.0));
        assert_eq!(renderer.min_window_size, Some(Vec2::new(640.0, 360.0)));
        assert!(!renderer.window_resizable);
        assert!(renderer.fullscreen);
        assert_eq!(renderer.present_mode, wgpu::PresentMode::AutoNoVsync);
    }

    #[test]
    fn atlas_and_tile_indices_clamp_to_bounds() {
        let atlas = bones::Atlas {
            tile_size: Vec2::new(16.0, 16.0),
            columns: 4,
            rows: 3,
            ..Default::default()
        };
        let sprite = bones::AtlasSprite {
            index: 99,
            atlas: bones::Handle::default(),
            ..Default::default()
        };
        let uniform = AtlasSpriteUniform::from_atlas_sprite(
            &sprite,
            &atlas,
            &bones::Transform::default(),
            ([0.0, 0.0], [1.0, 1.0]),
            Vec2::new(64.0, 48.0),
        );
        assert_eq!(uniform.index, 11);

        let empty_atlas = bones::Atlas {
            columns: 0,
            rows: 0,
            ..atlas
        };
        let empty_uniform = AtlasSpriteUniform::from_atlas_sprite(
            &sprite,
            &empty_atlas,
            &bones::Transform::default(),
            ([0.0, 0.0], [1.0, 1.0]),
            Vec2::new(64.0, 48.0),
        );
        assert_eq!(empty_uniform.index, 0);
    }

    #[test]
    fn physical_key_extracts_scan_code() {
        let key = winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::KeyA);
        let scancode = extract_scan_code(&key);
        #[cfg(any(
            target_os = "windows",
            target_os = "macos",
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd"
        ))]
        assert!(scancode.option().is_some());
    }

    #[test]
    fn visible_sessions_sort_by_priority_before_first_step() {
        let mut game = bones::Game::new();
        game.sessions
            .create_with("overlay", |s: &mut bones::Session| {
                s.priority = 10;
            });
        game.sessions
            .create_with("world", |s: &mut bones::Session| {
                s.priority = 0;
            });
        game.sessions
            .create_with("hidden", |s: &mut bones::Session| {
                s.priority = 5;
                s.visible = false;
            });

        let sorted = sorted_visible_sessions(&game);
        assert_eq!(sorted, vec![bones::ustr("world"), bones::ustr("overlay")]);
    }

    #[test]
    fn screen_to_world_2d_respects_hidpi_window_camera() {
        let mut game = bones::Game::new();
        game.sessions
            .create_with("world", |s: &mut bones::Session| {
                let ent = s.world.resource_mut::<bones::Entities>().create();
                s.world.component_mut::<bones::Camera>().insert(
                    ent,
                    bones::Camera {
                        size: bones::CameraSize::Window {
                            pixels_per_unit: 1.0,
                        },
                        ..Default::default()
                    },
                );
                s.world
                    .component_mut::<bones::Transform>()
                    .insert(ent, bones::Transform::default());
            });

        // Bottom-right physical pixel (2560, 1440) at 2x DPI => logical (1280, 720) => (+640, -360).
        let world = screen_to_world_2d(
            &game,
            Vec2::new(2560.0, 1440.0),
            UVec2::new(2560, 1440),
            2.0,
        )
        .unwrap();
        assert!((world.x - 640.0).abs() < 1e-3);
        assert!((world.y + 360.0).abs() < 1e-3);
    }

    #[test]
    fn wgsl_shaders_and_pipeline_layouts_validate() {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let Some(adapter) = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .block_on()
        else {
            // Headless environment without Vulkan/GL/software adapter; skip gracefully.
            return;
        };
        let required_limits = wgpu::Limits::default().using_resolution(adapter.limits());
        let Ok((device, _queue)) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_limits,
                    ..Default::default()
                },
                None,
            )
            .block_on()
        else {
            return;
        };

        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("test_texture_layout"),
        });
        let storage_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("test_storage_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let camera_entry_size =
            std::num::NonZeroU64::new(std::mem::size_of::<CameraTransform>() as u64);
        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("test_camera_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: true,
                    min_binding_size: camera_entry_size,
                },
                count: None,
            }],
        });

        let _pipelines = State::create_pipelines(
            &device,
            &texture_layout,
            &storage_layout,
            &camera_layout,
            wgpu::TextureFormat::Bgra8UnormSrgb,
        );
    }
}
