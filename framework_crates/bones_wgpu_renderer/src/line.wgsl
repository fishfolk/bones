// Must match AtlasSpriteUniform in sprite.rs
struct AtlasSpriteUniform {
    entity_type:  u32,
    camera_index: u32,
    _pad0:        u32,
    _pad1:        u32,
    transform:    mat4x4<f32>,
    color_tint:   vec4<f32>,

    flip_x:       u32,
    flip_y:       u32,
    uv_min:       vec2<f32>,
    uv_max:       vec2<f32>,

    tile_size:    vec2<f32>,
    image_size:   vec2<f32>,
    padding:      vec2<f32>,
    offset:       vec2<f32>,

    columns:      u32,
    index:        u32,
};

struct CameraUniform {
    transform: mat4x4<f32>,
    screen_size: vec2<f32>,
    _pad0: u32,
    _pad1: u32,
}

@group(0) @binding(0)
var<storage, read> sprite_data: array<AtlasSpriteUniform>;

@group(1) @binding(0)
var<storage, read> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color:    vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0)       color:    vec4<f32>,
};

@vertex
fn vs_main(
    vert: VertexInput,
    @builtin(instance_index) idx: u32,
) -> VertexOutput {
    let inst = sprite_data[idx];
    var out: VertexOutput;

    // entity_type == 3u: clip-space quad for clearing a camera viewport background
    if inst.entity_type == 3u {
        out.clip_pos = vec4<f32>(vert.position, 0.0, 1.0);
        out.color = inst.color_tint;
        return out;
    }

    let pixel_scale = 0.001;
    let scaled_position = vec3<f32>(
        vert.position.x * pixel_scale,
        vert.position.y * pixel_scale,
        0.0,
    );

    let transform_scale = vec3<f32>(pixel_scale, pixel_scale, 1.0);
    let inst_mat = scale_translation(inst.transform, transform_scale);

    out.clip_pos = camera.transform * inst_mat * vec4<f32>(scaled_position, 1.0);
    out.color = vert.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}

fn scale_translation(matrix: mat4x4<f32>, translation_scale: vec3<f32>) -> mat4x4<f32> {
    return mat4x4<f32>(
        matrix[0],
        matrix[1],
        matrix[2],
        vec4<f32>(matrix[3].xyz * translation_scale, matrix[3].w)
    );
}

