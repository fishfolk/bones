use image::{ImageBuffer, Rgba};

#[allow(unused)]
pub fn dump_texture_to_png(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    size: (u32, u32),
    path: &std::path::Path,
) -> anyhow::Result<()> {
    let mut encoder = device.create_command_encoder(&Default::default());

    let (width, height) = size;

    // 1) Create a buffer large enough to hold the RGBA8 data,
    //    with COPY_DST so we can copy into it
    let pixel_size = 4; // RGBA8
    let padded_bytes_per_row = {
        // GPU requires rows aligned to 256 bytes:
        let unpadded = pixel_size * width as usize;
        ((unpadded + 255) / 256) * 256
    };
    let output_buffer_size = (padded_bytes_per_row * height as usize) as u64;

    let dst_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("texture-readback-buffer"),
        size: output_buffer_size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    // 2) Copy the texture to the buffer
    encoder.copy_texture_to_buffer(
        wgpu::ImageCopyTexture {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::ImageCopyBuffer {
            buffer: &dst_buffer,
            layout: wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(padded_bytes_per_row as u32),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );

    // 3) Submit the copy command and wait for it to finish
    queue.submit(Some(encoder.finish()));
    device.poll(wgpu::Maintain::Wait);

    // 4) Map the buffer and read its contents
    let slice = dst_buffer.slice(..);
    slice.map_async(wgpu::MapMode::Read, |res| res.unwrap());
    // wait for the mapping to finish
    device.poll(wgpu::Maintain::Wait);
    let data = slice.get_mapped_range();

    // 5) Demultiplex rows (drop the padding) into a contiguous Vec<u8>
    let mut pixels = Vec::with_capacity((pixel_size * width as usize * height as usize) as usize);
    for row in 0..height as usize {
        let offset = row * padded_bytes_per_row;
        pixels.extend_from_slice(&data[offset..offset + (pixel_size * width as usize)]);
    }
    // Unmap so the buffer can be reused/dropped
    drop(data);
    dst_buffer.unmap();

    // 6) Encode & save with the `image` crate
    let img: ImageBuffer<Rgba<u8>, _> = ImageBuffer::from_raw(width, height, pixels)
        .expect("buffer size should match width*height*4");
    img.save(path)?;

    Ok(())
}
