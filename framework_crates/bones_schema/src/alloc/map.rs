use std::{
    any::TypeId,
    fmt::Debug,
    hash::{BuildHasher, Hasher},
    marker::PhantomData,
    sync::OnceLock,
};

use bones_utils::{hashbrown::hash_map, HashMap};

use crate::{
    prelude::*,
    raw_fns::{RawClone, RawDefault, RawDrop},
};

/// Untyped schema-aware "HashMap".
#[derive(Clone, Debug)]
pub struct SchemaMap {
    map: HashMap<SchemaBox, SchemaBox>,
    key_schema: &'static Schema,
    value_schema: &'static Schema,
}

impl SchemaMap {
    /// Create a new map, with the given key and value schemas.
    pub fn new(key_schema: &'static Schema, value_schema: &'static Schema) -> Self {
        assert!(
            key_schema.hash_fn.is_some() && key_schema.eq_fn.is_some(),
            "Key schema must implement hash and eq"
        );
        Self {
            map: HashMap::default(),
            key_schema,
            value_schema,
        }
    }

    /// Get the schema for the map keys.
    pub fn key_schema(&self) -> &'static Schema {
        self.key_schema
    }

    /// Get the schema for the map values.
    pub fn value_schema(&self) -> &'static Schema {
        self.value_schema
    }

    /// Insert an item into the map.
    /// # Panics
    /// Panics if the key or value schemas do not match the map's.
    #[inline]
    #[track_caller]
    pub fn insert<K: HasSchema, V: HasSchema>(&mut self, key: K, value: V) -> Option<V> {
        self.try_insert(key, value).unwrap()
    }

    /// Insert an item into the map.
    /// # Errors
    /// Errors if the key or value schemas do not match the map's.
    pub fn try_insert<K: HasSchema, V: HasSchema>(
        &mut self,
        key: K,
        value: V,
    ) -> Result<Option<V>, SchemaMismatchError> {
        let key = SchemaBox::new(key);
        let value = SchemaBox::new(value);
        self.try_insert_box(key, value)
            // SOUND: try_insert_box won't succeed unless the schema's match, so we have already
            // checked that the value schema matches.
            .map(|value| value.map(|x| unsafe { x.into_inner_unchecked() }))
    }

    /// Insert an untyped item into the map.
    /// # Panics
    /// Panics if the key or value schemas do not match the map's.
    #[track_caller]
    #[inline]
    pub fn insert_box(&mut self, key: SchemaBox, value: SchemaBox) -> Option<SchemaBox> {
        self.try_insert_box(key, value).unwrap()
    }

    /// Insert an untyped item into the map.
    /// # Errors
    /// Errors if the key or value schemas do not match the map's.
    pub fn try_insert_box(
        &mut self,
        key: SchemaBox,
        value: SchemaBox,
    ) -> Result<Option<SchemaBox>, SchemaMismatchError> {
        if key.schema() != self.key_schema || value.schema() != self.value_schema {
            Err(SchemaMismatchError)
        } else {
            // SOUnD: we've checked that the schemas are matching.
            let previous_value = unsafe { self.insert_box_unchecked(key, value) };
            Ok(previous_value)
        }
    }

    /// # Safety
    /// Key schema and value schema must match the map's.
    pub unsafe fn insert_box_unchecked(
        &mut self,
        key: SchemaBox,
        mut value: SchemaBox,
    ) -> Option<SchemaBox> {
        let hash = {
            let mut hasher = self.map.hasher().build_hasher();
            hasher.write_u64(key.hash());
            hasher.finish()
        };
        let entry = self
            .map
            .raw_entry_mut()
            .from_hash(hash, |other| other == &key);
        // Return the previous value.
        match entry {
            hash_map::RawEntryMut::Occupied(mut occupied) => {
                std::mem::swap(occupied.get_mut(), &mut value);
                Some(value)
            }
            hash_map::RawEntryMut::Vacant(vacant) => {
                vacant.insert(key, value);
                None
            }
        }
    }

    /// Get a value out of the map for the given key.
    /// # Panics
    /// Panics if the schemas of the key and value don't match the schemas of the map.
    #[track_caller]
    #[inline]
    pub fn get<K: HasSchema, V: HasSchema>(&self, key: &K) -> Option<&V> {
        self.try_get(key).unwrap()
    }

    /// Get a value out of the map for the given key.
    /// # Errors
    /// Errors if the schemas of the key and value don't match the schemas of the map.
    #[track_caller]
    pub fn try_get<K: HasSchema, V: HasSchema>(
        &self,
        key: &K,
    ) -> Result<Option<&V>, SchemaMismatchError> {
        if K::schema() != self.key_schema || V::schema() != self.value_schema {
            Err(SchemaMismatchError)
        } else {
            // SOUND: we've veriried that they key schema matches the map's
            let value = unsafe { self.get_ref_unchecked(SchemaRef::new(key)) }
                // SOUND: we've verified that the value schema maches the map's
                .map(|x| unsafe { x.deref() });

            Ok(value)
        }
    }

    /// Get an untyped reference to an item in the map.
    /// # Panics
    /// Panics if the schema of the key doesn't match.
    #[inline]
    #[track_caller]
    pub fn get_ref(&self, key: SchemaRef) -> Option<SchemaRef> {
        self.try_get_ref(key).unwrap()
    }

    /// Get an untyped reference to an item in the map.
    /// # Errors
    /// Errors if the schema of the key doesn't match.
    pub fn try_get_ref(&self, key: SchemaRef) -> Result<Option<SchemaRef>, SchemaMismatchError> {
        if key.schema() != self.key_schema {
            Err(SchemaMismatchError)
        } else {
            // SOUND: we've check the key schema matches.
            Ok(unsafe { self.get_ref_unchecked(key) })
        }
    }

    /// # Safety
    /// The key's schema must match this map's key schema.
    pub unsafe fn get_ref_unchecked(&self, key: SchemaRef) -> Option<SchemaRef> {
        let Some(hash_fn) = self.key_schema.hash_fn else {
            panic!("Key schema doesn't implement hash");
        };
        let Some(eq_fn) = self.key_schema.eq_fn else {
            panic!("Key schema doesn't implement eq");
        };
        let key_ptr = key.as_ptr();
        // SOUND: caller asserts the key schema matches
        let raw_hash = unsafe { (hash_fn)(key_ptr) };
        let hash = {
            let mut hasher = self.map.hasher().build_hasher();
            hasher.write_u64(raw_hash);
            hasher.finish()
        };
        self.map
            .raw_entry()
            .from_hash(hash, |key| {
                let other_ptr = key.as_ref().as_ptr();
                // SOUND: caller asserts the key schema matches.
                unsafe { (eq_fn)(key_ptr, other_ptr) }
            })
            .map(|x| x.1.as_ref())
    }

    /// Get an untyped reference to an item in the map.
    /// # Panics
    /// Panics if the schema of the key doesn't match.
    #[inline]
    #[track_caller]
    pub fn get_ref_mut(&mut self, key: SchemaRef) -> Option<SchemaRefMut> {
        self.try_get_ref_mut(key).unwrap()
    }

    /// Get an untyped reference to an item in the map.
    /// # Errors
    /// Errors if the schema of the key doesn't match.
    pub fn try_get_ref_mut(
        &mut self,
        key: SchemaRef,
    ) -> Result<Option<SchemaRefMut>, SchemaMismatchError> {
        if key.schema() != self.key_schema {
            Err(SchemaMismatchError)
        } else {
            // SOUND: we've checked that the key schema matches.
            Ok(unsafe { self.get_ref_unchecked_mut(key) })
        }
    }

    /// # Safety
    /// The key's schema must match this map's key schema.
    pub unsafe fn get_ref_unchecked_mut(&mut self, key: SchemaRef) -> Option<SchemaRefMut> {
        let Some(hash_fn) = self.key_schema.hash_fn else {
            panic!("Key schema doesn't implement hash");
        };
        let Some(eq_fn) = self.key_schema.eq_fn else {
            panic!("Key schema doesn't implement eq");
        };
        let key_ptr = key.as_ptr();
        // SOUND: caller asserts the key schema matches
        let raw_hash = unsafe { (hash_fn)(key_ptr) };
        let hash = {
            let mut hasher = self.map.hasher().build_hasher();
            hasher.write_u64(raw_hash);
            hasher.finish()
        };
        let entry = self.map.raw_entry_mut().from_hash(hash, |key| {
            let other_ptr = key.as_ref().as_ptr();
            // SOUND: caller asserts the key schema matches.
            unsafe { (eq_fn)(key_ptr, other_ptr) }
        });
        match entry {
            hash_map::RawEntryMut::Occupied(entry) => Some(entry.into_mut()),
            hash_map::RawEntryMut::Vacant(_) => None,
        }
        .map(|x| x.as_mut())
    }

    /// Get a value out of the map for the given key.
    /// # Panics
    /// Panics if the schemas of the key and value don't match the schemas of the map.
    #[track_caller]
    #[inline]
    pub fn get_mut<K: HasSchema, V: HasSchema>(&mut self, key: &K) -> Option<&mut V> {
        self.try_get_mut(key).unwrap()
    }

    /// Get a value out of the map for the given key.
    /// # Errors
    /// Errors if the schemas of the key and value don't match the schemas of the map.
    #[track_caller]
    pub fn try_get_mut<K: HasSchema, V: HasSchema>(
        &mut self,
        key: &K,
    ) -> Result<Option<&mut V>, SchemaMismatchError> {
        if K::schema() != self.key_schema || V::schema() != self.value_schema {
            Err(SchemaMismatchError)
        } else {
            // SOUND: we've checked that the key schema matches.
            let value = unsafe { self.get_ref_unchecked_mut(SchemaRef::new(key)) }
                // SOUND: we've checked that the value schema matches.
                .map(|x| unsafe { x.deref_mut() });
            Ok(value)
        }
    }

    /// Remove an item.
    /// # Panics
    /// Panics if the schemas of the key and value don't match the map.
    #[inline]
    #[track_caller]
    pub fn remove<K: HasSchema, V: HasSchema>(&mut self, key: &K) -> Option<V> {
        self.try_remove(key).unwrap()
    }

    /// Remove an item.
    /// # Errors
    /// Errors if the schemas of the key and value don't match the map.
    pub fn try_remove<K: HasSchema, V: HasSchema>(
        &mut self,
        key: &K,
    ) -> Result<Option<V>, SchemaMismatchError> {
        if K::schema() != self.key_schema || V::schema() != self.value_schema {
            Err(SchemaMismatchError)
        } else {
            // SOUND: we've checked that the key schema matches.
            let value = unsafe { self.remove_unchecked(SchemaRef::new(key)) }
                // SOUND: we've checked that the value schema matches.
                .map(|x| unsafe { x.into_inner_unchecked() });
            Ok(value)
        }
    }

    /// Untypededly remove an item.
    /// # Panics
    /// Panics if the key schema does not match the map's.
    #[inline]
    #[track_caller]
    pub fn remove_box(&mut self, key: SchemaRef) -> Option<SchemaBox> {
        self.try_remove_box(key).unwrap()
    }

    /// Untypededly remove an item.
    /// # Errors
    /// Errors if the key schema does not match the map's.
    pub fn try_remove_box(
        &mut self,
        key: SchemaRef,
    ) -> Result<Option<SchemaBox>, SchemaMismatchError> {
        if key.schema() != self.key_schema {
            Err(SchemaMismatchError)
        } else {
            // SOUND: we've checked the key schema matches.
            Ok(unsafe { self.remove_unchecked(key) })
        }
    }

    /// # Safety
    /// The key schema must match the map's.
    pub unsafe fn remove_unchecked(&mut self, key: SchemaRef) -> Option<SchemaBox> {
        let Some(hash_fn) = self.key_schema.hash_fn else {
            panic!("Key schema doesn't implement hash");
        };
        let Some(eq_fn) = self.key_schema.eq_fn else {
            panic!("Key schema doesn't implement eq");
        };
        let key_ptr = key.as_ptr();
        // SOUND: caller asserts the key schema matches
        let hash = unsafe { (hash_fn)(key_ptr) };
        let hash = {
            let mut hasher = self.map.hasher().build_hasher();
            hasher.write_u64(hash);
            hasher.finish()
        };
        let entry = self.map.raw_entry_mut().from_hash(hash, |key| {
            let other_ptr = key.as_ref().as_ptr();
            // SOUND: caller asserts the key schema matches
            unsafe { (eq_fn)(key_ptr, other_ptr) }
        });
        match entry {
            hash_map::RawEntryMut::Occupied(entry) => Some(entry.remove()),
            hash_map::RawEntryMut::Vacant(_) => None,
        }
    }

    /// Convert into a typed [`SMap`].
    /// # Panics
    /// Panics if the schemas of K and V don't match the [`SchemaMap`].
    #[inline]
    #[track_caller]
    pub fn into_smap<K: HasSchema, V: HasSchema>(self) -> SMap<K, V> {
        self.try_into_smap().unwrap()
    }

    /// Convert into a typed [`SMap`].
    /// # Errors
    /// Errors if the schemas of K and V don't match the [`SchemaMap`].
    pub fn try_into_smap<K: HasSchema, V: HasSchema>(
        self,
    ) -> Result<SMap<K, V>, SchemaMismatchError> {
        if K::schema() == self.key_schema && V::schema() == self.value_schema {
            Ok(SMap {
                map: self,
                _phantom: PhantomData,
            })
        } else {
            Err(SchemaMismatchError)
        }
    }
}

type SchemaMapIter<'iter> = std::iter::Map<
    hash_map::Iter<'iter, SchemaBox, SchemaBox>,
    for<'a> fn((&'a SchemaBox, &'a SchemaBox)) -> (SchemaRef<'a>, SchemaRef<'a>),
>;
type SchemaMapIterMut<'iter> = std::iter::Map<
    hash_map::IterMut<'iter, SchemaBox, SchemaBox>,
    for<'a> fn((&'a SchemaBox, &'a mut SchemaBox)) -> (SchemaRef<'a>, SchemaRefMut<'a, 'a>),
>;
impl SchemaMap {
    /// Iterate over entries in the map.
    #[allow(clippy::type_complexity)]
    pub fn iter(&self) -> SchemaMapIter {
        fn map_fn<'a>(
            (key, value): (&'a SchemaBox, &'a SchemaBox),
        ) -> (SchemaRef<'a>, SchemaRef<'a>) {
            (key.as_ref(), value.as_ref())
        }
        self.map.iter().map(map_fn)
    }

    /// Iterate over entries in the map.
    #[allow(clippy::type_complexity)]
    pub fn iter_mut(&mut self) -> SchemaMapIterMut {
        fn map_fn<'a>(
            (key, value): (&'a SchemaBox, &'a mut SchemaBox),
        ) -> (SchemaRef<'a>, SchemaRefMut<'a, 'a>) {
            (key.as_ref(), value.as_mut())
        }
        self.map.iter_mut().map(map_fn)
    }

    /// Iterate over keys in the map.
    #[allow(clippy::type_complexity)]
    pub fn keys(
        &self,
    ) -> std::iter::Map<
        hash_map::Keys<SchemaBox, SchemaBox>,
        for<'a> fn(&'a SchemaBox) -> SchemaRef<'a>,
    > {
        fn map_fn(key: &SchemaBox) -> SchemaRef {
            key.as_ref()
        }
        self.map.keys().map(map_fn)
    }

    /// Iterate over values in the map.
    #[allow(clippy::type_complexity)]
    pub fn values(
        &self,
    ) -> std::iter::Map<
        hash_map::Values<SchemaBox, SchemaBox>,
        for<'a> fn(&'a SchemaBox) -> SchemaRef<'a>,
    > {
        fn map_fn(key: &SchemaBox) -> SchemaRef {
            key.as_ref()
        }
        self.map.values().map(map_fn)
    }

    /// Iterate over values in the map.
    #[allow(clippy::type_complexity)]
    pub fn values_mut(
        &mut self,
    ) -> std::iter::Map<
        hash_map::ValuesMut<SchemaBox, SchemaBox>,
        for<'a> fn(&'a mut SchemaBox) -> SchemaRefMut<'a, 'a>,
    > {
        fn map_fn(key: &mut SchemaBox) -> SchemaRefMut {
            key.as_mut()
        }
        self.map.values_mut().map(map_fn)
    }
}
impl<'a> IntoIterator for &'a SchemaMap {
    type Item = (SchemaRef<'a>, SchemaRef<'a>);
    type IntoIter = SchemaMapIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl<'a> IntoIterator for &'a mut SchemaMap {
    type Item = (SchemaRef<'a>, SchemaRefMut<'a, 'a>);
    type IntoIter = SchemaMapIterMut<'a>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

/// Typed version of a [`SchemaMap`].
///
/// This works essentially like a [`HashMap`], but is compatible with the schema ecosystem.
///
/// It is also slightly more efficient to access an [`SMap`] compared to a [`SchemaMap`] because it
/// doesn't need to do a runtime schema check every time the map is accessed.
pub struct SMap<K: HasSchema, V: HasSchema> {
    map: SchemaMap,
    _phantom: PhantomData<(K, V)>,
}
impl<K: HasSchema + Debug, V: HasSchema + Debug> Debug for SMap<K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut f = f.debug_map();
        for (k, v) in self {
            f.entry(k, v);
        }
        f.finish()
    }
}
impl<K: HasSchema, V: HasSchema> Clone for SMap<K, V> {
    fn clone(&self) -> Self {
        Self {
            map: self.map.clone(),
            _phantom: self._phantom,
        }
    }
}
impl<K: HasSchema, V: HasSchema> Default for SMap<K, V> {
    fn default() -> Self {
        Self {
            map: SchemaMap::new(K::schema(), V::schema()),
            _phantom: Default::default(),
        }
    }
}
unsafe impl<K: HasSchema, V: HasSchema> HasSchema for SMap<K, V> {
    fn schema() -> &'static Schema {
        static S: OnceLock<&'static Schema> = OnceLock::new();
        S.get_or_init(|| {
            SCHEMA_REGISTRY.register(SchemaData {
                kind: SchemaKind::Map {
                    key: K::schema(),
                    value: V::schema(),
                },
                type_id: Some(TypeId::of::<Self>()),
                clone_fn: Some(<Self as RawClone>::raw_clone),
                drop_fn: Some(<Self as RawDrop>::raw_drop),
                default_fn: Some(<Self as RawDefault>::raw_default),
                hash_fn: Some(SchemaVec::raw_hash),
                eq_fn: Some(SchemaVec::raw_eq),
                type_data: Default::default(),
            })
        })
    }
}

impl<K: HasSchema, V: HasSchema> SMap<K, V> {
    /// Initialize the [`SMap`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert an entry into the map, returning the previous value, if it exists.
    pub fn insert(&mut self, k: K, v: V) -> Option<V> {
        // SOUND: schemas are checked to match when SMap is constructed.
        unsafe {
            self.map
                .insert_box_unchecked(SchemaBox::new(k), SchemaBox::new(v))
                .map(|x| x.into_inner_unchecked())
        }
    }

    /// Get a reference to an item in the map.
    pub fn get(&self, key: &K) -> Option<&V> {
        // SOUND: schemas are checked to match when SMap is constructed.
        unsafe {
            self.map
                .get_ref_unchecked(SchemaRef::new(key))
                .map(|x| x.deref())
        }
    }

    /// Get a mutable reference to an item in the map.
    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        // SOUND: schemas are checked to match when SMap is constructed.
        unsafe {
            self.map
                .get_ref_unchecked_mut(SchemaRef::new(key))
                .map(|x| x.deref_mut())
        }
    }

    /// Remove an item from the map.
    pub fn remove(&mut self, key: &K) -> Option<V> {
        // SOUND: schemas are checked to match when SMap is constructed.
        unsafe {
            self.map
                .remove_unchecked(SchemaRef::new(key))
                .map(|x| x.into_inner_unchecked())
        }
    }

    /// Convert into an untyped [`SchemaMap`].
    pub fn into_schema_map(self) -> SchemaMap {
        self.map
    }
}

type SMapIter<'iter, K, V> = std::iter::Map<
    hash_map::Iter<'iter, SchemaBox, SchemaBox>,
    for<'a> fn((&'a SchemaBox, &'a SchemaBox)) -> (&'a K, &'a V),
>;
type SMapIterMut<'iter, K, V> = std::iter::Map<
    hash_map::IterMut<'iter, SchemaBox, SchemaBox>,
    for<'a> fn((&'a SchemaBox, &'a mut SchemaBox)) -> (&'a K, &'a mut V),
>;
impl<K: HasSchema, V: HasSchema> SMap<K, V> {
    /// Iterate over entries in the map.
    #[allow(clippy::type_complexity)]
    pub fn iter(&self) -> SMapIter<K, V> {
        fn map_fn<'a, K, V>((key, value): (&'a SchemaBox, &'a SchemaBox)) -> (&K, &V) {
            // SOUND: SMap ensures K and V schemas always match.
            unsafe { (key.as_ref().deref(), value.as_ref().deref()) }
        }
        self.map.map.iter().map(map_fn)
    }

    /// Iterate over entries in the map.
    #[allow(clippy::type_complexity)]
    pub fn iter_mut(&mut self) -> SMapIterMut<K, V> {
        fn map_fn<'a, K, V>(
            (key, value): (&'a SchemaBox, &'a mut SchemaBox),
        ) -> (&'a K, &'a mut V) {
            // SOUND: SMap ensures K and V schemas always match.
            unsafe { (key.as_ref().deref(), value.as_mut().deref_mut()) }
        }
        self.map.map.iter_mut().map(map_fn)
    }

    /// Iterate over keys in the map.
    #[allow(clippy::type_complexity)]
    pub fn keys(
        &self,
    ) -> std::iter::Map<hash_map::Keys<SchemaBox, SchemaBox>, for<'a> fn(&'a SchemaBox) -> &'a K>
    {
        fn map_fn<K>(key: &SchemaBox) -> &K {
            // SOUND: SMap ensures key schema always match
            unsafe { key.as_ref().deref() }
        }
        self.map.map.keys().map(map_fn)
    }

    /// Iterate over values in the map.
    #[allow(clippy::type_complexity)]
    pub fn values(
        &self,
    ) -> std::iter::Map<hash_map::Values<SchemaBox, SchemaBox>, for<'a> fn(&'a SchemaBox) -> &V>
    {
        fn map_fn<V>(value: &SchemaBox) -> &V {
            // SOUND: SMap ensures value schema always matches.
            unsafe { value.as_ref().deref() }
        }
        self.map.map.values().map(map_fn)
    }

    /// Iterate over values in the map.
    #[allow(clippy::type_complexity)]
    pub fn values_mut(
        &mut self,
    ) -> std::iter::Map<
        hash_map::ValuesMut<SchemaBox, SchemaBox>,
        for<'a> fn(&'a mut SchemaBox) -> &mut V,
    > {
        fn map_fn<V>(value: &mut SchemaBox) -> &mut V {
            // SOUND: SMap ensures value schema always matches
            unsafe { value.as_mut().deref_mut() }
        }
        self.map.map.values_mut().map(map_fn)
    }
}
impl<'a, K: HasSchema, V: HasSchema> IntoIterator for &'a SMap<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = SMapIter<'a, K, V>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl<'a, K: HasSchema, V: HasSchema> IntoIterator for &'a mut SMap<K, V> {
    type Item = (&'a K, &'a mut V);
    type IntoIter = SMapIterMut<'a, K, V>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}
