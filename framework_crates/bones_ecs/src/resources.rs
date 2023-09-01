//! World resource storage.

use std::{fmt::Debug, marker::PhantomData, sync::Arc};

use atomicell::borrow::{AtomicBorrow, AtomicBorrowMut};

use crate::prelude::*;

/// An untyped resource that may be inserted into [`UntypedResources`].
#[derive(Clone)]
pub struct UntypedAtomicResource {
    cell: Arc<AtomicCell<SchemaBox>>,
    schema: &'static Schema,
}

impl UntypedAtomicResource {
    /// Creates a new [`UntypedAtomicResource`] storing the given data.
    pub fn new(resource: SchemaBox) -> Self {
        Self {
            schema: resource.schema(),
            cell: Arc::new(AtomicCell::new(resource)),
        }
    }

    /// Create a new [`UntypedAtomicResource`] for the given schema, initially populated with the default
    /// value for the schema.
    pub fn from_schema(schema: &'static Schema) -> Self {
        Self {
            cell: Arc::new(AtomicCell::new(SchemaBox::default(schema))),
            schema,
        }
    }

    /// Clone the inner data, creating a new copy instead of returning another handle the the same
    /// data, as the normal `clone()` implementation does.
    pub fn clone_data(&self) -> Self {
        Self {
            cell: Arc::new(AtomicCell::new((*self.cell.borrow()).clone())),
            schema: self.schema,
        }
    }

    /// Borrow the resource.
    pub fn borrow(&self) -> AtomicSchemaRef {
        // SOUND: we keep the borrow along with the reference.
        let (reference, borrow) = unsafe { Ref::into_split(self.cell.borrow()) };
        let schema_ref = reference.as_ref();
        AtomicSchemaRef { schema_ref, borrow }
    }

    /// Mutably borrow the resource.
    pub fn borrow_mut(&self) -> AtomicSchemaRefMut {
        // SOUND: we keep the borrow along with the reference.
        let (reference, borrow) = unsafe { RefMut::into_split(self.cell.borrow_mut()) };
        let schema_ref = reference.as_mut();
        AtomicSchemaRefMut { schema_ref, borrow }
    }

    /// Get the schema of the resource.
    pub fn schema(&self) -> &'static Schema {
        self.schema
    }
}

/// An atomic borrow of a [`SchemaRef`].
#[derive(Deref)]
pub struct AtomicSchemaRef<'a> {
    #[deref]
    schema_ref: SchemaRef<'a>,
    borrow: AtomicBorrow<'a>,
}

impl<'a> AtomicSchemaRef<'a> {
    /// # Safety
    /// You must know that T represents the data in the [`SchemaRef`].
    pub unsafe fn deref<T>(self) -> Ref<'a, T> {
        Ref::with_borrow(self.schema_ref.deref(), self.borrow)
    }

    /// Convert into typed [`Ref`]. This panics if the schema doesn't match.
    #[track_caller]
    pub fn typed<T: HasSchema>(self) -> Ref<'a, T> {
        assert_eq!(T::schema(), self.schema(), "Schema mismatch");
        // SOUND: we've checked for matching schema.
        unsafe { self.deref() }
    }
}

/// An atomic borrow of a [`SchemaRefMut`].
#[derive(Deref, DerefMut)]
pub struct AtomicSchemaRefMut<'a> {
    #[deref]
    schema_ref: SchemaRefMut<'a, 'a>,
    borrow: AtomicBorrowMut<'a>,
}

impl<'a> AtomicSchemaRefMut<'a> {
    /// # Safety
    /// You must know that T represents the data in the [`SchemaRefMut`].
    pub unsafe fn deref_mut<T>(self) -> RefMut<'a, T> {
        RefMut::with_borrow(self.schema_ref.deref_mut(), self.borrow)
    }

    /// Convert into typed [`RefMut`]. This panics if the schema doesn't match.
    #[track_caller]
    pub fn typed<T: HasSchema>(self) -> RefMut<'a, T> {
        assert_eq!(T::schema(), self.schema(), "Schema mismatch");
        // SOUND: we've checked for matching schema.
        unsafe { self.deref_mut() }
    }
}

/// Storage for un-typed resources.
///
/// This is the backing data store used by [`Resources`].
///
/// Unless you are intending to do modding or otherwise need raw pointers to your resource data, you
/// should use [`Resources`] instead.
#[derive(Default)]
pub struct UntypedResources {
    resources: HashMap<SchemaId, UntypedAtomicResource>,
}

impl Clone for UntypedResources {
    fn clone(&self) -> Self {
        let resources = self
            .resources
            .iter()
            .map(|(k, v)| (*k, v.clone_data()))
            .collect();
        Self { resources }
    }
}

impl UntypedResources {
    /// Create an empty [`UntypedResources`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a resource.
    pub fn insert(&mut self, resource: SchemaBox) -> Option<UntypedAtomicResource> {
        let id = resource.schema().id();
        self.resources
            .insert(id, UntypedAtomicResource::new(resource))
    }

    /// Check whether or not the resoruce with the given ID is present.
    pub fn contains(&self, id: SchemaId) -> bool {
        self.resources.contains_key(&id)
    }

    /// Insert a resource.
    pub fn insert_cell(
        &mut self,
        resource: UntypedAtomicResource,
    ) -> Option<UntypedAtomicResource> {
        let id = resource.schema().id();
        self.resources.insert(id, resource)
    }

    /// Get a cell containing the resource data pointer for the given ID.
    pub fn get_cell(&self, schema_id: SchemaId) -> Option<UntypedAtomicResource> {
        self.resources.get(&schema_id).cloned()
    }

    /// Get a reference to an untyped resource.
    pub fn get(&self, schema_id: SchemaId) -> Option<AtomicSchemaRef> {
        self.resources.get(&schema_id).map(|x| x.borrow())
    }

    /// Get a mutable reference to an untyped resource.
    pub fn get_mut(&mut self, schema_id: SchemaId) -> Option<AtomicSchemaRefMut> {
        self.resources.get(&schema_id).map(|x| x.borrow_mut())
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
    pub fn insert<T: HasSchema>(&mut self, resource: T) -> Option<AtomicResource<T>> {
        self.untyped
            .insert(SchemaBox::new(resource))
            .map(|x| AtomicResource::from_untyped(x))
    }

    /// Insert a resource cell.
    pub fn insert_cell<T: HasSchema>(&mut self, resource: AtomicResource<T>) {
        self.untyped.insert_cell(resource.untyped);
    }

    /// Borrow a resource.
    pub fn get<T: HasSchema>(&self) -> Option<Ref<T>> {
        self.untyped.get(T::schema().id()).map(|x| {
            // SOUND: untyped resources returns data matching the schema of T.
            unsafe { x.deref() }
        })
    }

    /// Mutably borrow a resource.
    pub fn get_mut<T: HasSchema>(&mut self) -> Option<RefMut<T>> {
        self.untyped.get_mut(T::schema().id()).map(|x| {
            // SOUND: untyped resources returns data matching the schema of T.
            unsafe { x.deref_mut() }
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
#[derive(Clone)]
pub struct AtomicResource<T: HasSchema> {
    untyped: UntypedAtomicResource,
    _phantom: PhantomData<T>,
}
impl<T: HasSchema + Debug> Debug for AtomicResource<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AtomicResource(")?;
        T::fmt(self.untyped.cell.borrow().cast_ref(), f)?;
        f.write_str(")")?;
        Ok(())
    }
}

impl<T: HasSchema + Default> Default for AtomicResource<T> {
    fn default() -> Self {
        Self {
            untyped: UntypedAtomicResource::new(SchemaBox::new(T::default())),
            _phantom: Default::default(),
        }
    }
}

impl<T: HasSchema> AtomicResource<T> {
    /// Create a new atomic resource.
    ///
    /// This can be inserted into a world by calling `world.resources.insert_cell`.
    pub fn new(data: T) -> Self {
        AtomicResource {
            untyped: UntypedAtomicResource::new(SchemaBox::new(data)),
            _phantom: PhantomData,
        }
    }

    /// Create from an [`UntypedAtomicResource`].
    pub fn from_untyped(untyped: UntypedAtomicResource) -> Self {
        assert_eq!(T::schema(), untyped.schema);
        AtomicResource {
            untyped,
            _phantom: PhantomData,
        }
    }

    /// Lock the resource for reading.
    ///
    /// This returns a read guard, very similar to an [`RwLock`][std::sync::RwLock].
    pub fn borrow(&self) -> Ref<T> {
        let borrow = self.untyped.borrow();
        // SOUND: We know that the data pointer is valid for type T.
        unsafe { borrow.deref() }
    }

    /// Lock the resource for read-writing.
    ///
    /// This returns a write guard, very similar to an [`RwLock`][std::sync::RwLock].
    pub fn borrow_mut(&self) -> RefMut<T> {
        let borrow = self.untyped.borrow_mut();
        // SOUND: We know that the data pointer is valid for type T.
        unsafe { borrow.deref_mut() }
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

    #[test]
    fn untyped_resources_no_rewrite() {
        let mut store = UntypedResources::default();

        // Create two datas with different types.
        let data1 = SchemaBox::new(0usize);
        let data1_schema = data1.schema();
        let mut data2 = SchemaBox::new(String::from("bad_data"));

        {
            // Try to "cheat" and overwrite the the data1 resource with data of a different type.
            store.insert(data1);
            let data1_cell = store.get_cell(data1_schema.id()).unwrap();
            let mut data1_borrow = data1_cell.borrow_mut();
            // This can't actually write to data1, it just changes the reference we have.
            *data1_borrow = data2.as_mut();
        }

        // Validate that data1 is unchanged
        let data1_cell = store.get_cell(data1_schema.id()).unwrap();
        let data1_borrow = data1_cell.borrow();
        assert_eq!(data1_borrow.cast::<usize>(), &0);
    }
}
