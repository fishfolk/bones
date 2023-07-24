use type_ulid::TypeUlid;

/// Information about the window the game is running in.
#[derive(Clone, Copy, Debug, Default, TypeUlid)]
#[ulid = "01GP70WMVH4HV4YHZ240E0YC7X"]
pub struct Window {
    /// The logical size of the window's client area.
    pub size: glam::Vec2,
}
