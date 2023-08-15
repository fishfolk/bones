//! The bones framework for game development.

#![warn(missing_docs)]
// This cfg_attr is needed because `rustdoc::all` includes lints not supported on stable
#![cfg_attr(doc, allow(unknown_lints))]
#![deny(rustdoc::all)]

#[doc(inline)]
pub use bones_lib as lib;

/// Math library.
#[doc(inline)]
pub use glam;

/// The prelude.
pub mod prelude {
    pub use crate::{
        animation::*, input::prelude::*, params::*, render::prelude::*, AssetServerExt,
        DefaultPlugin,
    };
    pub use bones_lib::prelude::*;
    pub use glam::*;
}

pub mod animation;
pub mod input;
pub mod params;
pub mod render;

/// Default plugins for bones framework sessions.
pub struct DefaultPlugin;
impl lib::Plugin for DefaultPlugin {
    fn install(self, session: &mut lib::Session) {
        session.install_plugin(animation::plugin);
    }
}

/// Extension trait for the bones [`AssetServer`][bones_lib::prelude::AssetServer].
pub trait AssetServerExt {
    /// Register the default assets from `bones_framework`.
    fn register_default_assets(self) -> Self;
}
impl AssetServerExt for &mut bones_lib::prelude::AssetServer {
    fn register_default_assets(self) -> Self {
        self.register_asset::<crate::prelude::Image>();
        self.register_asset::<crate::prelude::Atlas>();
        self
    }
}
