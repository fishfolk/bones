use std::{alloc::Layout, any::TypeId, marker::PhantomData, sync::OnceLock};

use bones_schema::{prelude::*, raw_fns::*};
use bones_utils::{parking_lot::RwLock, HashMap};
use ulid::Ulid;

/// A typed handle to an asset.
#[repr(C)]
pub struct Handle<T> {
    /// The runtime ID of the asset.
    pub id: Ulid,
    phantom: PhantomData<T>,
}

// Manually implement these traits we normally derive because the derive assumes that `T` must also
// implement these traits.
impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        *self
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

//
// Schema implementations
//

/// [Type data][TypeDatas] for asset handles.
///
/// This allows the asset loader to distinguish when a `SomeStruct(u128)` schema layout should be
/// deserialized as a normal struct or as an asset handle.
#[derive(HasSchema, Clone, Copy, Debug)]
#[schema(opaque, no_default)]
pub struct SchemaAssetHandle {
    /// The schema of the type pointed to by the handle, if this is not an [`UntypedHandle`].
    pub schema: Option<&'static Schema>,
}

// SAFE: We return a valid schema.
unsafe impl<T: HasSchema> HasSchema for Handle<T> {
    fn schema() -> &'static bones_schema::Schema {
        static S: OnceLock<RwLock<HashMap<TypeId, &'static Schema>>> = OnceLock::new();
        // This is a hack to make sure that `Ulid` has the memory representation we
        // expect. It is extremely unlike, but possible that this would otherwise be
        // unsound in the event that Rust picks a weird representation for the
        // `Ulid(u128)` struct, which doesn't have a `#[repr(C)]` or
        // `#[repr(transparent)]` annotation.
        assert_eq!(
            Layout::new::<Ulid>(),
            Layout::new::<u128>(),
            "ULID memory layout is unexpected! Bad Rust compiler! 😡"
        );

        let map = S.get_or_init(|| RwLock::new(HashMap::default()));

        let existing_schema = { map.read().get(&TypeId::of::<T>()).copied() };

        if let Some(existing_schema) = existing_schema {
            existing_schema
        } else {
            let schema = SCHEMA_REGISTRY.register(SchemaData {
                type_id: Some(TypeId::of::<Self>()),
                kind: SchemaKind::Struct(StructSchemaInfo {
                    fields: vec![StructFieldInfo {
                        name: Some("id".into()),
                        schema: u128::schema(),
                    }],
                }),
                clone_fn: Some(<Self as RawClone>::raw_clone),
                drop_fn: None,
                default_fn: Some(<Self as RawDefault>::raw_default),
                eq_fn: Some(<Self as RawEq>::raw_eq),
                hash_fn: Some(<Self as RawHash>::raw_hash),
                type_data: {
                    let mut td = bones_schema::alloc::SchemaTypeMap::default();
                    td.insert(SchemaAssetHandle {
                        schema: Some(T::schema()),
                    });
                    td
                },
            });

            {
                let mut map = map.write();
                map.insert(TypeId::of::<T>(), schema);
            }

            schema
        }
    }
}
// SAFE: We return a valid schema.
unsafe impl HasSchema for UntypedHandle {
    fn schema() -> &'static bones_schema::Schema {
        static S: OnceLock<&'static Schema> = OnceLock::new();
        // This is a hack to make sure that `Ulid` has the memory representation we
        // expect. It is extremely unlike, but possible that this would otherwise be
        // unsound in the event that Rust picks a weird representation for the
        // `Ulid(u128)` struct, which doesn't have a `#[repr(C)]` or
        // `#[repr(transparent)]` annotation.
        assert_eq!(
            Layout::new::<Ulid>(),
            Layout::new::<u128>(),
            "ULID memory layout is unexpected! Bad Rust compiler! 😡"
        );
        S.get_or_init(|| {
            SCHEMA_REGISTRY.register(SchemaData {
                type_id: Some(TypeId::of::<Self>()),
                kind: SchemaKind::Struct(StructSchemaInfo {
                    fields: vec![StructFieldInfo {
                        name: Some("id".into()),
                        schema: u128::schema(),
                    }],
                }),
                clone_fn: Some(<Self as RawClone>::raw_clone),
                drop_fn: None,
                default_fn: Some(<Self as RawDefault>::raw_default),
                eq_fn: Some(<Self as RawEq>::raw_eq),
                hash_fn: Some(<Self as RawHash>::raw_hash),
                type_data: {
                    let mut td = bones_schema::alloc::SchemaTypeMap::default();
                    td.insert(SchemaAssetHandle { schema: None });
                    td
                },
            })
        })
    }
}
