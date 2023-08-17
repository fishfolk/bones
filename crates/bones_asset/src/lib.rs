//! An asset interface for Bones.

#![warn(missing_docs)]
// This cfg_attr is needed because `rustdoc::all` includes lints not supported on stable
#![cfg_attr(doc, allow(unknown_lints))]
#![deny(rustdoc::all)]

/// Helper to export the same types in the crate root and in the prelude.
macro_rules! pub_use {
    () => {
        pub use crate::{asset::*, cid::*, handle::*, io::*, path::*, server::*};
        pub use anyhow;
        pub use bones_schema::prelude::*;
        pub use semver::Version;
    };
}
pub_use!();

/// The prelude.
pub mod prelude {
    pub_use!();
}

mod asset;
mod cid;
mod handle;
mod io;
mod parse;
mod path;
mod server;
