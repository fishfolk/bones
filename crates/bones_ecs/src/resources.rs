//! World resource storage.

use std::{marker::PhantomData, sync::Arc};

use crate::prelude::*;

/// Storage for un-typed resources.
///
/// This is the backing data store used by [`Resources`].
///
/// Unless you are intending to do modding or otherwise need raw pointers to your resource data, you
/// should use [`Resources`] instead.
#[derive(Clone, Default)]
pub struct UntypedResources {
    resources: HashMap<SchemaId, UntypedAtomicResource>,
}

/// An untyped resource that may be inserted into [`UntypedResources`].
#[derive(Deref, DerefMut)]
pub struct UntypedAtomicResource {
    #[deref]
    cell: Arc<AtomicRefCell<SchemaBox>>,
    schema: &'static Schema,
}

impl UntypedAtomicResource {
    /// Creates a new [`UntypedResource`] storing the given data.
    pub fn new<T: HasSchema>(resource: T) -> Self {
        Self {
            cell: Arc::new(AtomicRefCell::new(SchemaBox::new(resource))),
            schema: T::schema(),
        }
    }

    /// Create a new [`UntypedResource`] for the given schema, initially populated with the default
    /// value for the schema.
    pub fn from_schema(schema: &'static Schema) -> Self {
        Self {
            cell: Arc::new(AtomicRefCell::new(SchemaBox::default(schema))),
            schema,
        }
    }

    /// Get another [`UntypedAtomicResource`] that points to the same data.
    pub fn clone_cell(&self) -> UntypedAtomicResource {
        Self {
            cell: self.cell.clone(),
            schema: self.schema,
        }
    }

    /// Get the schema of the resource.
    pub fn schema(&self) -> &'static Schema {
        self.schema
    }
}

impl Clone for UntypedAtomicResource {
    fn clone(&self) -> Self {
        Self {
            cell: Arc::new(AtomicRefCell::new(self.cell.borrow().clone())),
            schema: self.schema,
        }
    }
}

impl UntypedResources {
    /// Create an empty [`UntypedResources`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a new resource.
    pub fn insert(&mut self, resource: UntypedAtomicResource) -> Option<UntypedAtomicResource> {
        let id = resource.cell.borrow().schema().id();
        self.resources.insert(id, resource)
    }

    /// Get a cell containing the resource data pointer for the given ID.
    pub fn get_cell(&self, schema_id: SchemaId) -> Option<UntypedAtomicResource> {
        self.resources.get(&schema_id).map(|x| x.clone_cell())
    }

    /// Get a reference to an untyped resource.
    pub fn get(&self, schema_id: SchemaId) -> Option<AtomicRef<SchemaBox>> {
        self.resources
            .get(&schema_id)
            .map(|x| AtomicRefCell::borrow(&x.cell))
    }

    /// Get a mutable reference to an untyped resource.
    pub fn get_mut(&mut self, schema_id: SchemaId) -> Option<AtomicRefMut<SchemaBox>> {
        self.resources
            .get(&schema_id)
            .map(|x| AtomicRefCell::borrow_mut(&x.cell))
    }

    /// Remove a resource.
    pub fn remove(&mut self, id: SchemaId) -> Option<UntypedAtomicResource> {
        self.resources.remove(&id)
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
        self.untyped.insert(UntypedAtomicResource::new(resource));
    }

    /// Borrow a resource.
    pub fn get<T: HasSchema>(&self) -> Option<AtomicRef<T>> {
        self.untyped.get(T::schema().id()).map(|sbox| {
            AtomicRef::map(sbox, |sbox| {
                // SOUND: schema matches as checked by retreiving from the untyped store by the schema
                // ID.
                unsafe { sbox.as_ref().deref() }
            })
        })
    }

    /// Mutably borrow a resource.
    pub fn get_mut<T: HasSchema>(&mut self) -> Option<AtomicRefMut<T>> {
        self.untyped.get_mut(T::schema().id()).map(|sbox| {
            AtomicRefMut::map(sbox, |sbox| {
                // SOUND: schema matches as checked by retreiving from the untyped store by the
                // schema ID.
                unsafe { sbox.as_mut().deref_mut() }
            })
        })
    }

    /// Check whether or not a resource is in the store.
    ///
    /// See [get()][Self::get]
    pub fn contains<T: HasSchema>(&self) -> bool {
        self.untyped.resources.contains_key(&T::schema().id())
    }

    /// Gets a clone of the resource cell for the resource of the given type.
    pub fn get_cell<T: HasSchema>(&self) -> Option<AtomicResource<T>> {
        let untyped = self.untyped.get_cell(T::schema().id())?;

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
    untyped: UntypedAtomicResource,
    _phantom: PhantomData<T>,
}

impl<T: HasSchema> AtomicResource<T> {
    /// Lock the resource for reading.
    ///
    /// This returns a read guard, very similar to an [`RwLock`][std::sync::RwLock].
    pub fn borrow(&self) -> AtomicRef<T> {
        let borrow = AtomicRefCell::borrow(&self.untyped);
        // SOUND: We know that the data pointer is valid for type T.
        AtomicRef::map(borrow, |data| unsafe { data.as_ref().deref() })
    }

    /// Lock the resource for read-writing.
    ///
    /// This returns a write guard, very similar to an [`RwLock`][std::sync::RwLock].
    pub fn borrow_mut(&self) -> AtomicRefMut<T> {
        let borrow = AtomicRefCell::borrow_mut(&self.untyped);
        // SOUND: We know that the data pointer is valid for type T.
        AtomicRefMut::map(borrow, |data| unsafe { data.as_mut().deref_mut() })
    }
}

#[cfg(test)]
mod test {
    use crate::prelude::*;

    #[test]
    fn sanity_check() {
        #[derive(HasSchema, Clone, Debug, Default)]
        #[repr(C)]
        struct A(String);

        #[derive(HasSchema, Clone, Debug, Default)]
        #[repr(C)]
        struct B(u32);

        let mut resources = Resources::new();

        resources.insert(A(String::from("hi")));
        assert_eq!(resources.get::<A>().unwrap().0, "hi");

        let r2 = resources.clone();

        resources.insert(A(String::from("bye")));
        resources.insert(A(String::from("world")));
        assert_eq!(resources.get::<A>().unwrap().0, "world");

        assert_eq!(r2.get::<A>().unwrap().0, "hi");

        resources.insert(B(1));
        assert_eq!(resources.get::<B>().unwrap().0, 1);
        resources.insert(B(2));
        assert_eq!(resources.get::<B>().unwrap().0, 2);
        assert_eq!(resources.get::<A>().unwrap().0, "world");
    }
}
