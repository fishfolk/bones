//! Window information.

use crate::prelude::*;

/// Information about the window the game is running in.
#[derive(Clone, Copy, Debug, Default, HasSchema)]
#[repr(C)]
pub struct Window {
    /// The logical size of the window's client area.
    ///
    /// This is considered read-only and is updated from the window size by the rendering
    /// integration.
    pub size: glam::Vec2,
    /// May be set to change whether or not the game is displayed full-screen.
    pub fullscreen: bool,
    /// Whether or not the window is focused.
    ///
    /// This is considered read-only and is updated from the window focus by the
    /// rendering integration.
    pub focused: bool,
}
