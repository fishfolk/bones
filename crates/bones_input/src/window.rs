/// Information about the window the game is running in.
#[derive(Clone, Copy, Debug, Default, bones_reflect::HasSchema)]
#[repr(C)]
pub struct Window {
    /// The logical size of the window's client area.
    pub size: glam::Vec2,
}
