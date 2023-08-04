//! World resource storage.

use std::{
    alloc::{self, Layout},
    any::TypeId,
    io::Write,
    marker::PhantomData,
    mem,
    ptr::NonNull,
    sync::Arc,
};

use crate::prelude::*;

/// Storage for un-typed resources.
///
/// This is the backing data store used by [`Resources`].
///
/// Unless you are intending to do modding or otherwise need raw pointers to your resource data, you
/// should use [`Resources`] instead.
#[derive(Clone, Default)]
pub struct UntypedResources {
    resources: UlidMap<UntypedResource>,
}

/// Used to construct an [`UntypedResource`].
pub struct UntypedResourceInfo {
    /// The memory layout of the resource
    pub layout: Layout,
    /// Cell containing the raw pointer to the resource's data
    // TODO: Evaluate possibility of avoiding an `Arc` clone.
    // We might be able to just just pass references with a lifetime for acessing it,
    // instead of cloning the Arc like we do here.
    pub cell: Arc<AtomicRefCell<*mut u8>>,
    /// A function that may be called to clone the resource from one pointer to another.
    pub clone_fn: unsafe extern "C" fn(*const u8, *mut u8),
    /// An optional function that will be called to drop the resource.
    pub drop_fn: Option<unsafe extern "C" fn(*mut u8)>,
}

/// An untyped resource that may be inserted into [`UntypedResources`].
pub struct UntypedResource {
    layout: Layout,
    cell: Arc<AtomicRefCell<*mut u8>>,
    clone_fn: unsafe extern "C" fn(*const u8, *mut u8),
    drop_fn: Option<unsafe extern "C" fn(*mut u8)>,
}

impl UntypedResource {
    /// Create a new [`UntypedResource`] from raw [`UntypedResourceInfo`].
    ///
    /// # Safety
    ///
    /// - The implementations for `info.clone_fn` and `info.drop_fn` must not do anything unsound
    ///   when given valid pointers to clone or drop.
    /// - The `info.cell` must contain a pointer that is writable and points to data with a layout
    ///   matching `info.layout`.
    pub unsafe fn new_raw(info: UntypedResourceInfo) -> Self {
        UntypedResource {
            layout: info.layout,
            cell: info.cell,
            clone_fn: info.clone_fn,
            drop_fn: info.drop_fn,
        }
    }

    /// Creates a new [`UntypedResource`] from an instance of a Rust type.
    ///
    /// This is the safest way to construct a valid [`UntypedResource`].
    pub fn new<T: Clone + Sync + Send>(resource: T) -> Self {
        let layout = Layout::new::<T>();

        let ptr = Box::into_raw(Box::new(resource)) as *mut u8;

        Self {
            cell: Arc::new(AtomicRefCell::new(ptr)),
            clone_fn: T::raw_clone,
            drop_fn: Some(T::raw_drop),
            layout,
        }
    }
}

unsafe impl Sync for UntypedResource {}
unsafe impl Send for UntypedResource {}

impl Clone for UntypedResource {
    fn clone(&self) -> Self {
        let prev_ptr = self.cell.borrow();
        let new_ptr = if self.layout.size() == 0 {
            NonNull::<u8>::dangling().as_ptr()
        } else {
            // SAFE: Non-zero size for layout
            unsafe { std::alloc::alloc(self.layout) }
        };

        if new_ptr.is_null() {
            writeln!(
                std::io::stderr(),
                "Error alloating memory while cloning resource"
            )
            .ok();
            std::process::abort();
        }

        // SAFE: UntypedResource can only be constructed with an unsafe function where the user
        // promises not to do anything unsound while dropping.
        //
        // And our source prev_ptr is valid, and the new_ptr is properly aligned and writable.
        unsafe {
            (self.clone_fn)(*prev_ptr, new_ptr);
        }

        Self {
            cell: Arc::new(AtomicRefCell::new(new_ptr)),
            clone_fn: self.clone_fn,
            drop_fn: self.drop_fn,
            layout: self.layout,
        }
    }
}

impl Drop for UntypedResource {
    fn drop(&mut self) {
        if let Some(drop_fn) = self.drop_fn {
            let null_cell = Arc::new(AtomicRefCell::new(std::ptr::null_mut()));
            let cell = mem::replace(&mut self.cell, null_cell);

            let ptr = Arc::try_unwrap(cell)
                .expect(
                    "You must drop all references to Resoruces before dropping `UntypedResources`",
                )
                .into_inner();

            // SAFE: UntypedResource can only be constructed with an unsafe function where the user
            // promises not to do anything unsound while dropping.
            //
            // And our ptr is valid.
            unsafe {
                drop_fn(ptr);

                if self.layout.size() != 0 {
                    alloc::dealloc(ptr, self.layout);
                }
            }
        }
    }
}

impl UntypedResources {
    /// Create an empty [`UntypedResources`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a new resource
    pub fn insert(&mut self, uuid: Ulid, resource: UntypedResource) -> Option<UntypedResource> {
        self.resources.insert(uuid, resource)
    }

    /// Get a cell containing the resource data pointer for the given ID
    pub fn get(&self, uuid: Ulid) -> Option<Arc<AtomicRefCell<*mut u8>>> {
        self.resources.get(&uuid).map(|x| x.cell.clone())
    }

    /// Remove a resource
    pub fn remove(&mut self, uuid: Ulid) -> Option<UntypedResource> {
        self.resources.remove(&uuid)
    }
}

/// A collection of resources.
///
/// [`Resources`] is essentially a type-map
#[derive(Clone, Default)]
pub struct Resources {
    untyped: UntypedResources,
    type_ids: UlidMap<TypeId>,
}

impl Resources {
    /// Create an empty [`Resources`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a resource.
    ///
    /// # Panics
    ///
    /// Panics if you try to insert a Rust type with a different [`TypeId`], but the same
    /// [`TypeUlid`] as another resource in the store.
    #[track_caller]
    pub fn insert<T: TypedEcsData>(&mut self, resource: T) {
        self.try_insert(resource).unwrap();
    }

    /// Try to insert a resource.
    ///
    /// # Errors
    ///
    /// Errors if you try to insert a Rust type with a different [`TypeId`], but the same
    /// [`TypeUlid`] as another resource in the store.
    pub fn try_insert<T: TypedEcsData>(&mut self, resource: T) -> Result<(), EcsError> {
        let uuid = T::ULID;
        let type_id = TypeId::of::<T>();

        match self.type_ids.entry(uuid) {
            std::collections::hash_map::Entry::Occupied(entry) => {
                if entry.get() != &type_id {
                    return Err(EcsError::TypeUlidCollision);
                }

                self.untyped.insert(uuid, UntypedResource::new(resource));
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(type_id);
                self.untyped.insert(uuid, UntypedResource::new(resource));
            }
        }

        Ok(())
    }

    /// Get a resource handle from the store.
    ///
    /// This is not the resource itself, but a handle, may be cloned cheaply.
    ///
    /// In order to access the resource you must call [`borrow()`][AtomicResource::borrow] or
    /// [`borrow_mut()`][AtomicResource::borrow_mut] on the returned value.
    ///
    /// # Panics
    ///
    /// Panics if the resource does not exist in the store.
    #[track_caller]
    pub fn get<T: TypedEcsData>(&self) -> AtomicResource<T> {
        self.try_get().unwrap()
    }

    /// Check whether or not a resource is in the store.
    ///
    /// See [get()][Self::get]
    pub fn contains<T: TypedEcsData>(&self) -> bool {
        self.untyped.resources.contains_key(&T::ULID)
    }

    /// Gets a resource handle from the store if it exists.
    pub fn try_get<T: TypedEcsData>(&self) -> Option<AtomicResource<T>> {
        let untyped = self.untyped.get(T::ULID)?;

        Some(AtomicResource {
            untyped,
            _phantom: PhantomData,
        })
    }

    /// Borrow the underlying [`UntypedResources`] store.
    pub fn untyped(&self) -> &UntypedResources {
        &self.untyped
    }
    /// Mutably borrow the underlying [`UntypedResources`] store.
    pub fn untyped_mut(&mut self) -> &mut UntypedResources {
        &mut self.untyped
    }
    /// Consume [`Resources`] and extract the underlying [`UntypedResources`].
    pub fn into_untyped(self) -> UntypedResources {
        self.untyped
    }
}

/// A handle to a resource from a [`Resources`] collection.
///
/// This is not the resource itself, but a cheaply clonable handle to it.
///
/// To access the resource you must borrow it with either [`borrow()`][Self::borrow] or
/// [`borrow_mut()`][Self::borrow_mut].
pub struct AtomicResource<T: TypedEcsData> {
    untyped: Arc<AtomicRefCell<*mut u8>>,
    _phantom: PhantomData<T>,
}

impl<T: TypedEcsData> AtomicResource<T> {
    /// Lock the resource for reading.
    ///
    /// This returns a read guard, very similar to an [`RwLock`][std::sync::RwLock].
    pub fn borrow(&self) -> AtomicRef<T> {
        let borrow = self.untyped.borrow();
        // SAFE: We know that the data pointer is valid for type T.
        AtomicRef::map(borrow, |data| unsafe { &*data.cast::<T>() })
    }

    /// Lock the resource for read-writing.
    ///
    /// This returns a write guard, very similar to an [`RwLock`][std::sync::RwLock].
    pub fn borrow_mut(&self) -> AtomicRefMut<T> {
        let borrow = self.untyped.borrow_mut();
        // SAFE: We know that the data pointer is valid for type T.
        AtomicRefMut::map(borrow, |data| unsafe { &mut *data.cast::<T>() })
    }
}

#[cfg(test)]
mod test {
    use crate::prelude::*;

    #[test]
    fn sanity_check() {
        #[derive(TypeUlid, Clone, Debug)]
        #[ulid = "01GNDJJ42DY5GSP9PGXPGF65GE"]
        struct A(Vec<u32>);

        #[derive(TypeUlid, Clone, Debug)]
        #[ulid = "01GNDJJDS0JRRN6TP3S89R4FQB"]
        struct B(u32);

        let mut resources = Resources::new();

        resources.insert(A(vec![3, 2, 1]));
        assert_eq!(resources.get::<A>().borrow_mut().0, vec![3, 2, 1]);

        let r2 = resources.clone();

        resources.insert(A(vec![4, 5, 6]));
        resources.insert(A(vec![7, 8, 9]));
        assert_eq!(resources.get::<A>().borrow().0, vec![7, 8, 9]);

        // TODO: Create more focused test for cloning resources.
        assert_eq!(r2.get::<A>().borrow().0, vec![3, 2, 1]);

        resources.insert(B(1));
        assert_eq!(resources.get::<B>().borrow().0, 1);
        resources.insert(B(2));
        assert_eq!(resources.get::<B>().borrow().0, 2);
        assert_eq!(resources.get::<A>().borrow().0, vec![7, 8, 9]);
    }
}
