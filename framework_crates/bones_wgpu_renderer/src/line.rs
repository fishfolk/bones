use bones_framework::glam::Vec2;
use lyon::math::Point;
use lyon::path::Path;
use lyon::tessellation::{
    BuffersBuilder, StrokeOptions, StrokeTessellator, StrokeVertex, VertexBuffers,
};

use crate::bones::Path2d;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 4],
}

impl Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
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

// Helper function to process line breaks
fn process_segments(path: &Path2d) -> Vec<&[Vec2]> {
    let mut segments = Vec::new();
    let mut line_breaks = path.line_breaks.clone();
    line_breaks.sort_unstable();
    line_breaks.dedup();

    let mut start = 0;
    for &break_point in &line_breaks {
        let end = break_point;
        if start <= end && end < path.points.len() {
            let segment = &path.points[start..=end];
            if segment.len() >= 2 {
                segments.push(segment);
            }
        }
        start = end + 1;
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
fn tessellate_path(path: &Path2d) -> VertexBuffers<Vertex, u16> {
    let segments = process_segments(path);
    let mut lyon_path_builder = Path::builder();

    for segment in segments {
        let mut points = segment.iter();
        if let Some(first) = points.next() {
            lyon_path_builder.begin(Point::new(first.x, first.y));
            for point in points {
                lyon_path_builder.line_to(Point::new(point.x, point.y));
            }
            lyon_path_builder.end(false);
        }
    }

    let lyon_path = lyon_path_builder.build();
    let options = StrokeOptions::default().with_line_width(path.thickness);
    let mut geometry = VertexBuffers::new();

    StrokeTessellator::new()
        .tessellate_path(
            &lyon_path,
            &options,
            &mut BuffersBuilder::new(&mut geometry, |vertex: StrokeVertex| Vertex {
                position: vertex.position().to_array(),
                color: path.color.as_rgba_f32(),
            }),
        )
        .expect("Path tessellation failed");

    geometry
}
