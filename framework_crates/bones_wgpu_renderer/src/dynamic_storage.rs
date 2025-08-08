use std::sync::Arc;

pub struct DynamicBuffer {
    pub layout: Arc<wgpu::BindGroupLayout>,
    pub buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    pub capacity: u64, // in bytes
    buffer_usage: wgpu::BufferUsages,
}

impl DynamicBuffer {
    /// Create with an initial capacity (in bytes).
    pub fn new(
        device: &wgpu::Device,
        layout: Arc<wgpu::BindGroupLayout>,
        initial_capacity: u64,
        buffer_usage: wgpu::BufferUsages,
    ) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("dynamic_storage_buffer"),
            size: initial_capacity,
            usage: buffer_usage | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &*layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some("dynamic_storage_bind_group"),
        });
        Self {
            layout,
            buffer,
            bind_group,
            capacity: initial_capacity,
            buffer_usage,
        }
    }

    /// Ensure we have at least `needed` bytes; if not, reallocate & rebind.
    fn ensure_capacity(&mut self, device: &wgpu::Device, needed: u64) {
        if needed <= self.capacity {
            return;
        }
        // double up (or at least `needed`)
        let new_capacity = (self.capacity.max(needed)) * 2;
        self.buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("dynamic_storage_buffer (resized)"),
            size: new_capacity,
            usage: self.buffer_usage | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &*self.layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: self.buffer.as_entire_binding(),
            }],
            label: Some("dynamic_storage_bind_group (resized)"),
        });
        self.capacity = new_capacity;
    }

    /// Write your Pod‐slice into the buffer, growing if necessary.
    pub fn write_pods<T: bytemuck::Pod>(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        data: &[T],
    ) {
        let needed_bytes = (data.len() * std::mem::size_of::<T>()) as u64;
        self.ensure_capacity(device, needed_bytes);
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(data));

        println!("DynamicBuffer: Wrote {} bytes", needed_bytes);
    }

    /// Access the bind group for binding in your render pass.
    pub fn get_bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}
