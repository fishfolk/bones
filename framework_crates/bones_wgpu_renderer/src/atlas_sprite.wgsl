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

    var uv_size: vec2<f32>;
    if inst.entity_type == 1u {
        // For atlas/tiles: use tile_size directly
        uv_size = inst.tile_size;
    } else {
        // For sprites: use calculated UV size multiplied by 4096 (atlas pool size)
        uv_size = (inst.uv_max - inst.uv_min) * 4096.0;
    }
    
    // Convert UV size to world size: use a fixed scale factor instead of screen size
    // This prevents distortion when window is resized
    let pixel_scale = 0.001; // Adjust this value to control sprite size (smaller = smaller sprites)
    uv_size = uv_size * pixel_scale;
    
    let aspect_ratio = uv_size.x / uv_size.y;

    // Scale position by UV size (not just aspect ratio)
    let scaled_position = vec3<f32>(
        vert.position.x * uv_size.x,
        vert.position.y * uv_size.y,
        vert.position.z
    );

    // load the camera uniform
    let camera = cameras[inst.camera_index];

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
        // Map UV from [0,1] to [tile_min, tile_max] (only tile area, not padding)
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
