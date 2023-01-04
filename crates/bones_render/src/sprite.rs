//! Sprite rendering components.

use crate::prelude::*;

/// Image asset type, contains no data, but [`Handle<Image>`] is still useful because it uniquely
/// represents an image that may be rendered outside of the core.
#[derive(Copy, Clone, TypeUlid, Debug)]
#[ulid = "01GNJGPQ8TKA234G1EA510BD96"]
pub struct Image;

/// A 2D sprite component
#[derive(Clone, TypeUlid, Debug)]
#[ulid = "01GNJXPWZKS6BHJEG1SX5B93DA"]
pub struct Sprite {
    /// The sprite image handle.
    pub image: Handle<Image>,
}
