//! Simple reflection system based on the `#[repr(C)]` memory layout.
//!
//! You can derive [`HasSchema`] for your Rust types to unlock integration with the `bones_schema`
//! ecosystem, including `bones_ecs` and `bones_asset`.

#![warn(missing_docs)]
// This cfg_attr is needed because `rustdoc::all` includes lints not supported on stable
#![cfg_attr(doc, allow(unknown_lints))]
#![deny(rustdoc::all)]
// This allows us to use our stable polyfills for nightly APIs under the same name.
#![allow(unstable_name_collisions)]

// import the macros if the derive feature is enabled.
#[cfg(feature = "derive")]
pub use bones_schema_macros::*;

/// The prelude.
pub mod prelude {
    #[cfg(feature = "serde")]
    pub use crate::ser_de::SchemaDeserialize;
    pub use crate::{
        alloc::{SMap, SVec, SchemaMap, SchemaVec},
        ptr::*,
        registry::*,
        schema::*,
        Maybe,
        Maybe::*,
    };
    #[cfg(feature = "derive")]
    pub use bones_schema_macros::*;
    pub use bones_utils;
    pub use ulid::Ulid;
}

mod schema;
pub use schema::*;

pub mod alloc;
pub mod ptr;
pub mod raw_fns;
pub mod registry;

/// Implementations of [`HasSchema`] for standard types.
mod std_impls;

/// Serde implementations for [`Schema`].
#[cfg(feature = "serde")]
pub mod ser_de;

/// An equivalent to [`Option<T>`] that has a stable memory layout and implements [`HasSchema`].
#[derive(HasSchema, Clone, Default)]
#[schema_module(crate)]
#[repr(C, u8)]
pub enum Maybe<T> {
    /// The value is set.
    Set(T),
    /// The value is not set.
    #[default]
    Unset,
}

impl<T> Maybe<T> {
    /// Convert this [`Maybe`] into an [`Option`].
    pub fn option(self) -> Option<T> {
        self.into()
    }
}

impl<T> From<Maybe<T>> for Option<T> {
    fn from(value: Maybe<T>) -> Self {
        match value {
            Maybe::Set(s) => Some(s),
            Maybe::Unset => None,
        }
    }
}

impl<T> From<Option<T>> for Maybe<T> {
    fn from(value: Option<T>) -> Self {
        match value {
            Some(s) => Maybe::Set(s),
            None => Maybe::Unset,
        }
    }
}
