//! Persistant storage API.

use crate::prelude::*;

/// Persitent storage resource.
///
/// > **Note:** data is not actually saved until you call [`Storage::save`]
///
/// > **🚧 Warning:** The storage interface uses the types [`SchemaData::full_name`] as a storage
/// > key, so you must ensure that all types that are stored have different full names or it may
/// > behave unexpectedly.
#[derive(HasSchema)]
#[schema(no_clone)]
pub struct Storage {
    /// The backend storage API.
    pub backend: Box<dyn StorageApi>,
    /// The cache of objects that have been read
    pub cache: HashMap<SchemaId, SchemaBox>,
}
#[allow(clippy::derivable_impls)] // false positive
impl Default for Storage {
    fn default() -> Self {
        Self {
            backend: Box::<MemoryBackend>::default(),
            cache: Default::default(),
        }
    }
}

impl Storage {
    /// Create a new storage resource with the given backend storage API.
    pub fn with_backend(backend: Box<dyn StorageApi>) -> Self {
        Self {
            backend,
            cache: default(),
        }
    }

    /// Load the data from the storage backend.
    pub fn load(&mut self) {
        self.cache = self
            .backend
            .load()
            .into_iter()
            .map(|x| (x.schema().id(), x))
            .collect();
    }

    /// Save the data to the storage backend.
    pub fn save(&mut self) {
        self.backend.save(self.cache.values().cloned().collect())
    }

    /// Insert the data into storage cache.
    pub fn insert<T: HasSchema>(&mut self, data: T) {
        let b = SchemaBox::new(data);
        self.cache.insert(b.schema().id(), b);
    }

    /// Get data from the storage cache.
    pub fn get<T: HasSchema>(&self) -> Option<&T> {
        self.cache.get(&T::schema().id()).map(|x| x.cast_ref())
    }

    /// Get data mutably from the storage cache.
    pub fn get_mut<T: HasSchema>(&mut self) -> Option<&mut T> {
        self.cache.get_mut(&T::schema().id()).map(|x| x.cast_mut())
    }

    /// Get data from the storage cache or insert it's default value
    pub fn get_or_insert_default<T: HasSchema + Default>(&mut self) -> &T {
        self.cache
            .entry(T::schema().id())
            .or_insert_with(|| SchemaBox::default(T::schema()))
            .cast_ref()
    }

    /// Get data mutably from the storage cache or insert it's default value
    pub fn get_or_insert_default_mut<T: HasSchema + Default>(&mut self) -> &mut T {
        self.cache
            .entry(T::schema().id())
            .or_insert_with(|| SchemaBox::default(T::schema()))
            .cast_mut()
    }

    /// Remove data for a type from the storage.
    pub fn remove<T: HasSchema>(&mut self) {
        self.cache.remove(&T::schema().id());
    }
}

/// Trait implemented by storage backends.
///
/// TODO: Implement asynchronous storage API.
/// Currently all storage access is synchronous, which is not good for the user experience when a
/// write to storage could delay the rendering of the next frame. We should come up with a
/// nice-to-use API for asynchronously loading and storing data.
pub trait StorageApi: Sync + Send {
    /// Write the entire collection of objects to storage, replacing the previous storage data. If
    /// set, the `handler` will be called when the data has been written.
    fn save(&mut self, data: Vec<SchemaBox>);
    /// Read the entire collection of objects from storage with `handler` being called with the data
    /// once the load is complete.
    fn load(&mut self) -> Vec<SchemaBox>;
}

/// Non-persistent [`Storage`] backend.
#[derive(Default)]
pub struct MemoryBackend {
    data: Vec<SchemaBox>,
}

impl StorageApi for MemoryBackend {
    fn save(&mut self, data: Vec<SchemaBox>) {
        self.data = data;
    }

    fn load(&mut self) -> Vec<SchemaBox> {
        self.data.clone()
    }
}
