#[derive(bones_schema::HasSchema)]
#[repr(C)]
#[schema(opaque, no_default, no_clone)]
pub struct RenderBuffers {
    // Instance data (updated per entity)
    pub base: wgpu::Buffer,
    pub sprite_flags: wgpu::Buffer,
    pub atlas_data: wgpu::Buffer,
}

impl RenderBuffers {
    pub fn new(device: &wgpu::Device) -> Self {
        // Initialize with empty data
        let initial_size = 1024; // Starting capacity
        let buffer_desc = |usage| wgpu::BufferDescriptor {
            label: None,
            size: initial_size as u64,
            usage,
            mapped_at_creation: false,
        };

        Self {
            base: device.create_buffer(&buffer_desc(
                wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            )),
            sprite_flags: device.create_buffer(&buffer_desc(
                wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            )),
            atlas_data: device.create_buffer(&buffer_desc(
                wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            )),
        }
    }
}

// Common to ALL renderable types
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BaseInstance {
    pub transform: [[f32; 4]; 4], // Mat4
    pub entity_type: u32,         // 0 = sprite, 1 = atlas, 2 = path2d
    pub color: [f32; 4],
}

// Sprite/Atlas-specific extensions
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpriteFlags {
    pub flip: [u32; 2],     // Packed as bits: 0x1 = flip_x, 0x2 = flip_y
    pub uv_min: [f32; 2], // Atlas UV coordinates (min_x, min_y)
    pub uv_max: [f32; 2], // Atlas UV coordinates (max_x, max_y)
}

// Atlas-specific data
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct AtlasData {
    pub tile_size: [f32; 2],
    pub image_size: [f32; 2],
    pub padding: [f32; 2],
    pub offset: [f32; 2],
    pub columns: u32,
    pub index: u32,
}
