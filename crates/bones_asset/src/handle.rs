use std::marker::PhantomData;

use ulid::Ulid;

/// A typed handle to an asset.
#[repr(C)]
pub struct Handle<T> {
    /// The runtime ID of the asset.
    pub id: Ulid,
    phantom: PhantomData<*const T>,
}

// Manually implement these traits we normally derive because the derive assumes that `T` must also
// implement these traits.
impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            phantom: self.phantom,
        }
    }
}
impl<T> Copy for Handle<T> {}
impl<T> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl<T> Eq for Handle<T> {}
impl<T> std::hash::Hash for Handle<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}
impl<T> Default for Handle<T> {
    fn default() -> Self {
        Self {
            id: Default::default(),
            phantom: Default::default(),
        }
    }
}

impl<T> std::fmt::Debug for Handle<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Handle").field("id", &self.id).finish()
    }
}

impl<T> Handle<T> {
    /// Convert the handle to an [`UntypedHandle`].
    pub fn untyped(self) -> UntypedHandle {
        UntypedHandle { rid: self.id }
    }
}

/// An untyped handle to an asset.
#[derive(Default, Clone, Debug, Hash, PartialEq, Eq, Copy)]
#[repr(C)]
pub struct UntypedHandle {
    /// The runtime ID of the handle
    pub rid: Ulid,
}

impl UntypedHandle {
    /// Create a typed [`Handle<T>`] from this [`UntypedHandle`].
    pub fn typed<T>(self) -> Handle<T> {
        Handle {
            id: self.rid,
            phantom: PhantomData,
        }
    }
}
