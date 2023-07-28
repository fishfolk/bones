//! Global schema registry.

use std::sync::{
    atomic::{AtomicU32, Ordering::SeqCst},
    OnceLock,
};

use bones_utils::{parking_lot::RwLock, *};

use crate::{SchemaData, Schema};

/// A unique identifier for a schema registered in the [`SCHEMA_REGISTRY`].
#[derive(Debug, Hash, PartialEq, Eq, Copy, Clone)]
pub struct SchemaId {
    id: u32,
}

/// A schema registry that alloates [`SchemaId`] for [`SchemaData`] and returns a registered
/// [`&'static Schema`][Schema].
pub struct SchemaRegistry {
    next_id: AtomicU32,
    schemas: OnceLock<RwLock<HashMap<SchemaId, &'static Schema>>>,
}

impl SchemaRegistry {
    /// Register a schema with the registry.
    pub fn register(&self, schema_data: SchemaData) -> &'static Schema {
        let schemas = self.schemas.get_or_init(default);

        // Allocate a new schema ID
        let id = SchemaId {
            id: self.next_id.fetch_add(1, SeqCst),
        };
        assert_ne!(id.id, u32::MAX, "Exhausted all {} schema IDs", u32::MAX);

        // Leak the registered schema to produce a static reference
        let schema = Box::leak(Box::new(Schema {
            id,
            data: schema_data,
        }));

        // Inser the schema into the registry.
        let entry = schemas.write().insert(id, schema);
        debug_assert!(entry.is_none(), "SchemaId already used!");

        schema
    }

    /// Get a `'static` reference to the schema associated to the given schema ID.
    pub fn get(&self, id: SchemaId) -> &'static Schema {
        let schemas = self.schemas.get_or_init(default);

        schemas
            .read()
            .get(&id)
            .expect("Reflection bug, schema Id created without associated registration")
    }
}

/// Global [`SchemaRegistry`] used to
pub static SCHEMA_REGISTRY: SchemaRegistry = SchemaRegistry {
    next_id: AtomicU32::new(0),
    schemas: OnceLock::new(),
};
