use std::{alloc::Layout, any::TypeId, marker::PhantomData, sync::OnceLock};

use bones_reflect::prelude::*;
use bones_utils::HashMap;
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

//
// Schema implementations
//

/// This is the [`Schema::type_data`] key for the asset handle marker.
///
/// The value is unimportant, as simply having a type data with this key indicates that the type is
/// an asset handle. This information is used when deserializing assets from a schema, to know that
/// a handle should be deserialized.
///
pub const ASSET_HANDLE_TYPE_DATA: Ulid = Ulid(2042034270141692702617108034127624904);

/// Helper to avoid typing the duplicate implementations of [`HasSchema`] for typed and untyped
/// handles.
macro_rules! schema_impl_for_handle {
    () => {
        fn schema() -> &'static bones_reflect::Schema {
            static S: OnceLock<Schema> = OnceLock::new();
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
            S.get_or_init(|| Schema {
                type_id: Some(TypeId::of::<Self>()),
                kind: SchemaKind::Struct(StructSchema {
                    fields: vec![StructField {
                        name: Some("id".into()),
                        schema: Schema {
                            type_id: Some(TypeId::of::<Ulid>()),
                            kind: SchemaKind::Primitive(Primitive::U128),
                            type_data: Default::default(),
                        },
                    }],
                }),
                type_data: {
                    let mut h = HashMap::with_capacity(1);
                    // TODO: Make that a `SchemaBox::new(())` once schema box can handle ZSTs.
                    h.insert(ASSET_HANDLE_TYPE_DATA, SchemaBox::new(true));
                    h
                },
            })
        }
    };
}

// SAFE: We return a valid schema.
unsafe impl<T: 'static> HasSchema for Handle<T> {
    schema_impl_for_handle!();
}
// SAFE: We return a valid schema.
unsafe impl HasSchema for UntypedHandle {
    schema_impl_for_handle!();
}
