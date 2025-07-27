use wgpu::util::DeviceExt;
use guillotiere::{AtlasAllocator, size2, Allocation};

// Your per-sprite instance data (with layer)
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct SpriteInstance {
    model: [[f32; 4]; 4],
    uv_min: [f32; 2],
    uv_max: [f32; 2],
    color: [f32; 4],
    layer: f32,
}

// Simplified sprite struct
struct Sprite {
    atlas_id: usize,
    allocation: Allocation,
    layer: f32,
    model: [[f32; 4]; 4],
    uv_min: [f32; 2],
    uv_max: [f32; 2],
    color: [f32; 4],
    is_transparent: bool,
}

impl Sprite {
    fn to_instance(&self) -> SpriteInstance {
        SpriteInstance {
            model: self.model,
            uv_min: self.uv_min,
            uv_max: self.uv_max,
            color: self.color,
            layer: self.layer,
        }
    }
}

// --- 1. Pipeline setup ---

// Depth format
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;

// Create two pipelines: opaque and transparent
fn create_pipelines(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    vs_module: &wgpu::ShaderModule,
    fs_module: &wgpu::ShaderModule,
    sc_format: wgpu::TextureFormat,
) -> (wgpu::RenderPipeline, wgpu::RenderPipeline) {
    let common = |depth_write: bool, blend: Option<wgpu::BlendState>| {
        wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[layout],
                push_constant_ranges: &[],
            })),
            vertex: wgpu::VertexState {
                module: vs_module,
                entry_point: "vs_main",
                buffers: &[
                    // quad vertex buffer (slot 0)
                    wgpu::VertexBufferLayout { /* pos+uv */ ..Default::default() },
                    // instance buffer (slot 1)
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<SpriteInstance>() as _,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &[
                            // model matrix (locations 2–5), uv_min(6), uv_max(7), color(8), layer(9)
                            wgpu::VertexAttribute {
                                offset: 16*4, // after model
                                shader_location: 6,
                                format: wgpu::VertexFormat::Float32x2,
                            },
                            wgpu::VertexAttribute {
                                offset: 16*4 + 2*4,
                                shader_location: 7,
                                format: wgpu::VertexFormat::Float32x2,
                            },
                            wgpu::VertexAttribute {
                                offset: 16*4 + 4*4,
                                shader_location: 8,
                                format: wgpu::VertexFormat::Float32x4,
                            },
                            wgpu::VertexAttribute {
                                offset: 16*4 + 8*4,
                                shader_location: 9,
                                format: wgpu::VertexFormat::Float32,
                            },
                        ],
                    },
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: fs_module,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: sc_format,
                    blend,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: depth_write,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview: None,
        }
    };

    // Opaque: depth-write ON, no blending
    let opaque = device.create_render_pipeline(&common(true, None));

    // Transparent: depth-write OFF, alpha blending
    let alpha_blend = wgpu::BlendState::ALPHA_BLENDING;
    let transparent = device.create_render_pipeline(&common(false, Some(alpha_blend)));

    (opaque, transparent)
}

// --- 2. Render loop ---

fn render(
    encoder: &mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    depth_view: &wgpu::TextureView,
    pipeline_opaque: &wgpu::RenderPipeline,
    pipeline_transparent: &wgpu::RenderPipeline,
    sprites: &mut Vec<Sprite>,
    instance_buf: &wgpu::Buffer,
    atlas_pool: &AtlasPool,
    queue: &wgpu::Queue,
) {
    // Separate lists
    let mut opaque_sprites: Vec<_> = sprites.iter_mut().filter(|s| !s.is_transparent).collect();
    let mut transparent_sprites: Vec<_> = sprites.iter_mut().filter(|s| s.is_transparent).collect();

    // === OPAQUE PASS ===
    {
        // sort only by layer first (secondary atlas to reduce binds if you like)
        opaque_sprites.sort_by(|a, b| {
            a.layer.partial_cmp(&b.layer).unwrap()
             .then(a.atlas_id.cmp(&b.atlas_id))
        });

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Opaque Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations::default(),
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth_view,
                depth_ops: Some(wgpu::Operations::default()),
                stencil_ops: None,
            }),
        });
        pass.set_pipeline(pipeline_opaque);
        pass.set_vertex_buffer(0, /* your quad VB */);
        // Batch by atlas
        let mut last_atlas = None;
        let mut batch = Vec::new();
        for sprite in opaque_sprites {
            if Some(sprite.atlas_id) != last_atlas {
                if let Some(id) = last_atlas {
                    // flush
                    queue.write_buffer(instance_buf, 0, bytemuck::cast_slice(&batch));
                    let atlas = atlas_pool.get(id);
                    pass.set_bind_group(0, &atlas.bind_group, &[]);
                    pass.set_vertex_buffer(1, instance_buf.slice(..));
                    pass.draw_indexed(0..6, 0, 0..batch.len() as u32);
                }
                batch.clear();
                last_atlas = Some(sprite.atlas_id);
            }
            batch.push(sprite.to_instance());
        }
        // flush last
        if let Some(id) = last_atlas {
            queue.write_buffer(instance_buf, 0, bytemuck::cast_slice(&batch));
            let atlas = atlas_pool.get(id);
            pass.set_bind_group(0, &atlas.bind_group, &[]);
            pass.set_vertex_buffer(1, instance_buf.slice(..));
            pass.draw_indexed(0..6, 0, 0..batch.len() as u32);
        }
    }

    // === TRANSPARENT PASS ===
    {
        // sort back-to-front: higher layer first
        transparent_sprites.sort_by(|a, b| {
            b.layer.partial_cmp(&a.layer).unwrap()
             .then(a.atlas_id.cmp(&b.atlas_id))
        });

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Transparent Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: true,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: false, // important: no depth write
                }),
                stencil_ops: None,
            }),
        });
        pass.set_pipeline(pipeline_transparent);
        pass.set_vertex_buffer(0, /* your quad VB */);

        let mut last_atlas = None;
        let mut batch = Vec::new();
        for sprite in transparent_sprites {
            if Some(sprite.atlas_id) != last_atlas {
                if let Some(id) = last_atlas {
                    queue.write_buffer(instance_buf, 0, bytemuck::cast_slice(&batch));
                    let atlas = atlas_pool.get(id);
                    pass.set_bind_group(0, &atlas.bind_group, &[]);
                    pass.set_vertex_buffer(1, instance_buf.slice(..));
                    pass.draw_indexed(0..6, 0, 0..batch.len() as u32);
                }
                batch.clear();
                last_atlas = Some(sprite.atlas_id);
            }
            batch.push(sprite.to_instance());
        }
        if let Some(id) = last_atlas {
            queue.write_buffer(instance_buf, 0, bytemuck::cast_slice(&batch));
            let atlas = atlas_pool.get(id);
            pass.set_bind_group(0, &atlas.bind_group, &[]);
            pass.set_vertex_buffer(1, instance_buf.slice(..));
            pass.draw_indexed(0..6, 0, 0..batch.len() as u32);
        }
    }
}
