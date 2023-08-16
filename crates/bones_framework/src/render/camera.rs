//! Camera components.

use crate::prelude::*;

/// Makes an entity behave like a camera.
///
/// The entity must also have a [`Transform`] component for the camera to render anything.
#[derive(Clone, Copy, Debug, HasSchema)]
#[schema(opaque)] // TODO: make repr(C) when `Option`s are supported.
pub struct Camera {
    /// The height of the camera in in-game pixels.
    ///
    /// The width of the camera will be determined from the window aspect ratio.
    // TODO: implement different scaling modes for bones cameras.
    pub height: f32,
    /// Whether or not the camera is enabled and rendering.
    pub active: bool,
    /// An optional viewport override, allowing you to specify that the camera should render to only
    /// a portion of the window.
    ///
    /// This can be used, for example, for split screen functionality.
    pub viewport: Option<Viewport>,
    /// Cameras with a higher priority will be rendered on top of cameras with a lower priority.
    pub priority: i32,
}

/// A custom viewport specification for a [`Camera`].
#[derive(Clone, Copy, Debug, HasSchema, Default)]
#[repr(C)]
pub struct Viewport {
    /// The physical position to render this viewport to within the RenderTarget of this Camera.
    /// (0,0) corresponds to the top-left corner.
    pub position: UVec2,
    /// The physical size of the viewport rectangle to render to within the RenderTarget of this
    /// Camera. The origin of the rectangle is in the top-left corner.
    pub size: UVec2,
    /// The minimum depth to render (on a scale from 0.0 to 1.0).
    pub depth_min: f32,
    /// The maximum depth to render (on a scale from 0.0 to 1.0).
    pub depth_max: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            height: 400.0,
            active: true,
            viewport: None,
            priority: 0,
        }
    }
}

/// Resource for controlling the clear color.
#[derive(Deref, DerefMut, Clone, Copy, HasSchema, Default)]
#[schema(opaque)]
pub struct ClearColor(pub Color);
