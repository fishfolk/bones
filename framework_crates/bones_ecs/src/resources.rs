//! World resource storage.

use std::{fmt::Debug, marker::PhantomData, sync::Arc};

use once_map::OnceMap;

use crate::prelude::*;

/// An untyped, atomic resource cell.
pub type AtomicUntypedResource = Arc<UntypedResource>;

/// An untyped resource that may be inserted into [`UntypedResources`].
///
/// This is fundamentally a [`Arc<AtomicCell<Option<SchemaBox>>>`] and thus represents
/// a cell that may or may not contain a resource of it's schema.
pub struct UntypedResource {
    cell: AtomicCell<Option<SchemaBox>>,
    schema: &'static Schema,
}

impl std::fmt::Debug for UntypedResource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UntypedResource").finish_non_exhaustive()
    }
}

impl DesyncHash for UntypedResource {
    fn hash(&self, hasher: &mut dyn std::hash::Hasher) {
        if let Some(schema_box) = self.cell.borrow().as_ref() {
            DesyncHash::hash(&schema_box.as_ref(), hasher);
        }
    }
}

impl UntypedResource {
    /// Initialize a new, empty [`UntypedResource`].
    pub fn empty(schema: &'static Schema) -> Self {
        Self {
            cell: AtomicCell::new(None),
            schema,
        }
    }

    /// Creates a new [`UntypedResource`] storing the given data.
    pub fn new(resource: SchemaBox) -> Self {
        Self {
            schema: resource.schema(),
            cell: AtomicCell::new(Some(resource)),
        }
    }

    /// Create a new [`UntypedResource`] for the given schema, initially populated with the default
    /// value for the schema.
    pub fn from_default(schema: &'static Schema) -> Self {
        Self {
            cell: AtomicCell::new(Some(SchemaBox::default(schema))),
            schema,
        }
    }

    /// Clone the inner data, creating a new copy instead of returning another handle the the same
    /// data, as the normal `clone()` implementation does.
    pub fn clone_data(&self) -> Option<SchemaBox> {
        (*self.cell.borrow()).clone()
    }

    /// Insert resource data into the cell, returning the previous data.
    /// # Errors
    /// Errors if the schema of the data does not match that of this cell.
    pub fn insert(&self, data: SchemaBox) -> Result<Option<SchemaBox>, SchemaMismatchError> {
        self.schema.ensure_match(data.schema())?;
        let mut data = Some(data);
        std::mem::swap(&mut data, &mut *self.cell.borrow_mut());
        Ok(data)
    }

    /// Remove the resource data, returning what was stored in it.
    pub fn remove(&self) -> Option<SchemaBox> {
        let mut data = None;
        std::mem::swap(&mut data, &mut *self.cell.borrow_mut());
        data
    }

    /// Borrow the resource.
    #[track_caller]
    pub fn borrow(&self) -> Ref<Option<SchemaBox>> {
        self.cell.borrow()
    }

    /// Mutably borrow the resource.
    #[track_caller]
    pub fn borrow_mut(&self) -> RefMut<Option<SchemaBox>> {
        self.cell.borrow_mut()
    }

    /// Get the schema of the resource.
    pub fn schema(&self) -> &'static Schema {
        self.schema
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
    resources: OnceMap<SchemaId, AtomicUntypedResource>,
    shared_resources: OnceMap<SchemaId, Box<()>>,
}

impl Clone for UntypedResources {
    fn clone(&self) -> Self {
        let binding = self.resources.read_only_view();
        let resources = binding.iter().map(|(_, v)| (v.schema, v));

        let new_resources = OnceMap::default();
        let new_shared_resources = OnceMap::default();
        for (schema, resource_cell) in resources {
            let is_shared = self.shared_resources.contains_key(&schema.id());

            if !is_shared {
                let resource = resource_cell.clone_data();
                new_resources.map_insert(
                    schema.id(),
                    |_| Arc::new(UntypedResource::empty(schema)),
                    |_, cell| {
                        if let Some(resource) = resource {
                            cell.insert(resource).unwrap();
                        }
                    },
                );
            } else {
                new_shared_resources.insert(schema.id(), |_| Box::new(()));
                new_resources.insert(schema.id(), |_| resource_cell.clone());
            }
        }
        Self {
            resources: new_resources,
            shared_resources: new_shared_resources,
        }
    }
}

impl DesyncHash for UntypedResources {
    fn hash(&self, hasher: &mut dyn std::hash::Hasher) {
        for (schema_id, resource_cell) in self.resources.read_only_view().iter() {
            let is_shared = self.shared_resources.contains_key(schema_id);

            if !is_shared {
                // Verify Schema for resource implement desync hash. If no hash fn,
                // we should avoid hashing the schema too.
                let schema = resource_cell.schema();
                if schema.type_data.get::<SchemaDesyncHash>().is_some() {
                    schema.full_name.hash(hasher);
                    resource_cell.hash(hasher);
                }
            }
        }
    }
}

/// Error thrown when a resource cell cannot be inserted because it already exists.
#[derive(Debug, Clone, Copy)]
pub struct CellAlreadyPresentError;
impl std::fmt::Display for CellAlreadyPresentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Resource cell already present")
    }
}
impl std::error::Error for CellAlreadyPresentError {}

impl UntypedResources {
    /// Create an empty [`UntypedResources`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Check whether or not a cell for the given resource has been initialized yet.
    pub fn contains_cell(&self, id: SchemaId) -> bool {
        self.resources.contains_key(&id)
    }

    /// Check whether or not the resource with the given ID is present.
    pub fn contains(&self, id: SchemaId) -> bool {
        self.resources
            .map_get(&id, |_, cell| cell.borrow().is_some())
            .unwrap_or_default()
    }

    /// This is an advanced use-case function that allows you to insert a resource cell directly.
    ///
    /// Normally this is completely unnecessary, because cells are automatically inserted lazily as
    /// requested.
    ///
    /// Inserting this manually is used internally for shared resources, by inserting the same
    /// cell into multiple worlds.
    ///
    /// # Errors
    /// This will error if there is already a cell for the resource present. You cannot add a new
    /// cell once one has already been inserted.
    pub fn insert_cell(&self, cell: AtomicUntypedResource) -> Result<(), CellAlreadyPresentError> {
        let schema = cell.schema;
        if self.resources.contains_key(&schema.id()) {
            Err(CellAlreadyPresentError)
        } else {
            self.resources.insert(schema.id(), |_| cell);
            self.shared_resources.insert(schema.id(), |_| Box::new(()));
            Ok(())
        }
    }

    /// Borrow the resource for the given schema.
    pub fn get(&self, schema: &'static Schema) -> &UntypedResource {
        self.resources
            .insert(schema.id(), |_| Arc::new(UntypedResource::empty(schema)))
    }

    /// Get a cell for the resource with the given schema.
    pub fn get_cell(&self, schema: &'static Schema) -> AtomicUntypedResource {
        self.resources.map_insert(
            schema.id(),
            |_| Arc::new(UntypedResource::empty(schema)),
            |_, cell| cell.clone(),
        )
    }
}

/// A collection of resources.
///
/// [`Resources`] is essentially a type-map
#[derive(Clone, Default)]
pub struct Resources {
    untyped: UntypedResources,
}

impl DesyncHash for Resources {
    fn hash(&self, hasher: &mut dyn std::hash::Hasher) {
        self.untyped.hash(hasher);
    }
}

impl Resources {
    /// Create an empty [`Resources`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a resource.
    pub fn insert<T: HasSchema>(&self, resource: T) -> Option<T> {
        self.untyped
            .get(T::schema())
            .insert(SchemaBox::new(resource))
            .unwrap()
            .map(|x| x.cast_into())
    }

    /// Check whether or not a resource is in the store.
    ///
    /// See [get()][Self::get]
    pub fn contains<T: HasSchema>(&self) -> bool {
        self.untyped.resources.contains_key(&T::schema().id())
    }

    /// Remove a resource from the store, if it is present.
    pub fn remove<T: HasSchema>(&self) -> Option<T> {
        self.untyped
            .get(T::schema())
            .remove()
            // SOUND: we know the type matches because we retrieve it by it's schema.
            .map(|x| unsafe { x.cast_into_unchecked() })
    }

    /// Borrow a resource.
    #[track_caller]
    pub fn get<T: HasSchema>(&self) -> Option<Ref<T>> {
        let b = self.untyped.get(T::schema()).borrow();
        if b.is_some() {
            Some(Ref::map(b, |b| unsafe {
                b.as_ref().unwrap().as_ref().cast_into_unchecked()
            }))
        } else {
            None
        }
    }

    /// Borrow a resource.
    #[track_caller]
    pub fn get_mut<T: HasSchema>(&self) -> Option<RefMut<T>> {
        let b = self.untyped.get(T::schema()).borrow_mut();
        if b.is_some() {
            Some(RefMut::map(b, |b| unsafe {
                b.as_mut().unwrap().as_mut().cast_into_mut_unchecked()
            }))
        } else {
            None
        }
    }

    /// Gets a clone of the resource cell for the resource of the given type.
    pub fn get_cell<T: HasSchema>(&self) -> AtomicResource<T> {
        let untyped = self.untyped.get_cell(T::schema()).clone();
        AtomicResource {
            untyped,
            _phantom: PhantomData,
        }
    }

    /// Borrow the underlying [`UntypedResources`] store.
    pub fn untyped(&self) -> &UntypedResources {
        &self.untyped
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
    untyped: AtomicUntypedResource,
    _phantom: PhantomData<T>,
}
impl<T: HasSchema + Debug> Debug for AtomicResource<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AtomicResource(")?;
        self.untyped
            .cell
            .borrow()
            .as_ref()
            .map(|x| x.cast_ref::<T>())
            .fmt(f)?;
        f.write_str(")")?;
        Ok(())
    }
}

impl<T: HasSchema + Default> Default for AtomicResource<T> {
    fn default() -> Self {
        Self {
            untyped: Arc::new(UntypedResource::new(SchemaBox::new(T::default()))),
            _phantom: Default::default(),
        }
    }
}

impl<T: HasSchema> AtomicResource<T> {
    /// Create a new, empty resource cell.
    pub fn empty() -> Self {
        Self {
            untyped: Arc::new(UntypedResource::empty(T::schema())),
            _phantom: PhantomData,
        }
    }

    /// Create a new resource cell with the given data.
    pub fn new(data: T) -> Self {
        AtomicResource {
            untyped: Arc::new(UntypedResource::new(SchemaBox::new(data))),
            _phantom: PhantomData,
        }
    }

    /// Create from an [`UntypedResource`].
    pub fn from_untyped(untyped: AtomicUntypedResource) -> Result<Self, SchemaMismatchError> {
        T::schema().ensure_match(untyped.schema)?;
        Ok(AtomicResource {
            untyped,
            _phantom: PhantomData,
        })
    }

    /// Remove the resource from the cell, leaving the cell empty.
    pub fn remove(&self) -> Option<T> {
        self.untyped
            .remove()
            // SOUND: The untyped data of an atomic resource must always be `T`.
            .map(|x| unsafe { x.cast_into_unchecked() })
    }

    /// Lock the resource for reading.
    ///
    /// This returns a read guard, very similar to an [`RwLock`][std::sync::RwLock].
    pub fn borrow(&self) -> Option<Ref<T>> {
        let borrow = self.untyped.borrow();
        if borrow.is_some() {
            Some(Ref::map(borrow, |r| unsafe {
                r.as_ref().unwrap().as_ref().cast_into_unchecked()
            }))
        } else {
            None
        }
    }

    /// Lock the resource for read-writing.
    ///
    /// This returns a write guard, very similar to an [`RwLock`][std::sync::RwLock].
    pub fn borrow_mut(&self) -> Option<RefMut<T>> {
        let borrow = self.untyped.borrow_mut();
        if borrow.is_some() {
            Some(RefMut::map(borrow, |r| unsafe {
                r.as_mut().unwrap().as_mut().cast_into_mut_unchecked()
            }))
        } else {
            None
        }
    }

    /// Convert into an untyped resource.
    pub fn into_untyped(self) -> AtomicUntypedResource {
        self.untyped
    }
}

impl<T: HasSchema + FromWorld> AtomicResource<T> {
    /// Initialize the resource using it's [`FromWorld`] implementation, if it is not present.
    pub fn init(&self, world: &World) {
        let mut borrow = self.untyped.borrow_mut();
        if unlikely(borrow.is_none()) {
            *borrow = Some(SchemaBox::new(T::from_world(world)))
        }
    }

    /// Borrow the resource, initializing it if it doesn't exist.
    #[track_caller]
    pub fn init_borrow(&self, world: &World) -> Ref<T> {
        let map_borrow = |borrow| {
            // SOUND: we know the schema matches.
            Ref::map(borrow, |b: &Option<SchemaBox>| unsafe {
                b.as_ref().unwrap().as_ref().cast_into_unchecked()
            })
        };
        let borrow = self.untyped.borrow();
        if unlikely(borrow.is_none()) {
            drop(borrow);
            {
                let mut borrow_mut = self.untyped.borrow_mut();
                *borrow_mut = Some(SchemaBox::new(T::from_world(world)));
            }

            map_borrow(self.untyped.borrow())
        } else {
            map_borrow(borrow)
        }
    }

    /// Borrow the resource, initializing it if it doesn't exist.
    #[track_caller]
    pub fn init_borrow_mut(&self, world: &World) -> RefMut<T> {
        let mut borrow = self.untyped.borrow_mut();
        if unlikely(borrow.is_none()) {
            *borrow = Some(SchemaBox::new(T::from_world(world)));
        }
        RefMut::map(borrow, |b| unsafe {
            b.as_mut().unwrap().as_mut().cast_into_mut_unchecked()
        })
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

        let r1 = Resources::new();

        r1.insert(A(String::from("hi")));
        let r1a = r1.get_cell::<A>();
        assert_eq!(r1a.borrow().unwrap().0, "hi");

        let r2 = r1.clone();

        r1.insert(A(String::from("bye")));
        r1.insert(A(String::from("world")));
        assert_eq!(r1a.borrow().unwrap().0, "world");

        let r2a = r2.get_cell::<A>();
        assert_eq!(r2a.borrow().unwrap().0, "hi");

        r1.insert(B(1));
        let r1b = r1.get_cell::<B>();
        assert_eq!(r1b.borrow().unwrap().0, 1);
        r1.insert(B(2));
        assert_eq!(r1b.borrow().unwrap().0, 2);
        assert_eq!(r1a.borrow().unwrap().0, "world");
    }
}
