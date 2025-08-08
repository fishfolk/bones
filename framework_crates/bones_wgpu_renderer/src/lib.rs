use bevy_tasks::{IoTaskPool, TaskPool};
use convert::IntoBones;
use image::RgbaImage;
use pollster::FutureExt;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};
use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
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
mod sprite;
mod storage;
mod texture;
mod texture_file;
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
type LoadingFunction = Box<dyn FnMut(&bones::AssetServer, &egui::Context) + Sync + Send + 'static>;

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

        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("camera_bind_group_layout"),
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
        let camera_dynamic_uniform =
            DynamicBuffer::new(&device, camera_layout, 1024, wgpu::BufferUsages::STORAGE);

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
        if let Some(mut asset_server) = self.game.shared_resource_mut::<bones::AssetServer>() {
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
        self.game.insert_shared_resource(bones::ExitBones(false));

        // Insert empty inputs that will be updated by the `insert_bones_input` system later.
        self.game.init_shared_resource::<bones::KeyboardInputs>();
        self.game.init_shared_resource::<bones::MouseInputs>();
        self.game.init_shared_resource::<bones::GamepadInputs>();
        self.game
            .init_shared_resource::<bones::MouseScreenPosition>();
        self.game
            .init_shared_resource::<bones::MouseWorldPosition>();

        //Insert needed systems
        self.game.systems.add_startup_system(load_egui_textures);
        self.game.systems.add_startup_system(asset_load_status);

        // wgpu uses `log` for all of our logging, so we initialize a logger with the `env_logger` crate.
        env_logger::init();

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
            _now: Instant::now(),
            atlas_pool: atlas_pool::AtlasPool::new(
                &device_clone,
                &bind_group_clone,
                (4096, 4096),
                8,
                self.pixel_art,
            ),
        };
        event_loop.run_app(&mut app).unwrap();

        app.device.poll(wgpu::Maintain::Wait);
    }
}

fn asset_load_status(game: &mut bones::Game) {
    let asset_server = game.shared_resource::<bones::AssetServer>().unwrap();
    let ctx = game.shared_resource::<bones::EguiCtx>().unwrap();
    let mut function = game.shared_resource_mut::<LoadingContext>().unwrap();

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

    let asset_server = game.shared_resource::<bones::AssetServer>();

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

//TODO Handle asset changes
fn load_egui_textures(game: &mut bones::Game) {
    let asset_server = game.shared_resource::<bones::AssetServer>().unwrap();
    let ctx = game.shared_resource::<bones::EguiCtx>().unwrap();
    let mut egui_textures = game.shared_resource_mut::<bones::EguiTextures>().unwrap();
    let pixel_art = game.shared_resource::<PixelArt>().unwrap();

    for entry in asset_server.store.asset_ids.iter() {
        let id = entry.key().typed();
        if egui_textures.contains_key(&id) {
            // we already loaded this one
            continue;
        }

        let asset = asset_server.store.assets.get_mut(entry.value()).unwrap();
        if let Ok(bones::Image::Data(data)) = asset.data.try_cast_ref::<bones::Image>() {
            let rgba: RgbaImage = data.to_rgba8();
            let (w, h) = (rgba.width() as usize, rgba.height() as usize);
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
        }
    }
}

//TODO Implement proper Drop for the app
struct App {
    state: Option<State>,
    atlas_pool: atlas_pool::AtlasPool,
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
    _now: Instant,
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
            state.surface_format = cap.formats[0];

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
                self.camera_dynamic_uniform
                    .take()
                    .unwrap_or(DynamicBuffer::new(
                        &self.device,
                        self.camera_layout.clone(),
                        1024,
                        wgpu::BufferUsages::STORAGE,
                    )),
            );

            setup_egui(&mut self.game, &state.egui_renderer.context().clone());

            self.state = Some(state);
        }

        window.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let state = self.state.as_mut().unwrap();
        // TODO: investigate possible ways to avoid allocating vectors every frame for event lists.
        // TODO: Maybe add some multithreading for the diferent fors in the function?
        let mut keyboard_inputs = bones::KeyboardInputs::default();
        let mut wheel_events = Vec::new();
        let mut button_events = Vec::new();

        // Egui input handling
        state
            .egui_renderer
            .handle_input(&state.get_window(), &event);

        match event {
            WindowEvent::CloseRequested => {
                //Close window
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                //println!("{}", self.now.elapsed().as_secs_f32());
                //self.now = Instant::now();

                if self.game.shared_resource::<bones::ExitBones>().unwrap().0 {
                    event_loop.exit();
                }

                state.render(&mut self.game, &mut self.atlas_pool);
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
                        key_code: bones::Set(code.into_bones()),
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
                keyboard_inputs.key_events.push(ev);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let ev: bones::MouseScrollEvent = delta.into_bones();
                wheel_events.push(ev);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let ev = bones::MouseButtonEvent {
                    button: button.into_bones(),
                    state: state.into_bones(),
                };
                button_events.push(ev);
            }
            WindowEvent::CursorMoved { position, .. } => {
                let screen_pos = Some(Vec2::new(position.x as f32, position.y as f32));
                self.game
                    .insert_shared_resource(bones::MouseScreenPosition(screen_pos));
            }
            WindowEvent::CursorLeft { .. } => {
                self.game
                    .insert_shared_resource(bones::MouseScreenPosition(None));
            }
            _ => (),
        }

        // Add the game inputs
        //TODO: Add world position
        //self.game.insert_shared_resource(MouseWorldPosition(world_pos));
        self.game
            .shared_resource_mut::<bones::MouseInputs>()
            .unwrap()
            .wheel_events = wheel_events;
        self.game
            .shared_resource_mut::<bones::MouseInputs>()
            .unwrap()
            .button_events = button_events;
        self.game.insert_shared_resource(keyboard_inputs);
        self.game.insert_shared_resource(process_gamepad_events());
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        let mut movement = Vec2::default();

        if let winit::event::DeviceEvent::MouseMotion { delta } = event {
            let delta = Vec2::new(delta.0 as f32, delta.1 as f32);
            movement += delta;
        };

        self.game
            .shared_resource_mut::<bones::MouseInputs>()
            .unwrap()
            .movement = movement;
    }
}

struct State {
    window: Arc<Window>,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface<'static>,
    surface_format: wgpu::TextureFormat,
    opaque_render_pipeline: wgpu::RenderPipeline,
    transparent_render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: Arc<wgpu::Buffer>,
    index_buffer: Arc<wgpu::Buffer>,
    dynamic_storage: DynamicBuffer,
    camera_dynamic_uniform: DynamicBuffer,
    egui_renderer: EguiRenderer,
    egui_scale_factor: f32,
}

impl State {
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
    ) -> Self {
        let size = window.inner_size();
        let surface = instance.create_surface(window.clone()).unwrap();
        let cap = surface.get_capabilities(adapter);
        let surface_format = cap.formats[0];

        // Configure surface for the first time
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            // Request compatibility with the sRGB-format texture view we‘re going to create later.
            view_formats: vec![surface_format.add_srgb_suffix()],
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            width: size.width,
            height: size.height,
            desired_maximum_frame_latency: 2,
            present_mode: wgpu::PresentMode::AutoVsync,
        };
        surface.configure(&device, &surface_config);

        let surface_caps = surface.get_capabilities(adapter);
        // This accounts only for Srgb surfaces
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

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

        let opaque_render_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Render Pipeline"),
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
                        format: config.format,
                        blend: None, //For opaque
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    // Setting this to anything other than Fill requires Features::POLYGON_MODE_LINE
                    // or Features::POLYGON_MODE_POINT
                    polygon_mode: wgpu::PolygonMode::Fill,
                    // Requires Features::DEPTH_CLIP_CONTROL
                    unclipped_depth: false,
                    // Requires Features::CONSERVATIVE_RASTERIZATION
                    conservative: false,
                },
                depth_stencil: None,

                /*Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    depth_write_enabled: true, //For opaque
                    depth_compare: wgpu::CompareFunction::LessEqual,
                    stencil: Default::default(),
                    bias: Default::default(),
                }),*/
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

        let transparent_render_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Render Pipeline"),
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
                        format: config.format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING), //For transparent
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    // Setting this to anything other than Fill requires Features::POLYGON_MODE_LINE
                    // or Features::POLYGON_MODE_POINT
                    polygon_mode: wgpu::PolygonMode::Fill,
                    // Requires Features::DEPTH_CLIP_CONTROL
                    unclipped_depth: false,
                    // Requires Features::CONSERVATIVE_RASTERIZATION
                    conservative: false,
                },
                depth_stencil: None,

                /*Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    depth_write_enabled: false, //For transparent
                    depth_compare: wgpu::CompareFunction::LessEqual,
                    stencil: Default::default(),
                    bias: Default::default(),
                }),*/
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

        let egui_renderer = EguiRenderer::new(&device, surface_config.format, None, 1, &window);

        State {
            window,
            device: device.clone(),
            queue,
            size,
            surface,
            surface_format,
            opaque_render_pipeline,
            transparent_render_pipeline,
            egui_renderer,
            dynamic_storage,
            camera_dynamic_uniform,
            egui_scale_factor: 1.0,
            vertex_buffer,
            index_buffer,
        }
    }

    fn get_window(&self) -> Arc<Window> {
        self.window.clone()
    }

    fn configure_surface(&self) {
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
        self.size = new_size;

        // reconfigure the surface
        self.configure_surface();
    }

    fn render(&mut self, game: &mut bones::Game, atlas_pool: &mut atlas_pool::AtlasPool) {
        // Create the command encoder.
        let mut encoder = self.device.create_command_encoder(&Default::default());

        //Run needed egui related function, needs to run before step
        self.egui_renderer.begin_frame(&self.window);

        //Step bones
        game.step(Instant::now());

        update_atlas_pool(game, atlas_pool);
        sort_sprites(game);
        update_uniforms(game, &mut self.dynamic_storage);
        update_cameras_uniform(
            game,
            &mut self.camera_dynamic_uniform,
            IVec2::new(self.size.width as i32, self.size.height as i32),
        );

        let cameras_sorted = game.shared_resource::<Cameras>().unwrap();

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

        let mut camera_index = 0;
        for (session_name, camera_vec) in cameras_sorted.0.iter() {
            let session = game.sessions.get(*session_name).unwrap();

            let sprite_lists = session.world.resource::<SpriteLists>();
            let atlas_handles = session.world.component::<AtlasPoolHandle>();
            let cameras = session.world.component::<bones::Camera>();

            for (camera_ent, camera_size) in camera_vec {
                // Create render passes for each camera
                let camera = cameras.get(*camera_ent).unwrap();

                // Set the camera for the sprites
                let n = self.dynamic_storage.capacity / size_of::<AtlasSpriteUniform>() as u64;
                for i in 0..n {
                    self.queue.write_buffer(
                        &self.dynamic_storage.buffer,
                        4 + i * size_of::<AtlasSpriteUniform>() as u64,
                        bytemuck::cast_slice(&[(camera_index as u32)]),
                    );
                }

                let clear_color = session.world.get_resource::<bones::ClearColor>();

                let load = if camera.draw_background_color {
                    let color: bones_framework::render::prelude::Color =
                        match (camera.background_color.option(), clear_color) {
                            (Some(color), _) => color,
                            (None, Some(color)) => color.0,
                            (None, None) => bones::Color::BLACK,
                        };
                    let color = color.as_rgba_f64();

                    let color = wgpu::Color {
                        r: color[0],
                        g: color[1],
                        b: color[2],
                        a: color[3],
                    };

                    wgpu::LoadOp::Clear(color)
                } else {
                    wgpu::LoadOp::Load
                };

                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &texture_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
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
                }

                // set the quad vertex buffer (slot 0)
                pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                // Before your draw loops, once:
                pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                pass.set_bind_group(1, &*self.dynamic_storage.get_bind_group(), &[]);
                pass.set_bind_group(2, &*self.camera_dynamic_uniform.get_bind_group(), &[]);

                // === OPAQUE PASS ===
                pass.set_pipeline(&self.opaque_render_pipeline);

                // Debug bind: use atlas 0 and instance 0 for a single test quad

                //pass.set_bind_group(0, &atlas_pool.atlases[0].bind_group, &[]);
                //pass.set_bind_group(1, self.dynamic_storage.get_bind_group(), &[]);
                //pass.draw_indexed(0..6, 0, 0..1); // should draw the first sprite once

                let opaque_list = &sprite_lists.opaque_list;
                let transparent_list = &sprite_lists.transparent_list;
                let index_of = &sprite_lists.index_of;

                // iterate through opaque_list and batch by atlas_id
                let mut i = 0;
                while i < opaque_list.len() {
                    let atlas_pool_handle =
                        atlas_handles.get(opaque_list[i]).unwrap_or_else(|| {
                            atlas_handles
                                .get(sprite_lists.tile_layer.get(&opaque_list[i]).unwrap().0)
                                .unwrap()
                        });
                    let current_atlas = atlas_pool_handle.atlas_id;

                    // find how many consecutive entries share this atlas
                    let start = i;
                    while i < opaque_list.len()
                        && atlas_handles
                            .get(opaque_list[i])
                            .unwrap_or_else(|| {
                                atlas_handles
                                    .get(sprite_lists.tile_layer.get(&opaque_list[i]).unwrap().0)
                                    .unwrap()
                            })
                            .atlas_id
                            == current_atlas
                    {
                        i += 1;
                    }
                    let count = (i - start) as u32;
                    let first_instance = index_of[&opaque_list[start]];

                    // bind this atlas’s bind group (group 0)
                    let atlas_bg = &atlas_pool.atlases[current_atlas].bind_group;
                    pass.set_bind_group(0, atlas_bg, &[]);

                    // draw a quad instanced `count` times, indexing into your storage arrays
                    pass.draw_indexed(0..6, 0, first_instance..first_instance + count);
                }

                // === TRANSPARENT PASS ===
                pass.set_pipeline(&self.transparent_render_pipeline);

                let mut j = 0;
                while j < transparent_list.len() {
                    let atlas_pool_handle = atlas_handles.get(transparent_list[i]).unwrap_or(
                        atlas_handles
                            .get(sprite_lists.tile_layer.get(&transparent_list[i]).unwrap().0)
                            .unwrap(),
                    );
                    let current_atlas = atlas_pool_handle.atlas_id;

                    let start = j;
                    while j < transparent_list.len()
                        && atlas_handles
                            .get(transparent_list[i])
                            .unwrap_or(
                                atlas_handles
                                    .get(
                                        sprite_lists
                                            .tile_layer
                                            .get(&transparent_list[i])
                                            .unwrap()
                                            .0,
                                    )
                                    .unwrap(),
                            )
                            .atlas_id
                            == current_atlas
                    {
                        j += 1;
                    }
                    let count = (j - start) as u32;
                    let first_instance = index_of[&transparent_list[start]];

                    let atlas_bg = &atlas_pool.atlases[current_atlas].bind_group;
                    pass.set_bind_group(0, atlas_bg, &[]);

                    pass.draw_indexed(0..6, 0, first_instance..first_instance + count);
                }
                camera_index += 1;
            }
        }

        // Draw the egui UI
        {
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
        }

        // Submit the command queue.
        self.queue.submit([encoder.finish()]);
        self.window.pre_present_notify();
        surface_texture.present();

        /*Write atlas pool textures to png files for debugging
        if !time_since_start().as_secs() < 10 {
            for atlas in atlas_pool.atlases.iter() {
                let path = String::from("atlas_") + &atlas.id.to_string() + ".png";

                let result = crate::texture_file::dump_texture_to_png(
                    &self.device,
                    &self.queue,
                    &atlas.texture,
                    (atlas.texture.width(), atlas.texture.height()),
                    std::path::Path::new(&path),
                );
                result.unwrap();
            }
            std::process::exit(1);
        }*/
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

const VERTICES_FULL: &[Vertex] = &[
    // Top-left vertex
    Vertex {
        position: [-1.0, 1.0, 0.0],
        tex_coords: [0.0, 0.0],
    },
    // Bottom-left vertex
    Vertex {
        position: [-1.0, -1.0, 0.0],
        tex_coords: [0.0, 1.0],
    },
    // Bottom-right vertex
    Vertex {
        position: [1.0, -1.0, 0.0],
        tex_coords: [1.0, 1.0],
    },
    // Top-right vertex
    Vertex {
        position: [1.0, 1.0, 0.0],
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
