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
    pub use crate::{input::prelude::*, render::prelude::*};
    pub use bones_lib::prelude::*;
    pub use glam::*;
}

pub mod input;
pub mod render;
