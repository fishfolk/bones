use std::ptr::NonNull;

use bevy_ptr::{Ptr, PtrMut};

/// Extension trait with utils for [`PtrMut`].
pub trait PtrMutExt {
    /// Unsafely alter the lifetime of this [`PtrMut`].
    /// # Safety
    /// You must ensure that the data referenced by this pointer will be valid for the new lifetime.
    unsafe fn transmute_lifetime<'new>(self) -> PtrMut<'new>;
}
impl<'a> PtrMutExt for PtrMut<'a> {
    unsafe fn transmute_lifetime<'new>(self) -> PtrMut<'new> {
        PtrMut::new(NonNull::new_unchecked(self.as_ptr()))
    }
}

/// Extension trait with utils for [`Ptr`].
pub trait PtrExt {
    /// Unsafely alter the lifetime of this [`Ptr`].
    /// # Safety
    /// You must ensure that the data referenced by this pointer will be valid for the new lifetime.
    unsafe fn transmute_lifetime<'new>(self) -> Ptr<'new>;
}
impl<'a> PtrExt for Ptr<'a> {
    unsafe fn transmute_lifetime<'new>(self) -> Ptr<'new> {
        Ptr::new(NonNull::new_unchecked(self.as_ptr()))
    }
}
