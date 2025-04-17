use bevy_tasks::{IoTaskPool, TaskPool};
use convert::IntoBones;
use crossbeam_channel::{unbounded, Receiver, Sender};
use image::{DynamicImage, RgbaImage};
use pollster::FutureExt;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};
use texture::Texture;
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
    prelude::{self as bones, BitSet, ComponentIterBitset},
};

mod convert;
mod texture;

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

// Texture sender to the wgpu thread
#[derive(bones_schema::HasSchema, Clone)]
#[repr(C)]
#[schema(opaque)]
#[schema(no_default)]
struct TextureSender(Sender<(Texture, bones::Entity)>);

type TextureReceiver = Receiver<(Texture, bones::Entity)>;

// Indicates that we already loaded the sprite texture
// and sent it to the wgpu thread
#[derive(bones_schema::HasSchema, Default, Clone)]
#[repr(C)]
struct TextureLoaded;

#[derive(bones_schema::HasSchema, Default, Clone)]
#[repr(C)]
struct PixelArt(bool);

struct App {
    state: Option<State>,
    instance: Arc<wgpu::Instance>,
    adapter: Arc<wgpu::Adapter>,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    texture_bind_group_layout: Arc<wgpu::BindGroupLayout>,
    receiver: TextureReceiver,
    game: bones::Game,
}

/// Renderer for [`bones_framework`] [`Game`][bones::Game]s using wgpu.
pub struct BonesWgpuRenderer {
    /// Whether or not to load all assets on startup with a loading screen,
    /// or skip straight to running the bones game immedietally.
    pub preload: bool,
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
            pixel_art: true,
            game,
            game_version: bones::Version::new(0, 1, 0),
            app_namespace: ("local".into(), "developer".into(), "bones_demo_game".into()),
            asset_dir: PathBuf::from("assets"),
            packs_dir: PathBuf::from("packs"),
        }
    }

    pub fn run(mut self) {
        let (instance, adapter, device, queue, texture_bind_group_layout) = async {
            let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
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
            let texture_bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
            (
                Arc::new(instance),
                Arc::new(adapter),
                Arc::new(device),
                Arc::new(queue),
                Arc::new(texture_bind_group_layout),
            )
        }
        .block_on();

        self.game
            .insert_shared_resource(WgpuDevice(Some(device.clone())));
        self.game
            .insert_shared_resource(WgpuQueue(Some(queue.clone())));
        self.game.insert_shared_resource(PixelArt(self.pixel_art));

        let (sender, receiver) = unbounded();
        self.game.insert_shared_resource(TextureSender(sender));

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

        // Insert empty inputs that will be updated by the `insert_bones_input` system later.
        self.game.init_shared_resource::<bones::KeyboardInputs>();
        self.game.init_shared_resource::<bones::MouseInputs>();
        self.game.init_shared_resource::<bones::GamepadInputs>();
        self.game
            .init_shared_resource::<bones::MouseScreenPosition>();
        self.game
            .init_shared_resource::<bones::MouseWorldPosition>();

        //Insert needed systems
        for (_, session) in self.game.sessions.iter_mut() {
            session.add_system_to_stage(bones::First, load_sprite);
        }

        // wgpu uses `log` for all of our logging, so we initialize a logger with the `env_logger` crate.
        env_logger::init();

        let event_loop = EventLoop::builder().build().unwrap();

        // When the current loop iteration finishes, immediately begin a new
        // iteration regardless of whether or not new events are available to
        // process.
        event_loop.set_control_flow(ControlFlow::Poll);

        let mut app = App {
            state: None,
            instance,
            adapter,
            device,
            queue,
            texture_bind_group_layout,
            receiver,
            game: self.game,
        };
        event_loop.run_app(&mut app).unwrap();
    }
}

fn load_sprite(
    entities: bones::Res<bones::Entities>,
    sprites: bones::Comp<bones::Sprite>,
    assets: bones::Res<bones::AssetServer>,
    device: bones::Res<WgpuDevice>,
    queue: bones::Res<WgpuQueue>,
    texture_sender: bones::Res<TextureSender>,
    pixel_art: bones::Res<PixelArt>,
    mut texture_loaded: bones::CompMut<TextureLoaded>,
) {
    let mut not_loaded = texture_loaded.bitset().clone();
    not_loaded.bit_not();

    for entity in entities.iter_with_bitset(&not_loaded) {
        let Some(sprite) = sprites.get(entity) else {
            continue;
        };

        //Load and send texture
        let image = assets.get(sprite.image);
        if let bones::Image::Data(img) = &*image {
            let texture =
                Texture::from_image(device.get(), queue.get(), img, None, pixel_art.0).unwrap();
            texture_sender.0.send((texture, entity)).unwrap();
            texture_loaded.insert(entity, TextureLoaded);
        } else {
            unreachable!()
        };
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

struct State {
    window: Arc<Window>,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface<'static>,
    surface_format: wgpu::TextureFormat,
    render_pipeline: wgpu::RenderPipeline,
    sprites: Vec<(wgpu::BindGroup, bones::Entity)>,
}

impl State {
    fn new(
        window: Arc<Window>,
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        instance: &wgpu::Instance,
        adapter: &wgpu::Adapter,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
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
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[texture_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent::REPLACE,
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
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

        State {
            window,
            device: device.clone(),
            queue,
            size,
            surface,
            surface_format,
            render_pipeline,
            sprites: vec![],
        }
    }

    fn get_window(&self) -> &Window {
        &self.window
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

    fn render(&mut self, sessions: &bones::Sessions) {
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

        // Create the command encoder.
        let mut encoder = self.device.create_command_encoder(&Default::default());

        //Index buffer is the same for all sprites
        let index_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(INDICES),
                usage: wgpu::BufferUsages::INDEX,
            });
        let num_indices = INDICES.len() as u32;

        for (_, session) in sessions.iter() {
            //Get cameras and sort them
            let cameras = session.world.component::<bones::Camera>();
            let transforms = session.world.component::<bones::Transform>();
            let entities = session.world.resource::<bones::Entities>();

            let mut bitset = cameras.bitset().clone();
            bitset.bit_and(transforms.bitset());

            let mut cameras_vec = vec![];
            for ent in entities.iter_with_bitset(&bitset) {
                let Some(camera) = cameras.get(ent) else {
                    continue;
                };

                if !camera.active {
                    continue;
                }

                // Get the transform from the ECS.
                let Some(transform) = transforms.get(ent).cloned() else {
                    continue;
                };

                cameras_vec.push((camera, transform));
            }
            cameras_vec.sort_by(|a, b| a.0.priority.cmp(&b.0.priority));

            for (camera, mut camera_transform) in cameras_vec {
                // Create one render pass for each camera
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
                render_pass.set_pipeline(&self.render_pipeline);

                let scale_ratio;
                if let Some(viewport) = camera.viewport.option() {
                    let size = IVec2::new(self.size.width as i32, self.size.height as i32)
                        - viewport.position.as_ivec2();
                    let size = IVec2::new(
                        (viewport.size.x as i32).min(size.x),
                        (viewport.size.y as i32).min(size.y),
                    );

                    if size.x <= 0 || size.y <= 0 {
                        continue;
                    }

                    render_pass.set_viewport(
                        viewport.position.x as f32,
                        viewport.position.y as f32,
                        size.x as f32,
                        size.y as f32,
                        viewport.depth_min,
                        viewport.depth_max,
                    );
                    if viewport.size.y <= viewport.size.x {
                        scale_ratio = Mat4::from_scale(Vec3::new(
                            size.x as f32 / viewport.size.x as f32,
                            viewport.size.y as f32 / viewport.size.x as f32 * size.y as f32
                                / viewport.size.y as f32,
                            1.,
                        ))
                        .inverse();
                    } else {
                        scale_ratio = Mat4::from_scale(Vec3::new(
                            viewport.size.x as f32 / viewport.size.y as f32 * size.x as f32
                                / viewport.size.x as f32,
                            size.y as f32 / viewport.size.y as f32,
                            1.,
                        ))
                        .inverse();
                    }
                } else if self.size.height <= self.size.width {
                    scale_ratio = Mat4::from_scale(Vec3::new(
                        1.,
                        self.size.height as f32 / self.size.width as f32,
                        1.,
                    ))
                    .inverse();
                } else {
                    scale_ratio = Mat4::from_scale(Vec3::new(
                        self.size.width as f32 / self.size.height as f32,
                        1.,
                        1.,
                    ))
                    .inverse();
                }

                if camera.draw_background_color {
                    let vertex_buffer =
                        self.device
                            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                                label: Some("Vertex Buffer"),
                                contents: bytemuck::cast_slice(&VERTICES_FULL),
                                usage: wgpu::BufferUsages::VERTEX,
                            });

                    let rgba = RgbaImage::from_pixel(
                        1,
                        1,
                        image::Rgba(camera.background_color.as_rgba_u8()),
                    );
                    let img = DynamicImage::ImageRgba8(rgba);
                    let texture = Texture::from_image(
                        &self.device,
                        &self.queue,
                        &img,
                        Some("background_color"),
                        false,
                    )
                    .unwrap();
                    let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                        layout: &self.render_pipeline.get_bind_group_layout(0),
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(&texture.view),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::Sampler(&texture.sampler),
                            },
                        ],
                        label: Some("diffuse_bind_group"),
                    });

                    render_pass.set_bind_group(0, &bind_group, &[]);
                    render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                    render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                    render_pass.draw_indexed(0..num_indices, 0, 0..1);
                }

                // Render each sprite with its own transform.
                for (bind_group, entity) in &self.sprites {
                    // Get the entity transform from the ECS.
                    let Some(transform) = session
                        .world
                        .component::<bones::Transform>()
                        .get(*entity)
                        .cloned()
                    else {
                        continue;
                    };

                    camera_transform.translation.z = 0.;
                    let camera_mat4 = camera_transform.to_matrix().inverse();

                    // Get the 4×4 transform matrix.
                    let mat4 = scale_ratio * camera_mat4 * transform.to_matrix();

                    // Compute transformed vertices on the CPU.
                    let transformed_vertices: Vec<Vertex> = VERTICES
                        .iter()
                        .map(|v: &Vertex| Vertex {
                            position: transform_vertex(
                                v.position.into(),
                                mat4,
                                Vec3::ZERO, // Use Vec3::ZERO as the pivot point for transformations
                            )
                            .into(),
                            tex_coords: v.tex_coords, // Texture coordinates remain unchanged.
                        })
                        .collect();

                    // Create a dynamic vertex buffer with the transformed vertices.
                    let vertex_buffer =
                        self.device
                            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                                label: Some("Transformed Vertex Buffer"),
                                contents: bytemuck::cast_slice(&transformed_vertices),
                                usage: wgpu::BufferUsages::VERTEX,
                            });

                    render_pass.set_bind_group(0, bind_group, &[]);
                    render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                    render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                    render_pass.draw_indexed(0..num_indices, 0, 0..1);
                }
            }
        }

        // Submit the command queue.
        self.queue.submit([encoder.finish()]);
        self.window.pre_present_notify();
        surface_texture.present();
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
                &self.texture_bind_group_layout,
            );

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

        for (texture, entity) in self.receiver.try_iter() {
            let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &self.texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&texture.sampler),
                    },
                ],
                label: Some("diffuse_bind_group"),
            });

            state.sprites.push((bind_group, entity));
        }

        match event {
            WindowEvent::CloseRequested => {
                //Close window
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                state.render(&self.game.sessions);
                // Emits a new redraw requested event.
                state.get_window().request_redraw();

                //Step bones
                self.game.step(Instant::now());
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

//TODO Add quaternion rotations, and move this calculation to
// wgsl code

fn transform_vertex(pos: Vec3, transform: Mat4, pivot: Vec3) -> Vec3 {
    // Translate the vertex to the pivot point.
    let to_origin = Mat4::from_translation(-pivot);
    let from_origin = Mat4::from_translation(pivot);

    // Apply the transformation: first translate to origin, then transform, then translate back.
    let combined = from_origin * transform * to_origin;

    // Apply the transformation matrix to the position
    let pos4 = combined * pos.extend(1.0);

    // Return the truncated Vec3 position (ignoring the w-component)
    pos4.truncate()
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
