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
pub mod ulid;

mod error;
pub use error::EcsError;

mod world;
pub use world::World;

/// The prelude.
pub mod prelude {
    pub use {
        atomic_refcell::*,
        bevy_derive::{Deref, DerefMut},
        bitset_core::BitSet,
        type_ulid::{TypeUlid, Ulid},
    };

    pub use crate::{
        bitset::*, components::*, default, entities::*, error::*, resources::*, stage::*,
        system::*, ulid::*, EcsData, RawFns, TypedEcsData, UnwrapMany, World,
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

/// Helper trait that is auto-implemented for all `Clone`-able types. Provides easy access to drop
/// and clone funcitons for raw pointers.
///
/// This simply serves as a convenient way to obtain a drop/clone function implementation for
/// [`UntypedResource`][crate::resources::UntypedResource] or
/// [`UntypedComponentStore`][crate::components::UntypedComponentStore].
///
/// > **Note:** This is an advanced feature that you don't need if you aren't working with some sort
/// > of scripting or otherwise untyped data access.
///
/// # Example
///
/// ```
/// # use bones_ecs::prelude::*;
/// # use core::alloc::Layout;
/// let components = unsafe {
///     UntypedComponentStore::new(Layout::new::<String>(), String::raw_clone, Some(String::raw_drop));
/// };
/// ```
pub trait RawFns {
    /// Drop the value at `ptr`.
    ///
    /// # Safety
    /// - The pointer must point to a valid instance of the type that this implementation is
    /// assocated with.
    /// - The pointer must be writable.
    unsafe extern "C" fn raw_drop(ptr: *mut u8);

    /// Clone the value at `src`, writing the new value to `dst`.
    ///
    /// # Safety
    /// - The src pointer must point to a valid instance of the type that this implementation is
    /// assocated with.
    /// - The destination pointer must be properly aligned and writable.
    unsafe extern "C" fn raw_clone(src: *const u8, dst: *mut u8);
}

impl<T: Clone> RawFns for T {
    unsafe extern "C" fn raw_drop(ptr: *mut u8) {
        use std::io::{self, Write};

        let result = std::panic::catch_unwind(|| {
            if std::mem::needs_drop::<T>() {
                (ptr as *mut T).drop_in_place()
            }
        });

        if result.is_err() {
            writeln!(
                io::stderr(),
                "Rust type {} panicked in destructor.\n\
                Unable to panic across C ABI: aborting.",
                std::any::type_name::<T>()
            )
            .ok();
            std::process::abort();
        }
    }

    unsafe extern "C" fn raw_clone(src: *const u8, dst: *mut u8) {
        use std::io::{self, Write};

        let result = std::panic::catch_unwind(|| {
            let t = &*(src as *const T);
            let t = t.clone();
            (dst as *mut T).write(t)
        });

        if result.is_err() {
            writeln!(
                io::stderr(),
                "Rust type {} panicked in clone implementation.\n\
                Unable to panic across C ABI: aborting.",
                std::any::type_name::<T>()
            )
            .ok();
            std::process::abort();
        }
    }
}

/// Free-standing, shorter equivalent to [`Default::default()`].
#[inline]
pub fn default<T: Default>() -> T {
    std::default::Default::default()
}
