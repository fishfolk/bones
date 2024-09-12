//! Implementation of [`DesyncHash`] for [`SchemaRef`].

use std::{any::type_name, hash::Hasher};

use bones_utils::DesyncHash;

use crate::{prelude::*, ptr::SchemaRef, FromType, HasSchema, Schema, SchemaData};

/// Used in [`Schema`] [`TypeDatas`] to optionally implement desync hash.
pub struct SchemaDesyncHash {
    /// Desync hash fn pointer
    pub desync_hash_fn: for<'a> fn(SchemaRef<'a>, hasher: &mut dyn Hasher),
}

unsafe impl HasSchema for SchemaDesyncHash {
    fn schema() -> &'static crate::Schema {
        use std::{alloc::Layout, any::TypeId, sync::OnceLock};
        static S: OnceLock<&'static Schema> = OnceLock::new();
        let layout = Layout::new::<Self>();
        S.get_or_init(|| {
            SCHEMA_REGISTRY.register(SchemaData {
                name: type_name::<Self>().into(),
                full_name: format!("{}::{}", module_path!(), type_name::<Self>()).into(),
                kind: SchemaKind::Primitive(Primitive::Opaque {
                    size: layout.size(),
                    align: layout.align(),
                }),
                type_id: Some(TypeId::of::<Self>()),
                clone_fn: None,
                drop_fn: None,
                default_fn: None,
                hash_fn: None,
                eq_fn: None,
                type_data: Default::default(),
            })
        })
    }
}

impl<T: HasSchema + DesyncHash> FromType<T> for SchemaDesyncHash {
    fn from_type() -> Self {
        SchemaDesyncHash {
            desync_hash_fn: |reference, hasher| {
                T::schema()
                    .ensure_match(reference.schema())
                    .expect("Schema type does not match schema ref.");

                unsafe {
                    DesyncHash::hash(&*reference.as_ptr().cast::<T>(), hasher);
                }
            },
        }
    }
}

impl<'a> DesyncHash for SchemaRef<'a> {
    fn hash(&self, hasher: &mut dyn std::hash::Hasher) {
        if let Some(schema_desync_hash) = self.schema().type_data.get::<SchemaDesyncHash>() {
            (schema_desync_hash.desync_hash_fn)(*self, hasher);
        }
    }
}

impl<'a> DesyncHash for SchemaRefMut<'a> {
    fn hash(&self, hasher: &mut dyn std::hash::Hasher) {
        if let Some(schema_desync_hash) = self.schema().type_data.get::<SchemaDesyncHash>() {
            (schema_desync_hash.desync_hash_fn)(self.as_ref(), hasher);
        }
    }
}

impl DesyncHash for SchemaId {
    fn hash(&self, hasher: &mut dyn std::hash::Hasher) {
        hasher.write_u32(self.id());
    }
}

impl<T: DesyncHash + HasSchema> DesyncHash for SVec<T> {
    fn hash(&self, hasher: &mut dyn std::hash::Hasher) {
        for value in self {
            value.hash(hasher);
        }
    }
}

impl<K: DesyncHash + HasSchema, V: DesyncHash + HasSchema> DesyncHash for SMap<K, V> {
    fn hash(&self, hasher: &mut dyn std::hash::Hasher) {
        for (key, value) in self.iter() {
            key.hash(hasher);
            value.hash(hasher);
        }
    }
}
