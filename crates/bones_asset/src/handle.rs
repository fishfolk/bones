use std::{alloc::Layout, any::TypeId, marker::PhantomData, sync::OnceLock};

use bones_schema::prelude::*;
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

//
// Schema implementations
//

/// [Type data][TypeDatas] for asset handles.
///
/// This allows the asset loader to distinguish when a `SomeStruct(u128)` schema layout should be
/// deserialized as a normal struct or as an asset handle.
///
/// It doesn't need to contain any data, because it's very presense in a schema's [`TypeDatas`]
/// indicates that the schema represents a handle.
#[derive(HasSchema, Clone, Copy, Default, Debug)]
#[schema(opaque)]
pub struct SchemaAssetHandle;

/// Helper to avoid typing the duplicate implementations of [`HasSchema`] for typed and untyped
/// handles.
macro_rules! schema_impl_for_handle {
    () => {
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
                    kind: SchemaKind::Struct(StructSchema {
                        fields: vec![StructField {
                            name: Some("id".into()),
                            schema: SCHEMA_REGISTRY.register(SchemaData {
                                type_id: Some(TypeId::of::<Ulid>()),
                                kind: SchemaKind::Primitive(Primitive::U128),
                                type_data: Default::default(),
                                clone_fn: Some(<u128 as RawClone>::raw_clone),
                                drop_fn: None,
                                default_fn: Some(<u128 as RawDefault>::raw_default),
                            }),
                        }],
                    }),
                    clone_fn: Some(<Self as RawClone>::raw_clone),
                    drop_fn: None,
                    default_fn: Some(<Self as RawDefault>::raw_default),
                    type_data: {
                        let mut td = TypeDatas::default();
                        td.insert(SchemaAssetHandle);
                        td
                    },
                })
            })
        }
    };
}

// SAFE: We return a valid schema.
unsafe impl<T: Sync + Send + 'static> HasSchema for Handle<T> {
    schema_impl_for_handle!();
}
// SAFE: We return a valid schema.
unsafe impl HasSchema for UntypedHandle {
    schema_impl_for_handle!();
}
