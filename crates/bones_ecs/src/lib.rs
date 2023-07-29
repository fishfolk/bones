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
pub use bones_schema as reflect;

mod error;
pub use error::EcsError;

mod world;
pub use world::{FromWorld, World};

/// The prelude.
pub mod prelude {
    pub use {
        atomic_refcell::*, bitset_core::BitSet, bones_schema::prelude::*, bones_utils::prelude::*,
    };

    pub use crate::{
        bitset::*, components::*, entities::*, error::*, resources::*, stage::*, system::*,
        FromWorld, UnwrapMany, World,
    };

    // Make bones_schema available for derive macros
    pub use bones_schema;
}

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
