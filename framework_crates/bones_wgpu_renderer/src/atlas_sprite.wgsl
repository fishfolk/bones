struct AtlasSpriteUniform {
    // Atlas parameters
    tile_size:   vec2<f32>,
    image_size:  vec2<f32>,
    padding:     vec2<f32>,
    offset:      vec2<f32>,
    columns:     u32,
    index:       u32,

    // State flags
    use_atlas:   u32,
    flip_x:      u32,
    flip_y:      u32,

    // Padding
    _pad0:       u32,

    color_tint:  vec4<f32>,
};

@group(0) @binding(0) var<uniform> spriteUniform: AtlasSpriteUniform;
@group(0) @binding(1) var diffuseTex: texture_2d<f32>;
@group(0) @binding(2) var texSampler: sampler;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv:       vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clipPos: vec4<f32>,
    @location(0) uv:            vec2<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    return VertexOutput(
        vec4(in.position, 0.0, 1.0),
        in.uv
    );
}

fn compute_atlas_uv(base_uv: vec2<f32>) -> vec2<f32> {
    // Precompute normalized dimensions
    let tile_scale = spriteUniform.tile_size / spriteUniform.image_size;
    let padding_scale = spriteUniform.padding / spriteUniform.image_size;
    let base_offset = spriteUniform.offset / spriteUniform.image_size;

    // Calculate grid position
    let grid_pos = vec2<u32>(
        spriteUniform.index % spriteUniform.columns,
        spriteUniform.index / spriteUniform.columns
    );

    // Calculate tile origin in UV space
    let tile_step = tile_scale + padding_scale;
    let origin = base_offset + vec2<f32>(grid_pos) * tile_step;

    // Calculate tile center and aspect ratio correction
    let tile_center = origin + tile_scale * 0.5;
    let aspect_ratio = normalize(1.0 / spriteUniform.tile_size);

    // Base UV calculation with aspect correction
    var uv = tile_center + (base_uv - 0.5) * tile_scale * aspect_ratio;

    // Apply flipping transformations
    if (spriteUniform.flip_x == 1u) {
        uv.x = tile_center.x + (0.5 - base_uv.x) * tile_scale.x * aspect_ratio.x;
    }
    if (spriteUniform.flip_y == 1u) {
        uv.y = tile_center.y + (0.5 - base_uv.y) * tile_scale.y * aspect_ratio.y;
    }

    return uv;
}

fn apply_flip(uv: vec2<f32>) -> vec2<f32> {
    var fuv = uv;
    if (spriteUniform.flip_x == 1u) {
        fuv.x = 1.0 - fuv.x;
    }
    if (spriteUniform.flip_y == 1u) {
        fuv.y = 1.0 - fuv.y;
    }
    return fuv;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var uv = select(apply_flip(in.uv), compute_atlas_uv(in.uv), spriteUniform.use_atlas == 1u);
    return textureSample(diffuseTex, texSampler, uv) * 
    select(vec4<f32>(1.0, 1.0, 1.0, 1.0), spriteUniform.color_tint, spriteUniform.use_atlas == 1u);
}