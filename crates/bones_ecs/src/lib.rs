#![doc = include_str!("../README.md")]
#![cfg_attr(doc, allow(unknown_lints))]
#![deny(rustdoc::all)]
#![warn(missing_docs)]

pub mod atomic {
    //! Atomic Refcell implmentation.
    //!
    //! Atomic Refcells are from the [`atomic_refcell`] crate.
    //!
    //! [`atomic_refcell`]: https://docs.rs/atomic_refcell
    pub use atomic_refcell::*;
}
pub mod bitset;
pub mod components;
pub mod entities;
pub mod resources;
pub mod stage;
pub mod system;

mod error;
pub use error::EcsError;

mod world;
pub use world::{FromWorld, World};

/// The prelude.
pub mod prelude {
    pub use {
        atomic_refcell::*,
        bitset_core::BitSet,
        bones_ecs_macros::*,
        bones_reflect::RawFns,
        bones_utils::HashMap,
        type_ulid::{TypeUlid, Ulid},
    };

    pub use crate::{
        bitset::*, components::*, default, entities::*, error::*, resources::*, stage::*,
        system::*, EcsData, FromWorld, TypedEcsData, UnwrapMany, World,
    };
}

/// Helper trait that is auto-implemented for anything that may be stored in the ECS's untyped
/// storage.
///
/// Examples of untyped storage are [`UntypedResources`][crate::resources::UntypedResources] and
/// [`UntypedComponentStore`][crate::components::UntypedComponentStore].
pub trait EcsData: Clone + Sync + Send + 'static {}
impl<T: Clone + Sync + Send + 'static> EcsData for T {}

/// Helper trait that is auto-implemented for anything that may be stored in the ECS's typed
/// storage.
///
/// Examples of typed storage are [`Resources<T>`][crate::resources::Resources] and
/// [`ComponentStore<T>`][crate::components::ComponentStore].
pub trait TypedEcsData: type_ulid::TypeUlid + EcsData {}
impl<T: type_ulid::TypeUlid + EcsData> TypedEcsData for T {}

/// Helper trait for unwraping each item in an array.
///
/// # Example
///
/// ```
/// # use bones_ecs::UnwrapMany;
/// let data = [Some(1), Some(2)];
/// let [data1, data2] = data.unwrap_many();
/// ```
pub trait UnwrapMany<const N: usize, T> {
    /// Unwrap all the items in an array.
    fn unwrap_many(self) -> [T; N];
}

impl<const N: usize, T> UnwrapMany<N, T> for [Option<T>; N] {
    fn unwrap_many(self) -> [T; N] {
        let mut iter = self.into_iter();
        std::array::from_fn(|_| iter.next().unwrap().unwrap())
    }
}
impl<const N: usize, T, E: std::fmt::Debug> UnwrapMany<N, T> for [Result<T, E>; N] {
    fn unwrap_many(self) -> [T; N] {
        let mut iter = self.into_iter();
        std::array::from_fn(|_| iter.next().unwrap().unwrap())
    }
}

/// Free-standing, shorter equivalent to [`Default::default()`].
#[inline]
pub fn default<T: Default>() -> T {
    std::default::Default::default()
}
