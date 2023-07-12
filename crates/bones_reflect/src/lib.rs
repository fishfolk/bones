pub mod registry;
pub mod schema;

pub use bones_reflect_macros::*;

pub mod prelude {
    pub use {crate::registry::*, crate::schema::*, crate::RawFns, bones_reflect_macros::*};
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
