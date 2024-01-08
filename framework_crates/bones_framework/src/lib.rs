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
        animation::*, input::prelude::*, params::*, render::prelude::*,
        storage::*, time::*, utils::*, AssetServerExt, DefaultGamePlugin, DefaultSessionPlugin,
    };

    #[cfg(not(target_arch = "wasm32"))]
    pub use crate::networking::prelude::*;

    pub use bones_asset::anyhow::Context;
    pub use bones_asset::prelude::*;
    pub use bones_lib::prelude::*;
    pub use glam::*;

    #[cfg(feature = "scripting")]
    pub use bones_scripting::prelude::*;

    #[cfg(feature = "localization")]
    pub use crate::localization::*;
}

pub mod animation;
pub mod input;
pub mod params;
pub mod render;
pub mod storage;
pub mod time;
pub mod utils;

#[cfg(not(target_arch = "wasm32"))]
pub mod networking;

#[cfg(feature = "scripting")]
pub use bones_scripting as scripting;

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

/// Default plugins for bones framework games.
pub struct DefaultGamePlugin;
impl lib::GamePlugin for DefaultGamePlugin {
    fn install(self, game: &mut lib::Game) {
        game.install_plugin(render::audio::game_plugin);

        #[cfg(feature = "scripting")]
        game.install_plugin(bones_scripting::ScriptingGamePlugin::default());
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

        // Register asset schemas
        Image::register_schema();
        Atlas::register_schema();

        #[cfg(feature = "localization")]
        {
            LocalizationAsset::register_schema();
            FluentBundleAsset::register_schema();
            FluentResourceAsset::register_schema();
        }

        #[cfg(feature = "ui")]
        Font::register_schema();

        self
    }
}
