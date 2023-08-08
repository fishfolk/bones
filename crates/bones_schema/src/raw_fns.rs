//! Traits implementing raw function calls for cloning, dropping, and creating default values for
//! Rust types.

use std::hash::{Hash, Hasher};

use bones_utils::fxhash::FxHasher;

/// Trait implemented automatically for types that implement [`Clone`] and can be used to clone the
/// type through raw pointers.
pub trait RawClone {
    /// Write the default value of the type to the pointer.
    ///
    /// # Safety
    ///
    /// The `dst` pointer must be aligned, writable, and have the same layout that this function is
    /// assocated to, and the `src` pointer must be readable and point to a valid instance of the
    /// type that this function is associated with.
    unsafe extern "C-unwind" fn raw_clone(src: *const u8, dst: *mut u8);
}
impl<T: Clone> RawClone for T {
    unsafe extern "C-unwind" fn raw_clone(src: *const u8, dst: *mut u8) {
        let t = &*(src as *const T);
        let t = t.clone();
        (dst as *mut T).write(t)
    }
}

/// Trait implemented automatically for types that implement [`Drop`] and can be used to drop the
/// type through a raw pointer.
pub trait RawDrop {
    /// Write the default value of the type to the pointer.
    ///
    /// # Safety
    ///
    /// The pointer must be aligned, writable, and have the same layout that this function is
    /// assocated to.
    unsafe extern "C-unwind" fn raw_drop(ptr: *mut u8);
}
impl<T> RawDrop for T {
    unsafe extern "C-unwind" fn raw_drop(ptr: *mut u8) {
        if std::mem::needs_drop::<T>() {
            (ptr as *mut T).drop_in_place()
        }
    }
}

/// Trait implemented automatically for types that implement [`Hash`] and can be used compute a
/// [`u64`] hash for a type using a pointer to it.
pub trait RawHash {
    /// Get the hash of the type.
    ///
    /// # Safety
    ///
    /// The pointer must be aligned, readable, and be a pointer to the type that this Hash function
    /// was created for.
    unsafe extern "C-unwind" fn raw_hash(ptr: *const u8) -> u64;
}
impl<T: Hash> RawHash for T {
    unsafe extern "C-unwind" fn raw_hash(ptr: *const u8) -> u64 {
        let this = unsafe { &*(ptr as *const Self) };
        let mut hasher = FxHasher::default();
        this.hash(&mut hasher);
        hasher.finish()
    }
}

/// Trait implemented automatically for types that implement [`Eq`] that can compare two values
/// through their pointers.
pub trait RawEq {
    /// Get the hash of the type.
    ///
    /// # Safety
    ///
    /// The pointer must be aligned, readable, and be a pointer to the type that this Hash function
    /// was created for.
    unsafe extern "C-unwind" fn raw_eq(a: *const u8, b: *const u8) -> bool;
}
impl<T: Eq> RawEq for T {
    unsafe extern "C-unwind" fn raw_eq(a: *const u8, b: *const u8) -> bool {
        let a = unsafe { &*(a as *const Self) };
        let b = unsafe { &*(b as *const Self) };
        a.eq(b)
    }
}

/// Trait implemented automatically for types that implement [`Default`] and can be used to write
/// the default value of the type to a pointer.
pub trait RawDefault {
    /// Write the default value of the type to the pointer.
    ///
    /// # Safety
    ///
    /// The pointer must be aligned, writable, and have the same layout that this function is
    /// assocated to.
    unsafe extern "C-unwind" fn raw_default(dst: *mut u8);
}
impl<T: Default> RawDefault for T {
    unsafe extern "C-unwind" fn raw_default(dst: *mut u8) {
        let d = T::default();
        (dst as *mut T).write(d)
    }
}
