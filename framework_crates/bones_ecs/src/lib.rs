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
    pub use atomicell::*;
}
pub mod bitset;
pub mod components;
pub mod entities;
pub mod resources;
pub mod stage;
pub mod system;

pub use bones_schema as schema;
pub use bones_utils as utils;

mod world;
pub use world::{FromWorld, World};

/// The prelude.
pub mod prelude {
    pub use {
        atomicell::*, bitset_core::BitSet, bones_schema::prelude::*, bones_utils::prelude::*,
    };

    pub use crate::{
        bitset::*,
        components::*,
        entities::*,
        resources::*,
        stage::{CoreStage::*, *},
        system::*,
        FromWorld, UnwrapMany, World,
    };

    #[cfg(feature = "derive")]
    pub use bones_ecs_macros::*;

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

#[cfg(test)]
mod test {
    use crate::prelude::*;

    #[test]
    fn insert_comp_with_gap() {
        let w = World::new();

        #[derive(HasSchema, Default, Clone)]
        #[repr(C)]
        struct MyComp(u32, u32, u32);

        w.run_system(
            |mut entities: ResMut<Entities>, mut comps: CompMut<MyComp>| {
                for _ in 0..3 {
                    entities.create();
                }

                let e = entities.create();
                comps.insert(e, default());
            },
            (),
        )
    }
}
