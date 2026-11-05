use bevy_tasks::{IoTaskPool, TaskPool};
use convert::IntoBones;
use image::RgbaImage;
use pollster::FutureExt;
use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};
use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Fullscreen, Window, WindowId},
};

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

impl WgpuDevice {
    fn get(&self) -> &wgpu::Device {
        self.0.as_ref().unwrap()
    }
}

#[derive(bones_schema::HasSchema, Default, Clone)]
#[repr(C)]
#[schema(opaque)]
struct WgpuQueue(Option<Arc<wgpu::Queue>>);

impl WgpuQueue {
    fn get(&self) -> &wgpu::Queue {
        self.0.as_ref().unwrap()
    }
}

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

    pub fn run(mut self) {
        //Start wgpu
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let (adapter, device, queue) = async {
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions::default())
                .await
                .unwrap();

            let (device, queue) = adapter
                .request_device(
                    &wgpu::DeviceDescriptor::default(),
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
        self.game
            .insert_shared_resource(WgpuQueue(Some(queue.clone())));
        self.game.insert_shared_resource(PixelArt(self.pixel_art));
        self.game
            .insert_shared_resource(LoadingContext(self.custom_load_progress));
        self.game.insert_shared_resource(Cameras(Vec::new()));

        //Deal with asset server
        IoTaskPool::init(TaskPool::default);
        if let Some(mut asset_server) = self.game.get_shared_resource_mut::<bones::AssetServer>() {
            asset_server.set_game_version(self.game_version.clone());
            asset_server.set_io(asset_io(&self.asset_dir, &self.packs_dir));

            if self.preload {
                // Load assets
                let s = asset_server.clone();
                println!("Loading Assets...");

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
        self.game.insert_shared_resource(bones::ExitBones(false));

        // Insert empty inputs that will be updated by the `insert_bones_input` system later.
        self.game.init_shared_resource::<bones::KeyboardInputs>();
        self.game.init_shared_resource::<bones::MouseInputs>();
        self.game.init_shared_resource::<bones::GamepadInputs>();
        self.game.init_shared_resource::<bones::GamepadsRumble>();
        self.game
            .init_shared_resource::<bones::MouseScreenPosition>();
        self.game
            .init_shared_resource::<bones::MouseWorldPosition>();
        self.game.init_shared_resource::<bones::Window>();
        self.game
            .init_shared_resource::<bones::debug::FrameDiagState>();

        // wgpu uses `log` for all of our logging, so we initialize a logger with the `env_logger` crate.
        let _ = env_logger::try_init();

        let event_loop = EventLoop::builder().build().unwrap();

        // When the current loop iteration finishes, immediately begin a new
        // iteration regardless of whether or not new events are available to
        // process.
        event_loop.set_control_flow(ControlFlow::Poll);

        let bind_group_clone = texture_layout.clone();
        let device_clone = device.clone();

        let mut app = App {
            state: None,
            instance,
            adapter,
            device,
            queue,
            texture_layout,
            storage_layout: dynamic_storage.layout.clone(),
            dynamic_storage: Some(dynamic_storage),
            camera_layout: camera_dynamic_uniform.layout.clone(),
            camera_dynamic_uniform: Some(camera_dynamic_uniform),
            game: self.game,
            vertex_buffer,
            index_buffer,
            last_frame_instant: None,
            frame_time_history: VecDeque::with_capacity(20),
            rumble_manager: rumble::RumbleManager::default(),
            preload_assets: self.preload,
            atlas_pool: atlas_pool::AtlasPool::new(
                &device_clone,
                &bind_group_clone,
                (4096, 4096),
                8,
                self.pixel_art,
            ),
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

    let asset_server = game.get_shared_resource::<bones::AssetServer>();

    if let Some(bones_assets) = asset_server {
        update_egui_fonts(ctx, &bones_assets);

        // Insert the bones egui textures
        ctx.data_mut(|map| {
            map.insert_temp(
                egui::Id::NULL,
                game.shared_resource_cell::<bones::EguiTextures>().unwrap(),
            );
        });
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

//TODO Implement proper Drop for the app
struct App {
    state: Option<State>,
    atlas_pool: atlas_pool::AtlasPool,
    preload_assets: bool,
    instance: Arc<wgpu::Instance>,
    adapter: Arc<wgpu::Adapter>,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    texture_layout: Arc<wgpu::BindGroupLayout>,
    storage_layout: Arc<wgpu::BindGroupLayout>,
    dynamic_storage: Option<DynamicBuffer>,
    camera_layout: Arc<wgpu::BindGroupLayout>,
    camera_dynamic_uniform: Option<DynamicBuffer>,
    vertex_buffer: Arc<wgpu::Buffer>,
    index_buffer: Arc<wgpu::Buffer>,
    game: bones::Game,
    last_frame_instant: Option<Instant>,
    frame_time_history: VecDeque<f64>,
    rumble_manager: rumble::RumbleManager,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Create window object
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap(),
        );

        if let Some(state) = &mut self.state {
            state.window = window.clone();
            state.size = window.inner_size();

            state.surface = self.instance.create_surface(window.clone()).unwrap();
            let cap = state.surface.get_capabilities(&self.adapter);
            state.surface_format = cap
                .formats
                .iter()
                .copied()
                .find(|f| f.is_srgb())
                .unwrap_or(cap.formats[0]);

            state.configure_surface();
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
                self.dynamic_storage.take().unwrap_or(DynamicBuffer::new(
                    &self.device,
                    self.storage_layout.clone(),
                    1024,
                    wgpu::BufferUsages::STORAGE,
                )),
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
            );

            setup_egui(&mut self.game, &state.egui_renderer.context().clone());

            // Initialize/update the Bones window resource now that we have a real window.
            if let Some(mut bones_window) = self.game.get_shared_resource_mut::<bones::Window>() {
                let logical: winit::dpi::LogicalSize<f32> =
                    window.inner_size().to_logical(window.scale_factor());
                bones_window.size = Vec2::new(logical.width, logical.height);
                bones_window.focused = window.has_focus();
            }

            self.state = Some(state);
        }

        window.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(state) = self.state.as_mut() else {
            // State may already be dropped during shutdown.
            return;
        };

        // Egui input handling
        state
            .egui_renderer
            .handle_input(&state.get_window(), &event);

        match event {
            WindowEvent::CloseRequested => {
                self.state.take();
                event_loop.exit();
                std::process::exit(0);
            }
            WindowEvent::RedrawRequested => {
                if self.game.shared_resource::<bones::ExitBones>().0 {
                    self.state.take();
                    event_loop.exit();
                    std::process::exit(0);
                }

                // Update frame time diagnostics (`FrameDiagState`).
                let now = Instant::now();
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
                let scale_factor = state.window.scale_factor() as f32;
                let screen_pos = self
                    .game
                    .get_shared_resource::<bones::MouseScreenPosition>()
                    .and_then(|p| p.0);
                let world_pos = screen_pos.and_then(|logical_sp| {
                    screen_to_world_2d(
                        &self.game,
                        logical_sp * scale_factor,
                        Vec2::new(state.size.width as f32, state.size.height as f32),
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
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let ev = match event.physical_key {
                    winit::keyboard::PhysicalKey::Code(code) => bones::KeyboardEvent {
                        scan_code: bones::Unset,
                        key_code: code.into_bones(),
                        button_state: event.state.into_bones(),
                    },
                    winit::keyboard::PhysicalKey::Unidentified(native_key_code) => {
                        let scan_code = match native_key_code {
                            winit::keyboard::NativeKeyCode::Android(u) => bones::Set(u),
                            winit::keyboard::NativeKeyCode::MacOS(u) => bones::Set(u as u32),
                            winit::keyboard::NativeKeyCode::Windows(u) => bones::Set(u as u32),
                            winit::keyboard::NativeKeyCode::Xkb(u) => bones::Set(u),
                            winit::keyboard::NativeKeyCode::Unidentified => bones::Unset,
                        };
                        bones::KeyboardEvent {
                            scan_code,
                            key_code: bones::Unset,
                            button_state: event.state.into_bones(),
                        }
                    }
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
                let ev = bones::MouseButtonEvent {
                    button: button.into_bones(),
                    state: state.into_bones(),
                };
                if let Some(mut mouse_inputs) =
                    self.game.get_shared_resource_mut::<bones::MouseInputs>()
                {
                    mouse_inputs.button_events.push(ev);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let scale_factor = state.window.scale_factor() as f32;
                let physical_sp = Vec2::new(position.x as f32, position.y as f32);
                let logical_sp = physical_sp / scale_factor;
                let world_pos = screen_to_world_2d(
                    &self.game,
                    physical_sp,
                    Vec2::new(state.size.width as f32, state.size.height as f32),
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
            let delta = Vec2::new(delta.0 as f32, delta.1 as f32);
            self.game
                .shared_resource_mut::<bones::MouseInputs>()
                .movement += delta;
        }
    }
}

struct State {
    window: Arc<Window>,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface<'static>,
    surface_format: wgpu::TextureFormat,
    sprite_render_pipeline: wgpu::RenderPipeline,
    line_render_pipeline: wgpu::RenderPipeline,
    line_buffers: line::LineBuffers,
    vertex_buffer: Arc<wgpu::Buffer>,
    index_buffer: Arc<wgpu::Buffer>,
    dynamic_storage: DynamicBuffer,
    camera_dynamic_uniform: DynamicBuffer,
    egui_renderer: EguiRenderer,
    egui_scale_factor: f32,
    egui_fonts_loaded: bool,
    preload_assets: bool,
    /// When preloading assets, we pause all sessions to avoid running user startup systems before
    /// assets exist (which would create default handles with no CID and panic on access).
    saved_session_states: Option<Vec<(bones::Ustr, bool, bool)>>,
}

impl State {
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
        let view_format = surface_format.add_srgb_suffix();

        // Configure surface for the first time (if non-zero size)
        if size.width > 0 && size.height > 0 {
            let surface_config = wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: surface_format,
                // Request compatibility with the sRGB-format texture view we‘re going to create later.
                view_formats: vec![view_format],
                alpha_mode: wgpu::CompositeAlphaMode::Auto,
                width: size.width,
                height: size.height,
                desired_maximum_frame_latency: 2,
                present_mode: wgpu::PresentMode::AutoVsync,
            };
            surface.configure(&device, &surface_config);
        }

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("atlas_sprite.wgsl").into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &texture_layout,
                    &dynamic_storage.layout,
                    &camera_dynamic_uniform.layout,
                ],
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
                // If the pipeline will be used with a multiview render pass, this
                // indicates how many array layers the attachments will have.
                multiview: None,
                // Useful for optimizing shader compilation on Android
                cache: None,
            });

        let egui_renderer = EguiRenderer::new(&device, view_format, None, 1, &window);

        let line_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Line Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("line.wgsl").into()),
        });

        let line_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Line Render Pipeline Layout"),
                bind_group_layouts: &[&dynamic_storage.layout, &camera_dynamic_uniform.layout],
                push_constant_ranges: &[],
            });

        let line_render_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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

        let line_buffers = line::LineBuffers::new(&device);

        State {
            window,
            device: device.clone(),
            queue,
            size,
            surface,
            surface_format,
            sprite_render_pipeline,
            line_render_pipeline,
            line_buffers,
            egui_renderer,
            dynamic_storage,
            camera_dynamic_uniform,
            egui_scale_factor: 1.0,
            egui_fonts_loaded: false,
            preload_assets,
            vertex_buffer,
            index_buffer,
            saved_session_states: None,
        }
    }

    fn get_window(&self) -> Arc<Window> {
        self.window.clone()
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
            present_mode: wgpu::PresentMode::AutoVsync,
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

    fn render(&mut self, game: &mut bones::Game, atlas_pool: &mut atlas_pool::AtlasPool) {
        if self.size.width == 0 || self.size.height == 0 {
            return;
        }

        // Create the command encoder.
        let mut encoder = self.device.create_command_encoder(&Default::default());

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

        // Begin egui frame (and let the game mutate RawInput via EguiInputHook).
        // Many games (including Jumpy) also piggyback on this hook to collect input for menus.
        let mut raw_input = self.egui_renderer.take_raw_input(&self.window);
        let input_hook = game
            .get_shared_resource::<bones::EguiInputHook>()
            .map(|h| h.0.clone());
        if let Some(input_hook) = input_hook {
            (input_hook)(game, &mut raw_input);
        }
        self.egui_renderer.begin_frame_with_raw_input(raw_input);

        // Keep the Bones window resource in sync with the actual winit window size.
        if let Some(mut bones_window) = game.get_shared_resource_mut::<bones::Window>() {
            let logical: winit::dpi::LogicalSize<f32> = self
                .window
                .inner_size()
                .to_logical(self.window.scale_factor());
            bones_window.size = Vec2::new(logical.width, logical.height);
            bones_window.focused = self.window.has_focus();
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

            // Render the loading UI (and do NOT step the game).
            asset_load_status(game);

            let surface_texture = self
                .surface
                .get_current_texture()
                .expect("failed to acquire next swapchain texture");
            let texture_view = surface_texture
                .texture
                .create_view(&wgpu::TextureViewDescriptor {
                    format: Some(self.surface_format.add_srgb_suffix()),
                    ..Default::default()
                });

            let screen_descriptor = ScreenDescriptor {
                size_in_pixels: [self.size.width, self.size.height],
                pixels_per_point: self.window.as_ref().scale_factor() as f32
                    * self.egui_scale_factor,
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

        // Step bones normally once assets are ready (or if preload is disabled).
        game.step(Instant::now());

        // Sync egui scale factor from active sessions' EguiSettings resource.
        for session_name in &game.sorted_session_keys {
            if let Some(session) = game.sessions.get(*session_name) {
                if let Some(settings) = session.world.get_resource::<bones::EguiSettings>() {
                    self.egui_scale_factor = settings.scale as f32;
                }
            }
        }

        // Apply fullscreen changes requested by the game.
        if let Some(bones_window) = game.get_shared_resource::<bones::Window>() {
            let want_fullscreen = bones_window.fullscreen;
            let is_fullscreen = self.window.fullscreen().is_some();

            if want_fullscreen && !is_fullscreen {
                self.window
                    .set_fullscreen(Some(Fullscreen::Borderless(self.window.current_monitor())));
            } else if !want_fullscreen && is_fullscreen {
                self.window.set_fullscreen(None);
            }
        }

        update_atlas_pool(game, atlas_pool);
        sort_sprites(game);
        update_cameras_uniform(
            game,
            &mut self.camera_dynamic_uniform,
            IVec2::new(self.size.width as i32, self.size.height as i32),
        );
        update_uniforms(game, &mut self.dynamic_storage, &mut self.line_buffers);

        let cameras_sorted = game.shared_resource::<Cameras>();

        // Create texture view
        let surface_texture = self
            .surface
            .get_current_texture()
            .expect("failed to acquire next swapchain texture");
        let texture_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                // Without add_srgb_suffix() the image we will be working with
                // might not be "gamma correct".
                format: Some(self.surface_format.add_srgb_suffix()),
                ..Default::default()
            });

        // Clear the frame once up-front.
        let mut frame_clear_color = wgpu::Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        };
        for session_name in &game.sorted_session_keys {
            if let Some(session) = game.sessions.get(*session_name) {
                if !session.visible {
                    continue;
                }
                if let Some(clear_color) = session.world.get_resource::<bones::ClearColor>() {
                    let rgba = clear_color.0.as_rgba_f64();
                    frame_clear_color = wgpu::Color {
                        r: rgba[0],
                        g: rgba[1],
                        b: rgba[2],
                        a: rgba[3],
                    };
                    break;
                }
            }
        }

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

        let mut global_camera_idx = 0u32;
        for (session_name, camera_vec) in cameras_sorted.0.iter() {
            let session = game.sessions.get(*session_name).unwrap();

            let sprite_lists = session.world.resource::<SpriteLists>();
            let cameras = session.world.component::<bones::Camera>();
            let clear_color = session.world.get_resource::<bones::ClearColor>();

            if log::log_enabled!(log::Level::Debug) {
                log::debug!(
                    "Render session {:?}: cameras={}, sprites={}, batches={}, tiles={}",
                    session_name,
                    camera_vec.len(),
                    sprite_lists.sprite_list.len(),
                    sprite_lists.batches.len(),
                    sprite_lists.tile_layer.len(),
                );
            }

            for (camera_ent, camera_size) in camera_vec {
                let camera = cameras.get(*camera_ent).unwrap();
                let camera_offset =
                    global_camera_idx * std::mem::size_of::<CameraTransform>() as u32;
                global_camera_idx += 1;

                let camera_bg_color = if camera.draw_background_color {
                    let color: bones_framework::render::prelude::Color =
                        match (camera.background_color.option(), clear_color.as_deref()) {
                            (Some(color), _) => color,
                            (None, Some(color)) => color.0,
                            (None, None) => bones::Color::BLACK,
                        };
                    let rgba = color.as_rgba_f64();
                    Some(wgpu::Color {
                        r: rgba[0],
                        g: rgba[1],
                        b: rgba[2],
                        a: rgba[3],
                    })
                } else {
                    None
                };

                // If the camera has no custom sub-window viewport and wants a background color,
                // we can clear the render target directly via LoadOp::Clear.
                let load_op = match (camera.viewport.option(), camera_bg_color) {
                    (None, Some(color)) => wgpu::LoadOp::Clear(color),
                    _ => wgpu::LoadOp::Load,
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

                if let Some(viewport) = camera.viewport.option() {
                    pass.set_viewport(
                        viewport.position.x as f32,
                        viewport.position.y as f32,
                        camera_size.x,
                        camera_size.y,
                        viewport.depth_min,
                        viewport.depth_max,
                    );

                    // Clear sub-window viewport if camera.draw_background_color is true
                    if let Some(bg_draw) = self
                        .line_buffers
                        .camera_bg_draws
                        .get(&(*session_name, *camera_ent))
                    {
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
                            &[camera_offset],
                        );
                        pass.draw_indexed(
                            bg_draw.index_range.clone(),
                            bg_draw.base_vertex,
                            bg_draw.instance_index..bg_draw.instance_index + 1,
                        );
                    }
                }

                // === SPRITE PASS ===
                if !sprite_lists.batches.is_empty() {
                    pass.set_pipeline(&self.sprite_render_pipeline);
                    pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                    pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                    pass.set_bind_group(1, self.dynamic_storage.get_bind_group(), &[]);
                    pass.set_bind_group(
                        2,
                        self.camera_dynamic_uniform.get_bind_group(),
                        &[camera_offset],
                    );

                    for (atlas_id, instance_range) in &sprite_lists.batches {
                        let atlas_bg = &atlas_pool.atlases[*atlas_id].bind_group;
                        pass.set_bind_group(0, atlas_bg, &[]);
                        pass.draw_indexed(0..6, 0, instance_range.clone());
                    }
                }

                // === LINE (Path2d) PASS ===
                if let Some(draws) = self.line_buffers.session_draws.get(session_name) {
                    if !draws.is_empty() {
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
                            &[camera_offset],
                        );

                        for draw in draws {
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
                                            "Atlas #{} ({}/{}) [{} allocs] — F4 to cycle",
                                            atlas.id,
                                            i + 1,
                                            atlas_pool.atlases.len(),
                                            atlas.allocated
                                        ))
                                        .color(egui::Color32::WHITE)
                                        .strong(),
                                    );
                                    let side = ui.ctx().screen_rect().height() * 0.85;
                                    ui.image(egui::load::SizedTexture::new(tex_id, [side, side]));
                                });
                        });
                    tex_id
                });

            let screen_descriptor = ScreenDescriptor {
                size_in_pixels: [self.size.width, self.size.height],
                pixels_per_point: self.window.as_ref().scale_factor() as f32
                    * self.egui_scale_factor,
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
