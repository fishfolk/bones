//! Global schema registry.

use std::sync::{
    atomic::{AtomicU32, Ordering::SeqCst},
    OnceLock,
};

use bones_utils::{parking_lot::RwLock, *};

use crate::{Schema, SchemaData, SchemaLayoutInfo};

/// A unique identifier for a schema registered in the [`SCHEMA_REGISTRY`].
#[derive(Debug, Hash, PartialEq, Eq, Copy, Clone)]
pub struct SchemaId {
    id: u32,
}

/// A schema registry that alloates [`SchemaId`] for [`SchemaData`] and returns a registered
/// [`&'static Schema`][Schema].
pub struct SchemaRegistry {
    next_id: AtomicU32,
    state: OnceLock<RwLock<RegistryState>>,
}

#[derive(Default)]
struct RegistryState {
    schemas: HashMap<SchemaId, &'static Schema>,
}

impl SchemaRegistry {
    /// Register a schema with the registry.
    pub fn register(&self, schema_data: SchemaData) -> &'static Schema {
        let state = self.state.get_or_init(default);

        // Allocate a new schema ID
        let id = SchemaId {
            id: self.next_id.fetch_add(1, SeqCst),
        };
        assert_ne!(id.id, u32::MAX, "Exhausted all {} schema IDs", u32::MAX);

        // Compute the schema layout info so we can cache it with the Schema.
        let SchemaLayoutInfo {
            layout,
            field_offsets,
        } = schema_data.compute_layout_info();

        // Leak the field offsets to get static references
        let field_offsets: Box<_> = field_offsets
            .into_iter()
            .map(|(name, offset)| {
                (
                    name.map(|n| Box::leak(Box::new(n.to_string())).as_str()),
                    offset,
                )
            })
            .collect();
        let field_offsets = Box::leak(field_offsets);

        // Leak the schema to get a static reference
        let schema = Box::leak(Box::new(Schema {
            id,
            data: schema_data,
            layout,
            field_offsets,
        }));

        // Inser the schema into the registry.
        let entry = state.write().schemas.insert(id, schema);
        debug_assert!(entry.is_none(), "SchemaId already used!");

        schema
    }

    /// Get a `'static` reference to the schema associated to the given schema ID.
    pub fn get(&self, id: SchemaId) -> &'static Schema {
        let schemas = self.state.get_or_init(default);

        schemas
            .read()
            .schemas
            .get(&id)
            .expect("Reflection bug, schema Id created without associated registration")
    }
}

/// Global [`SchemaRegistry`] used to
pub static SCHEMA_REGISTRY: SchemaRegistry = SchemaRegistry {
    next_id: AtomicU32::new(0),
    state: OnceLock::new(),
};
