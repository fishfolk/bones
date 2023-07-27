//! World resource storage.

use std::{any::TypeId, marker::PhantomData, sync::Arc};

use crate::{prelude::*, SCHEMA_NOT_REGISTERED};

/// Storage for un-typed resources.
///
/// This is the backing data store used by [`Resources`].
///
/// Unless you are intending to do modding or otherwise need raw pointers to your resource data, you
/// should use [`Resources`] instead.
#[derive(Clone, Default)]
pub struct UntypedResources {
    resources: HashMap<SchemaId, UntypedResource>,
}

/// An untyped resource that may be inserted into [`UntypedResources`].
pub struct UntypedResource {
    cell: Arc<AtomicRefCell<SchemaBox>>,
}

impl UntypedResource {
    /// Creates a new [`UntypedResource`] storing the given data.
    pub fn new<T: HasSchema>(resource: T) -> Self {
        Self {
            cell: Arc::new(AtomicRefCell::new(SchemaBox::new(resource))),
        }
    }

    /// Create a new [`UntypedResource`] for the given schema, initially populated with the default
    /// value for the schema.
    pub fn from_schema<S: Into<MaybeOwned<'static, Schema>>>(schema: S) -> Self {
        Self {
            cell: Arc::new(AtomicRefCell::new(SchemaBox::default(schema.into()))),
        }
    }

    /// Get another [`UntypedResource`] that points to the same data,
    pub fn clone_cell(&self) -> UntypedResource {}
}

impl Clone for UntypedResource {
    fn clone(&self) -> Self {
        Self {
            cell: Arc::new(AtomicRefCell::new(self.cell.borrow().clone())),
        }
    }
}

impl UntypedResources {
    /// Create an empty [`UntypedResources`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a new resource
    pub fn insert(&mut self, resource: UntypedResource) -> Option<UntypedResource> {
        self.resources
            .insert(resource.cell.borrow().schema().id.unwrap(), resource)
    }

    /// Get a cell containing the resource data pointer for the given ID
    pub fn get(&self, schema_id: SchemaId) -> Option<Arc<AtomicRefCell<*mut u8>>> {
        self.resources.get(&schema_id).map(|x| x.cell.clone())
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
}

impl Resources {
    /// Create an empty [`Resources`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a resource.
    pub fn insert<T: HasSchema>(&mut self, resource: T) {
        let schema = T::schema();
        let type_id = TypeId::of::<T>();

        self.untyped.insert(UntypedResource::new(resource));
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
    pub fn get<T: HasSchema>(&self) -> AtomicResource<T> {
        self.try_get().unwrap()
    }

    /// Check whether or not a resource is in the store.
    ///
    /// See [get()][Self::get]
    pub fn contains<T: HasSchema>(&self) -> bool {
        T::schema()
            .id
            .map(|id| self.untyped.resources.contains_key(&id))
            .unwrap_or(false)
    }

    /// Gets a resource handle from the store if it exists.
    pub fn try_get<T: HasSchema>(&self) -> Option<AtomicResource<T>> {
        let untyped = self
            .untyped
            .get(T::schema().id.expect(SCHEMA_NOT_REGISTERED))?;

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
pub struct AtomicResource<T: HasSchema> {
    untyped: Arc<AtomicRefCell<*mut u8>>,
    _phantom: PhantomData<T>,
}

impl<T: HasSchema> AtomicResource<T> {
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
        #[derive(HasSchema, Clone, Debug, Default)]
        #[repr(C)]
        struct A(Vec<u32>);

        #[derive(HasSchema, Clone, Debug, Default)]
        #[repr(C)]
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
