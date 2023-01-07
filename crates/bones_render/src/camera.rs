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
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            height: 400.0,
            active: true,
        }
    }
}

/// Resource for controlling the clear color.
#[derive(Deref, DerefMut, Clone, Copy, TypeUlid, Default)]
#[ulid = "01GP4XRQYRPQNX4J22E513975M"]
pub struct ClearColor(pub [f32; 4]);
