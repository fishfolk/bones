//! Global schema registry.

use std::sync::{
    atomic::{AtomicU32, Ordering::SeqCst},
    OnceLock,
};

use bones_utils::{parking_lot::RwLock, *};

use crate::Schema;

/// A unique identifier for a schema registered in the [`SCHEMA_REGISTRY`].
#[derive(Debug, Hash, PartialEq, Eq, Copy, Clone)]
pub struct SchemaId {
    id: u32,
}

/// An ID registry that simply allows you to obtain unique, non-clonable [`RegistryId`]s.
pub struct SchemaRegistry {
    next_id: AtomicU32,
    schemas: OnceLock<RwLock<HashMap<SchemaId, &'static Schema>>>,
}

impl SchemaRegistry {
    /// Register a schema with the registry.
    pub fn register(&self, mut schema: Schema) -> (SchemaId, &'static Schema) {
        let schemas = self.schemas.get_or_init(default);

        // Allocate a new schema ID
        let id = SchemaId {
            id: self.next_id.fetch_add(1, SeqCst),
        };
        assert_ne!(id.id, u32::MAX, "Exhausted all {} schema IDs", u32::MAX);

        // Update the schema ID
        schema.id = Some(id);

        // Leak the schema to create a static reference.
        let schema = Box::leak(Box::new(schema));

        // Inser the schema into the registry.
        let entry = schemas.write().insert(id, schema);
        debug_assert!(entry.is_none(), "SchemaId already used!");

        // Return the ID and reference
        (id, schema)
    }
}

/// Global [`SchemaRegistry`] used to
pub static SCHEMA_REGISTRY: SchemaRegistry = SchemaRegistry {
    next_id: AtomicU32::new(0),
    schemas: OnceLock::new(),
};
