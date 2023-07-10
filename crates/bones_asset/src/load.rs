use crate::prelude::*;

/// Context needed to load an asset pack. Can be used to deserialize assets.
pub struct AssetPackLoadCtx<'a> {
    /// Reference to the asset server.
    pub server: &'a AssetServer,
}
