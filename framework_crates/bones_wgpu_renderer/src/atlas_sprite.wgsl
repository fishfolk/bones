// Must match the Rust layout
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

// Camera uniform, one per render pass 
struct CameraUniform {
    transform: mat4x4<f32>,
    screen_size: vec2<f32>,
    _pad0: u32,  
    _pad1: u32, // Padding to ensure 16-byte alignment
}

// Bindings
@group(0) @binding(0) var diffuseTex: texture_2d<f32>;
@group(0) @binding(1) var texSampler: sampler;

// All per-instance data lives here
@group(1) @binding(0)
var<storage, read> sprite_data: array<AtlasSpriteUniform>;

@group(2) @binding(0)
var<storage, read> camera: CameraUniform;

// Quad vertex input
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv:       vec2<f32>,
};

// Data we pass to the fragment stage
struct VertexOutput {
    @builtin(position) clipPos:    vec4<f32>,
    @location(0)       quad_uv:    vec2<f32>,
    @location(1) @interpolate(flat) inst_index: u32,
};

// Vertex Shader
@vertex
fn vs_main(
    vert: VertexInput,
    @builtin(instance_index) idx: u32
) -> VertexOutput {
    let inst = sprite_data[idx];
    var out: VertexOutput;

    // Both sprites (0) and atlas/tiles (1) store their unscaled pixel size in `tile_size`.
    let pixel_scale = 0.001;
    let uv_size = inst.tile_size * pixel_scale;

    // Scale position by UV size
    let scaled_position = vec3<f32>(
        vert.position.x * uv_size.x,
        vert.position.y * uv_size.y,
        vert.position.z
    );

    // 1) Transform should use the same pixel_scale so 1 unit = 1 pixel
    // Scale transform by pixel_scale to match sprite scaling
    let transform_scale = vec3<f32>(pixel_scale, pixel_scale, 1.0);
    let inst_mat = scale_translation(inst.transform, transform_scale);

    // 2) apply camera→clip transform
    out.clipPos = camera.transform * inst_mat * vec4(scaled_position, 1.0);

    // 3) Apply per‑instance flip in normalized [0, 1] quad UV space
    var base_uv = vert.uv;
    if inst.flip_x == 1u { base_uv.x = 1.0 - base_uv.x; }
    if inst.flip_y == 1u { base_uv.y = 1.0 - base_uv.y; }
    out.quad_uv = base_uv;

    // 4) Pass instance index along
    out.inst_index = idx;
    return out;
}

// Fragment Shader
@fragment
fn fs_main(
    in: VertexOutput
) -> @location(0) vec4<f32> {
    let inst = sprite_data[in.inst_index];

    var uv: vec2<f32>;

    // If this is a tiled sheet sprite, compute the sub‐cell directly from [0, 1] `quad_uv`
    if inst.entity_type == 1u {
        // Inset slightly inside [0, 1] (never more than 0.25 texels) so sampling at the 1.0
        // boundary does not bleed into the adjacent tile in an unpadded spritesheet.
        let safe_tile_size = max(inst.tile_size, vec2<f32>(1.0, 1.0));
        let eps = min(vec2<f32>(1e-4, 1e-4), vec2<f32>(0.25, 0.25) / safe_tile_size);
        let quad_uv = clamp(in.quad_uv, eps, vec2<f32>(1.0, 1.0) - eps);

        // Normalize dimensions by image size
        let safe_image_size = max(inst.image_size, vec2<f32>(1.0, 1.0));
        let ts = inst.tile_size / safe_image_size;
        let ps = inst.padding / safe_image_size;
        let os = inst.offset / safe_image_size;

        // Calculate tile position
        let cols = max(inst.columns, 1u);
        let col = inst.index % cols;
        let row = inst.index / cols;

        // Calculate step size (tile + padding)
        let step = ts + ps;

        // Calculate top-left corner of tile
        let tile_min = os + vec2<f32>(f32(col), f32(row)) * step;

        // Calculate bottom-right corner of tile (exclude padding)
        let tile_max = tile_min + ts;

        // 1) Quad UV -> tile UV (inside the atlas texture), clamped to [0, 1] so out-of-bounds
        //    indices never bleed into unrelated sprites packed in the same AtlasPool texture.
        let tile_uv = clamp(mix(tile_min, tile_max, quad_uv), vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 1.0));
        // 2) Tile UV -> atlas-pool UV (inside the packed atlas pool rect)
        uv = mix(inst.uv_min, inst.uv_max, tile_uv);
    } else {
        let clamped_uv = clamp(in.quad_uv, vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 1.0));
        uv = mix(inst.uv_min, inst.uv_max, clamped_uv);
    }

    // Sample the atlas at the correct UV and apply tint
    let tex_col = textureSample(diffuseTex, texSampler, uv);
    return tex_col * inst.color_tint;
}

fn scale_translation(matrix: mat4x4<f32>, translation_scale: vec3<f32>) -> mat4x4<f32> {
    return mat4x4<f32>(
        matrix[0],
        matrix[1],
        matrix[2],
        vec4<f32>(matrix[3].xyz * translation_scale, matrix[3].w)
    );
}
