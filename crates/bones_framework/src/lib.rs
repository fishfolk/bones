//! The bones framework for game development.

#![warn(missing_docs)]
// This cfg_attr is needed because `rustdoc::all` includes lints not supported on stable
#![cfg_attr(doc, allow(unknown_lints))]
#![deny(rustdoc::all)]

#[doc(inline)]
pub use bones_asset as asset;
#[doc(inline)]
pub use bones_lib as lib;

/// Math library.
#[doc(inline)]
pub use glam;

/// The prelude.
pub mod prelude {
    pub use crate::{
        animation::*, input::prelude::*, params::*, render::prelude::*, DefaultPlugins,
    };
    pub use bones_asset::prelude::*;
    pub use bones_lib::prelude::*;
    pub use glam::*;
}

pub mod animation;
pub mod input;
pub mod params;
pub mod render;

/// Default plugins for bones framework sessions.
pub struct DefaultPlugins;
impl lib::Plugin for DefaultPlugins {
    fn install(self, session: &mut lib::Session) {
        session.install_plugin(animation::plugin);
    }
}
