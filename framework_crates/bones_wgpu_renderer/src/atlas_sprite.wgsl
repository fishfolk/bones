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
    @location(0)       atlas_uv:   vec2<f32>,
    @location(1)       inst_index: u32,
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

    // 2) Apply per‑instance flip
    var base_uv = vert.uv;
    if inst.flip_x == 1u { base_uv.x = 1.0 - base_uv.x; }
    if inst.flip_y == 1u { base_uv.y = 1.0 - base_uv.y; }

    // 3) Remap [0,1] → [uv_min,uv_max] of the atlas
    out.atlas_uv = mix(inst.uv_min, inst.uv_max, base_uv);

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

    // Start with the quad’s atlas-mapped UV
    var uv = in.atlas_uv;

    //If this is a tiled sheet sprite, compute the sub‐cell
    if inst.entity_type == 1u {
        // `uv` is currently in atlas-pool space (already remapped by the vertex shader).
        // Recover the original quad UV in [0,1] (after flip) so we can do correct tile selection.
        // Inset slightly so nearest-neighbor sampling at the 1.0 boundary does not bleed into
        // the adjacent tile in the atlas texture.
        let uv_span = max(inst.uv_max - inst.uv_min, vec2<f32>(1e-6, 1e-6));
        let quad_uv = clamp((uv - inst.uv_min) / uv_span, vec2<f32>(1e-4, 1e-4), vec2<f32>(1.0 - 1e-4, 1.0 - 1e-4));

        // Normalize dimensions by image size
        let ts = inst.tile_size / inst.image_size;
        let ps = inst.padding / inst.image_size;
        let os = inst.offset / inst.image_size;

        // Calculate tile position
        let col = inst.index % inst.columns;
        let row = inst.index / inst.columns;
        
        // Calculate step size (tile + padding)
        let step = ts + ps;
        
        // Calculate top-left corner of tile
        let tile_min = os + vec2<f32>(f32(col), f32(row)) * step;
        
        // Calculate bottom-right corner of tile (exclude padding)
        let tile_max = tile_min + ts;

        // 1) Quad UV -> tile UV (inside the atlas texture)
        let tile_uv = mix(tile_min, tile_max, quad_uv);
        // 2) Tile UV -> atlas-pool UV (inside the packed atlas pool rect)
        uv = mix(inst.uv_min, inst.uv_max, tile_uv);
    }

    // Sample the atlas at the correct UV and apply tint
    let tex_col = textureSample(diffuseTex, texSampler, uv);
    return tex_col * inst.color_tint;
    //return inst.color_tint;
}

fn scale_translation(matrix: mat4x4<f32>, translation_scale: vec3<f32>) -> mat4x4<f32> {
    return mat4x4<f32>(
        matrix[0],
        matrix[1],
        matrix[2],
        vec4<f32>(matrix[3].xyz * translation_scale, matrix[3].w)
    );
}
