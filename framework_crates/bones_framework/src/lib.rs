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
        animation::*, input::prelude::*, params::*, render::prelude::*, storage::*, time::*,
        utils::*, AssetServerExt, DefaultGamePlugin, DefaultSessionPlugin, ExitBones,
    };

    #[cfg(feature = "ui")]
    pub use crate::debug;

    #[cfg(not(target_arch = "wasm32"))]
    pub use crate::networking::prelude::*;

    pub use bones_asset::anyhow::Context;
    pub use bones_asset::prelude::*;
    pub use bones_lib::prelude::*;
    pub use glam::*;

    pub use serde::{Deserialize, Serialize};

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

#[cfg(feature = "ui")]
pub mod debug;

#[cfg(not(target_arch = "wasm32"))]
pub mod networking;

#[cfg(feature = "scripting")]
pub use bones_scripting as scripting;

#[cfg(feature = "localization")]
pub mod localization;

/// External crate documentation.
///
/// This module only exists during docs builds and serves to make it eaiser to link to relevant
/// documentation in external crates.
#[cfg(doc)]
pub mod external {
    #[doc(inline)]
    pub use ggrs;

    #[doc(inline)]
    pub use bones_matchmaker_proto;
}

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
    #[allow(unused_variables)]
    fn install(self, game: &mut lib::Game) {
        #[cfg(feature = "audio")]
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

/// Resource for exiting bones games.
///
/// ## Notes
/// This has to be supported by the platform that is using bones.
/// Though it is not enforced, whether or not the platform inserts
/// this as a shared resource on your bones game should determine
/// whether or not it supports an exit functionality.
#[derive(bones_schema::HasSchema, Default, Clone)]
pub struct ExitBones(pub bool);

impl std::ops::Deref for ExitBones {
    type Target = bool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::ops::DerefMut for ExitBones {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
