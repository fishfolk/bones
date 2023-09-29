//! Global schema registry.

use std::{
    alloc::Layout,
    sync::{
        atomic::{AtomicU32, Ordering::SeqCst},
        OnceLock,
    },
};

use bones_utils::{parking_lot::RwLock, *};

use crate::prelude::*;

/// A unique identifier for a schema registered in the [`SCHEMA_REGISTRY`].
#[derive(Debug, Hash, PartialEq, Eq, Copy, Clone)]
pub struct SchemaId {
    id: u32,
}

impl SchemaId {
    /// Get the schema associated to the ID.
    pub fn schema(&self) -> &'static Schema {
        SCHEMA_REGISTRY.get(*self)
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
    state: OnceLock<RwLock<RegistryState>>,
}

/// The internal state o the [`SchemaRegistry`].
#[derive(Default)]
pub struct RegistryState {
    /// The registered schemas.
    pub schemas: HashMap<SchemaId, &'static Schema>,
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
            .map(|(name, offset)| (name.map(|n| n.to_string()), offset))
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

    /// Borrow the registry state for reading.
    ///
    /// > **Note:** This locks the registry for reading, preventing access by things that may need
    /// > to register schemas, so it is best to hold the lock for as short as possible.
    pub fn borrow(&self) -> bones_utils::parking_lot::RwLockReadGuard<RegistryState> {
        self.state.get_or_init(default).read()
    }
}

/// Global [`SchemaRegistry`] used to register [`SchemaData`]s and produce [`Schema`]s.
pub static SCHEMA_REGISTRY: SchemaRegistry = SchemaRegistry {
    next_id: AtomicU32::new(0),
    state: OnceLock::new(),
};
