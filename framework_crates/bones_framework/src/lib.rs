//! The bones framework for game development.
//!
#![cfg_attr(feature = "document-features", doc = "## Features")]
#![cfg_attr(feature = "document-features", doc = document_features::document_features!())]
#![warn(missing_docs)]
// This cfg_attr is needed because `rustdoc::all` includes lints not supported on stable
#![cfg_attr(doc, allow(unknown_lints))]
#![deny(rustdoc::all)]

#[doc(inline)]
pub use bones_lib as lib;

#[doc(inline)]
pub use bones_asset as asset;

/// Math library.
#[doc(inline)]
pub use glam;

/// The prelude.
pub mod prelude {
    pub use crate::{
        animation::*, input::prelude::*, params::*, render::prelude::*, time::*, AssetServerExt,
        DefaultSessionPlugin,
    };
    pub use bones_asset::anyhow::Context;
    pub use bones_asset::prelude::*;
    pub use bones_lib::prelude::*;
    pub use glam::*;

    #[cfg(feature = "localization")]
    pub use crate::localization::*;
}

pub mod animation;
pub mod input;
pub mod params;
pub mod render;
pub mod time;

#[cfg(feature = "localization")]
pub mod localization;

/// Default plugins for bones framework sessions.
pub struct DefaultSessionPlugin;
impl lib::SessionPlugin for DefaultSessionPlugin {
    fn install(self, session: &mut lib::Session) {
        session
            .install_plugin(animation::animation_plugin)
            .install_plugin(render::render_plugin);
    }
}

/// Extension trait for the bones [`AssetServer`][bones_asset::AssetServer].
pub trait AssetServerExt {
    /// Register the default assets from `bones_framework`.
    fn register_default_assets(self) -> Self;
}
impl AssetServerExt for &mut bones_asset::AssetServer {
    fn register_default_assets(self) -> Self {
        use crate::prelude::*;

        self.register_asset::<Image>().register_asset::<Atlas>();

        #[cfg(feature = "localization")]
        {
            self.register_asset::<LocalizationAsset>()
                .register_asset::<FluentBundleAsset>()
                .register_asset::<FluentResourceAsset>();
        }

        #[cfg(feature = "ui")]
        self.register_asset::<Font>();

        self
    }
}
