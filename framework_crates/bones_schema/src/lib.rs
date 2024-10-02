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
    pub use crate::ser_de::*;
    pub use crate::{
        alloc::{SMap, SVec, SchemaMap, SchemaVec},
        desync_hash::*,
        ptr::*,
        registry::*,
        schema::*,
    };
    #[cfg(feature = "derive")]
    pub use bones_schema_macros::*;
    pub use bones_utils;
    pub use ulid::Ulid;
}

mod schema;
pub use schema::*;

pub mod alloc;
pub mod desync_hash;
pub mod ptr;
pub mod raw_fns;
pub mod registry;

/// Implementations of [`HasSchema`] for standard types.
mod std_impls;

/// Serde implementations for [`Schema`].
#[cfg(feature = "serde")]
pub mod ser_de;

#[cfg(test)]
mod test {
    #[cfg(feature = "derive")]
    mod derive_test {
        #![allow(dead_code)]

        use crate::prelude::*;

        #[derive(HasSchema, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Debug)]
        #[schema_module(crate)]
        #[repr(C, u8)]
        pub enum Maybe<T> {
            /// The value is not set.
            #[default]
            Unset,
            /// The value is set.
            Set(T),
        }

        #[derive(HasSchema, Clone, Copy, Debug, PartialEq, Eq, Default)]
        #[schema_module(crate)]
        #[repr(u8)]
        pub enum E {
            #[default]
            None,
            L,
            R,
            U,
            D,
            G,
            S,
        }

        /// Represents the ball in the game
        #[derive(HasSchema, Clone, Default)]
        #[schema_module(crate)]
        pub struct A {
            pub c: Maybe<u32>,
            pub d: SVec<u64>,
            pub e: Maybe<u64>,
            pub f: u32,
            pub g: f32,
            pub h: f32,
            pub i: E,
            pub j: u32,
            pub k: u32,
        }

        #[derive(HasSchema, Clone, Default)]
        #[schema_module(crate)]
        #[repr(C)]
        pub struct B {
            pub c: Maybe<u32>,
            pub d: SVec<u64>,
            pub e: Maybe<u64>,
            pub f: u32,
            pub g: f32,
            pub h: f32,
            pub i: E,
            pub j: u32,
            pub k: u32,
        }

        #[derive(HasSchema, Clone, Default)]
        #[schema_module(crate)]
        pub struct C {
            pub c: Maybe<u32>,
            pub e: Maybe<u64>,
        }

        #[derive(HasSchema, Clone, Default)]
        #[schema_module(crate)]
        #[repr(C)]
        pub struct D {
            pub c: Maybe<u32>,
            pub e: Maybe<u64>,
        }

        #[derive(HasSchema, Clone)]
        #[schema(no_default)]
        #[schema_module(crate)]
        #[repr(C)]
        struct F<T> {
            a: bool,
            b: T,
        }

        /// Makes sure that the layout reported in the schema for a generic type matches the layout
        /// reported by Rust, for two different type parameters.
        #[test]
        fn generic_type_schema_layouts_match() {
            assert_eq!(
                Maybe::<u32>::schema().layout(),
                std::alloc::Layout::new::<Maybe<u32>>()
            );
            assert_eq!(
                Maybe::<u64>::schema().layout(),
                std::alloc::Layout::new::<Maybe<u64>>()
            );
            assert_eq!(
                F::<u64>::schema().layout(),
                std::alloc::Layout::new::<F<u64>>()
            );
            assert_eq!(
                F::<u32>::schema().layout(),
                std::alloc::Layout::new::<F<u32>>()
            );

            // Check a normal enum too, just in case.
            assert_eq!(E::schema().layout(), std::alloc::Layout::new::<E>());
        }

        // Makes sure that the layout reported for two structs, where the only difference between
        // them is the `#[repr(C)]` annotation, matches.
        #[test]
        fn schema_layout_for_repr_c_matches_repr_rust() {
            assert_eq!(A::schema().layout(), B::schema().layout());
            assert_eq!(C::schema().layout(), D::schema().layout());
        }
    }
}
