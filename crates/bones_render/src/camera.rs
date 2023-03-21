//! Camera components.

use crate::prelude::*;

/// Makes an entity behave like a camera.
///
/// The entity must also have a [`Transform`] component for the camera to render anything.
#[derive(Clone, Copy, Debug, TypeUlid)]
#[ulid = "01GNR2978NRN7PH5XWBXP3KMD7"]
#[repr(C)]
pub struct Camera {
    /// The height of the camera in in-game pixels.
    ///
    /// The width of the camera will be determined from the window aspect ratio.
    pub height: f32,
    /// Whether or not the camera is enabled and rendering.
    pub active: bool,
    /// An optional viewport override, allowing you to specify that the camera should render to only
    /// a portion of the window.
    ///
    /// This can be used, for example, for split screen functionality.
    pub viewport: Option<Viewport>,
}

/// A custom viewport specification for a [`Camera`].
#[derive(Clone, Copy, Debug)]
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
        }
    }
}

/// Resource for controlling the clear color.
#[derive(Deref, DerefMut, Clone, Copy, TypeUlid, Default)]
#[ulid = "01GP4XRQYRPQNX4J22E513975M"]
pub struct ClearColor(pub Color);
