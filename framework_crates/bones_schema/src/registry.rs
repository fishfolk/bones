//! Global schema registry.

use std::{
    alloc::Layout,
    sync::atomic::{AtomicU32, Ordering::SeqCst},
};

use append_only_vec::AppendOnlyVec;
use bones_utils::*;

use crate::prelude::*;

/// A unique identifier for a schema registered in the [`SCHEMA_REGISTRY`].
#[derive(Debug, Hash, PartialEq, Eq, Copy, Clone)]
pub struct SchemaId {
    id: u32,
}

impl SchemaId {
    /// Get schema id
    pub fn id(&self) -> u32 {
        self.id
    }
}

// Note: The schema type is here in the registry module to prevent modification of registered
// schemas by other modules. The idea is that once a schema is registered, it is unchangable and
// "certified" so to speak.
#[doc(hidden)]
/// A schema registered with the [`SCHEMA_REGISTRY`].
///
/// ## Known Limitations
///
/// Currently there isn't a known-safe way to construct a schema for a recursive struct. For
/// example, this struct is troublesome:
///
/// ```rust
/// struct Data {
///     others: Vec<Data>,
/// }
/// ```
///
/// This is because nested schemas are required to be a `&'static Schema`, and it would probalby
/// require unsafe code to create a schema that references itself.
///
/// If this is a problem for your use-case, please open an issue. We would like to remove this
/// limitation or find a suitable workaround in the future.
#[derive(Deref, Clone, Debug)]
pub struct Schema {
    id: SchemaId,
    #[deref]
    data: SchemaData,
    layout: Layout,
    field_offsets: &'static [(Option<String>, usize)],
}

impl PartialEq for Schema {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for Schema {}

impl Schema {
    /// Get the registered, unique ID of the [`Schema`].
    #[inline]
    pub fn id(&self) -> SchemaId {
        self.id
    }

    /// Get a static reference to the [`Schema`] that was registered.
    #[inline]
    pub fn schema(&self) -> &SchemaData {
        &self.data
    }

    /// Get the [`Layout`] of the [`Schema`].
    #[inline]
    pub fn layout(&self) -> Layout {
        self.layout
    }

    /// If this schema represents a struct, this returns the list of fields, with the names of the
    /// fields, and their byte offsets from the beginning of the struct.
    #[inline]
    pub fn field_offsets(&self) -> &'static [(Option<String>, usize)] {
        self.field_offsets
    }

    /// Helper function to make sure that this schema matches another or return a
    /// [`SchemaMismatchError`].
    pub fn ensure_match(&self, other: &Self) -> Result<(), SchemaMismatchError> {
        if self == other {
            Ok(())
        } else {
            Err(SchemaMismatchError)
        }
    }
}

/// A schema registry that alloates [`SchemaId`]s for [`SchemaData`]s and returns a registered
/// [`&'static Schema`][Schema].
pub struct SchemaRegistry {
    next_id: AtomicU32,
    /// The registered schemas.
    pub schemas: AppendOnlyVec<Schema>,
}

impl SchemaRegistry {
    /// Register a schema with the registry.
    #[track_caller]
    pub fn register(&self, schema_data: SchemaData) -> &Schema {
        // Allocate a new schema ID
        let id = SchemaId {
            id: self.next_id.fetch_add(1, SeqCst),
        };
        assert_ne!(id.id, u32::MAX, "Exhausted all {} schema IDs", u32::MAX);

        // Compute the schema layout info so we can cache it with the Schema.
        let SchemaLayoutInfo {
            layout,
            field_offsets,
        } = schema_data.kind.compute_layout_info();

        // Leak the field offsets to get static references
        let field_offsets: Box<_> = field_offsets
            .into_iter()
            .map(|(name, offset)| (name.map(|n| n.to_string()), offset))
            .collect();
        let field_offsets = Box::leak(field_offsets);

        // Create the schema struct.
        let schema = Schema {
            id,
            data: schema_data,
            layout,
            field_offsets,
        };

        // Insert the schema into the registry.
        let idx = self.schemas.push(schema);

        &self.schemas[idx]
    }
}

/// Global [`SchemaRegistry`] used to register [`SchemaData`]s and produce [`Schema`]s.
pub static SCHEMA_REGISTRY: SchemaRegistry = SchemaRegistry {
    next_id: AtomicU32::new(0),
    schemas: AppendOnlyVec::new(),
};

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn registry_smoke() {
        let mut schemas = Vec::new();
        for i in 0..100 {
            let data = SchemaData {
                name: format!("data{i}").into(),
                full_name: format!("data{i}").into(),
                kind: SchemaKind::Primitive(Primitive::U8),
                type_data: default(),
                type_id: None,
                clone_fn: None,
                drop_fn: None,
                default_fn: None,
                hash_fn: None,
                eq_fn: None,
            };

            let schema = SCHEMA_REGISTRY.register(data.clone());
            schemas.push(schema);
        }

        for (i, schema) in schemas.iter().enumerate() {
            assert_eq!(schema.data.name, format!("data{i}"));
        }
    }
}
