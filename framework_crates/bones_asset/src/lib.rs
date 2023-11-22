//! An asset interface for Bones.

#![warn(missing_docs)]
// This cfg_attr is needed because `rustdoc::all` includes lints not supported on stable
#![cfg_attr(doc, allow(unknown_lints))]
#![deny(rustdoc::all)]

use serde::{de::DeserializeSeed, Deserializer};

/// Helper to export the same types in the crate root and in the prelude.
macro_rules! pub_use {
    () => {
        pub use crate::{asset::*, cid::*, handle::*, io::*, server::*};
        pub use anyhow;
        pub use bones_schema::prelude::*;
        pub use dashmap;
        pub use path_absolutize::Absolutize;
        pub use semver::Version;
    };
}
pub_use!();

/// The prelude.
pub mod prelude {
    pub_use!();
    pub use super::{Maybe, Maybe::*};
}

mod asset;
mod cid;
mod handle;
mod io;
mod parse;
mod server;

/// An equivalent to [`Option<T>`] that has a stable memory layout and implements [`HasSchema`].
#[derive(HasSchema, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Debug)]
#[type_data(SchemaMetaAssetLoader(maybe_loader))]
#[repr(C, u8)]
pub enum Maybe<T> {
    /// The value is not set.
    #[default]
    Unset,
    /// The value is set.
    Set(T),
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

fn maybe_loader(
    ctx: &mut MetaAssetLoadCtx,
    ptr: SchemaRefMut<'_>,
    deserialzer: &mut dyn erased_serde::Deserializer,
) -> anyhow::Result<()> {
    deserialzer.deserialize_option(MaybeVisitor { ctx, ptr })?;

    Ok(())
}

struct MaybeVisitor<'a, 'srv> {
    ctx: &'a mut MetaAssetLoadCtx<'srv>,
    ptr: SchemaRefMut<'a>,
}

impl<'a, 'srv, 'de> serde::de::Visitor<'de> for MaybeVisitor<'a, 'srv> {
    type Value = ();

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "an optional value")
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(())
    }
    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(())
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Write the enum discriminant for the `Set` variant
        // SOUND: we know the discriminant due to the `#[repr(C, u8)]` annotation.
        unsafe {
            self.ptr.as_ptr().cast::<u8>().write(1);
        }

        // Get the pointer to the enum value
        let value_offset = self.ptr.schema().field_offsets()[0].1;
        // NOTE: we take the schema of the first argument of the second enum variant of the
        // [`Maybe`] enum because we know that the `Set` variant only has one argument at offset 0
        // and we actually want to deserialize the inner type, not a typle of length zero.
        let value_schema = self.ptr.schema().kind.as_enum().unwrap().variants[1]
            .schema
            .kind
            .as_struct()
            .unwrap()
            .fields[0]
            .schema;
        // SOUND: the schema asserts this is valid.
        let value_ref = unsafe {
            SchemaRefMut::from_ptr_schema(self.ptr.as_ptr().add(value_offset), value_schema)
        };

        // Load the enum value
        SchemaPtrLoadCtx {
            ctx: self.ctx,
            ptr: value_ref,
        }
        .deserialize(deserializer)
    }
}
