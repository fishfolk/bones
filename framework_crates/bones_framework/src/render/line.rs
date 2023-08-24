//! Line rendering, useful for debugging.

use crate::prelude::*;

/// A component for rendering a 2D line path, made up of a list of straight line segments.
#[derive(Clone, Debug, HasSchema)]
pub struct Path2d {
    /// The color of the path.
    pub color: Color,
    /// The list of points in the path
    pub points: Vec<Vec2>,

    /// The thickness of the line.
    pub thickness: f32,

    /// List of indexes into the `points` vector, for which that point should **not** beconnected to
    /// the next point in the list.
    ///
    /// This allows you to make multiple, disconnected paths without needing to create more entities
    /// with a [`Path2d`] component.
    pub line_breaks: Vec<usize>,
}

impl Default for Path2d {
    fn default() -> Self {
        Self {
            thickness: 1.0,
            points: default(),
            color: Color::WHITE,
            line_breaks: default(),
        }
    }
}
