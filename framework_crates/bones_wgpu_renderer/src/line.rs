use std::ops::Range;

use bones_framework::glam::Vec2;
use lyon::math::Point;
use lyon::path::Path;
use lyon::tessellation::{
    BuffersBuilder, StrokeOptions, StrokeTessellator, StrokeVertex, VertexBuffers,
};

use crate::bones::Path2d;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
}

impl Vertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

#[derive(Clone, Debug)]
pub struct LineDrawCall {
    pub index_range: Range<u32>,
    pub base_vertex: i32,
    pub instance_index: u32,
}

pub struct LineBuffers {
    pub vertex_buffer: wgpu::Buffer,
    vertex_capacity: u64,
    pub index_buffer: wgpu::Buffer,
    index_capacity: u64,
}

impl LineBuffers {
    const INITIAL_CAPACITY: u64 = 4096;

    pub fn new(device: &wgpu::Device) -> Self {
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("line_vertex_buffer"),
            size: Self::INITIAL_CAPACITY,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("line_index_buffer"),
            size: Self::INITIAL_CAPACITY,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            vertex_buffer,
            vertex_capacity: Self::INITIAL_CAPACITY,
            index_buffer,
            index_capacity: Self::INITIAL_CAPACITY,
        }
    }

    pub fn write_buffers(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        vertices: &[Vertex],
        indices: &[u32],
    ) {
        if !vertices.is_empty() {
            let needed = std::mem::size_of_val(vertices) as u64;
            if needed > self.vertex_capacity {
                let new_cap = (self.vertex_capacity.max(needed)) * 2;
                self.vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("line_vertex_buffer (resized)"),
                    size: new_cap,
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                self.vertex_capacity = new_cap;
            }
            queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(vertices));
        }

        if !indices.is_empty() {
            let needed = std::mem::size_of_val(indices) as u64;
            if needed > self.index_capacity {
                let new_cap = (self.index_capacity.max(needed)) * 2;
                self.index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("line_index_buffer (resized)"),
                    size: new_cap,
                    usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                self.index_capacity = new_cap;
            }
            queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(indices));
        }
    }
}

/// Splits `path.points` into disconnected polyline subpaths according to `path.line_breaks`.
///
/// Each index `i` in `line_breaks` starts a new subpath at `points[i]` (i.e. `points[i - 1]` is
/// not connected to `points[i]`), matching `bones_bevy_renderer` and `jumpy`'s debug line builder.
fn process_segments(path: &Path2d) -> Vec<&[Vec2]> {
    let mut segments = Vec::new();
    if path.points.len() < 2 {
        return segments;
    }

    let mut line_breaks = path.line_breaks.clone();
    line_breaks.sort_unstable();
    line_breaks.dedup();

    let mut start = 0;
    for &break_point in &line_breaks {
        if break_point == 0 {
            continue;
        }
        let end = break_point.min(path.points.len());
        if start < end {
            let segment = &path.points[start..end];
            if segment.len() >= 2 {
                segments.push(segment);
            }
        }
        start = end;
        if start >= path.points.len() {
            break;
        }
    }

    if start < path.points.len() {
        let segment = &path.points[start..];
        if segment.len() >= 2 {
            segments.push(segment);
        }
    }

    segments
}

// Tessellate path into vertex/index buffers
pub fn tessellate_path(path: &Path2d) -> VertexBuffers<Vertex, u32> {
    let mut geometry = VertexBuffers::new();
    if path.points.len() < 2 || path.thickness <= 0.0 || !path.thickness.is_finite() {
        return geometry;
    }

    let segments = process_segments(path);
    if segments.is_empty() {
        return geometry;
    }

    let mut lyon_path_builder = Path::builder();
    let mut has_valid_segment = false;

    for segment in segments {
        if !segment.iter().all(|p| p.is_finite()) {
            continue;
        }
        let mut points = segment.iter();
        if let Some(first) = points.next() {
            lyon_path_builder.begin(Point::new(first.x, first.y));
            for point in points {
                lyon_path_builder.line_to(Point::new(point.x, point.y));
            }
            lyon_path_builder.end(false);
            has_valid_segment = true;
        }
    }

    if !has_valid_segment {
        return geometry;
    }

    let lyon_path = lyon_path_builder.build();
    let options = StrokeOptions::default().with_line_width(path.thickness);
    let color = crate::convert::color_to_linear_rgba_f32(path.color);

    if let Err(err) = StrokeTessellator::new().tessellate_path(
        &lyon_path,
        &options,
        &mut BuffersBuilder::new(&mut geometry, |vertex: StrokeVertex| Vertex {
            position: vertex.position().to_array(),
            color,
        }),
    ) {
        log::warn!("Path2d tessellation failed: {err:?}");
        geometry.vertices.clear();
        geometry.indices.clear();
    }

    geometry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_segments_splits_before_line_break_indices() {
        let a = Vec2::new(0.0, 0.0);
        let b = Vec2::new(10.0, 0.0);
        let c = Vec2::new(20.0, 20.0);
        let d = Vec2::new(30.0, 20.0);

        // Matches `jumpy`'s `RapierDebugBackend`: push two endpoints, then push `points.len()`.
        let path = Path2d {
            points: vec![a, b, c, d],
            line_breaks: vec![2, 4],
            thickness: 2.0,
            ..Default::default()
        };
        let segs = process_segments(&path);
        assert_eq!(segs, vec![&[a, b][..], &[c, d][..]]);

        let geom = tessellate_path(&path);
        assert!(!geom.vertices.is_empty());
        assert!(!geom.indices.is_empty());
    }

    #[test]
    fn tessellate_path_ignores_non_finite_geometry() {
        let bad = Path2d {
            points: vec![Vec2::ZERO, Vec2::new(f32::NAN, 5.0)],
            thickness: 2.0,
            ..Default::default()
        };
        let geom = tessellate_path(&bad);
        assert!(geom.vertices.is_empty());
        assert!(geom.indices.is_empty());
    }
}
