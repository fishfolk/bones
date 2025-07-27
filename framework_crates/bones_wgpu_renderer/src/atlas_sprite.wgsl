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
var<storage, read> cameras: array<CameraUniform>;

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

    // 1) compute quad size in *pixels* from your atlas‐tile:
    //    inst.tile_size is already [width_px, height_px]
    let quad_px = vec3<f32>(
        vert.position.x * inst.tile_size.x,
        vert.position.y * inst.tile_size.y,
        vert.position.z
    );

    // 2) convert pixel‐coordinates → NDC ([-1,1]) per axis
    let cam = cameras[inst.camera_index];
    let px_to_ndc = vec2<f32>(
        2.0 / cam.screen_size.x,
        2.0 / cam.screen_size.y
    );

    // 3) build your instance matrix so that
    //    translation is in *pixels* → NDC,
    //    but rotation & scale from inst.transform stay in world‐units
    let inst_mat = scale_translation(
        inst.transform,
        vec3<f32>(px_to_ndc.x, px_to_ndc.y, 1.0)
    );

    // 4) finally, emit clip‐space position:
    out.clipPos = cam.transform * inst_mat * vec4<f32>(quad_px, 1.0);

    // …then do your UV flip/remap, color_tint etc…
    var base_uv = vert.uv;
    if inst.flip_x == 1u { base_uv.x = 1.0 - base_uv.x; }
    if inst.flip_y == 1u { base_uv.y = 1.0 - base_uv.y; }
    out.atlas_uv = mix(inst.uv_min, inst.uv_max, base_uv);
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
        
        // Calculate bottom-right corner of tile
        let tile_max = tile_min + step;
        
        // Map UV from [0,1] to [tile_min, tile_max]
        uv = mix(tile_min, tile_max, uv);
    }

    // Sample the atlas at the correct UV and apply tint
    let tex_col = textureSample(diffuseTex, texSampler, uv);
    return tex_col * inst.color_tint;
    //return inst.color_tint;
}

fn scale_translation(matrix: mat4x4<f32>, translation_scale: vec3<f32>) -> mat4x4<f32> {
    // Extract translation from last column
    let translation = matrix[3].xyz;

    // Extract scale from basis vector lengths
    let scale = vec3<f32>(
        length(matrix[0].xyz),
        length(matrix[1].xyz),
        length(matrix[2].xyz)
    );

    // Extract Z-rotation from first column
    let angle = atan2(matrix[0].y, matrix[0].x);

    // Compute sin/cos once
    let c = cos(angle);
    let s = sin(angle);

    // Scale translation components
    let tx = translation.x * translation_scale.x;
    let ty = translation.y * translation_scale.y;
    let tz = translation.z * translation_scale.z;

    // Rebuild matrix in column-major order
    return mat4x4<f32>(
        vec4(c * scale.x, s * scale.x, 0.0, 0.0),   // Column 0
        vec4(-s * scale.y, c * scale.y, 0.0, 0.0),  // Column 1
        vec4(0.0, 0.0, scale.z, 0.0),               // Column 2
        vec4(tx, ty, tz, 1.0)                       // Column 3
    );
}
