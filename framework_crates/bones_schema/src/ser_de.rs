use std::any::type_name;

use erased_serde::Deserializer;
use serde::{de::Error, Deserialize};

use crate::prelude::*;

impl<'de> Deserialize<'de> for &'static Schema {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let data = SchemaData::deserialize(deserializer)?;
        Ok(SCHEMA_REGISTRY.register(data))
    }
}

impl<'de> serde::Deserialize<'de> for StructSchemaInfo {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(StructSchemaVisitor)
    }
}

struct StructSchemaVisitor;
impl<'de> serde::de::Visitor<'de> for StructSchemaVisitor {
    type Value = StructSchemaInfo;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a struct definition")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut struct_schema = StructSchemaInfo {
            fields: Vec::with_capacity(seq.size_hint().unwrap_or(0)),
        };
        while let Some(schema) = seq.next_element()? {
            struct_schema
                .fields
                .push(StructFieldInfo { name: None, schema });
        }
        Ok(struct_schema)
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut struct_schema = StructSchemaInfo {
            fields: Vec::with_capacity(map.size_hint().unwrap_or(0)),
        };
        while let Some((name, schema)) = map.next_entry()? {
            struct_schema.fields.push(StructFieldInfo {
                name: Some(name),
                schema,
            });
        }
        Ok(struct_schema)
    }
}

/// Derivable schema [`type_data`][SchemaData::type_data] for types that implement
/// [`Deserialize`][serde::Deserialize].
///
/// This allows you use serde to implement custom deserialization logic instead of the default one
/// used for `#[repr(C)]` structs that implement [`HasSchema`].
pub struct SchemaDeserialize {
    /// The function that may be used to deserialize the type.
    pub deserialize_fn: for<'a, 'de> fn(
        SchemaRefMut<'a, 'a>,
        deserializer: &'a mut dyn Deserializer<'de>,
    ) -> Result<(), erased_serde::Error>,
}

unsafe impl HasSchema for SchemaDeserialize {
    fn schema() -> &'static Schema {
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

impl SchemaDeserialize {
    /// Use this [`SchemaDeserialize`] to deserialize data from the `deserializer` into the
    /// `reference`.
    pub fn deserialize<'a, 'de, D>(
        &self,
        reference: SchemaRefMut<'a, 'a>,
        deserializer: D,
    ) -> Result<(), D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let mut erased = <dyn erased_serde::Deserializer>::erase(deserializer);
        (self.deserialize_fn)(reference, &mut erased)
            .map_err(<<D as serde::Deserializer<'de>>::Error as serde::de::Error>::custom)
    }
}

impl<T: HasSchema + for<'de> Deserialize<'de>> FromType<T> for SchemaDeserialize {
    fn from_type() -> Self {
        SchemaDeserialize {
            deserialize_fn: |reference, deserializer| {
                T::schema()
                    .ensure_match(reference.schema())
                    .map_err(|e| erased_serde::Error::custom(e.to_string()))?;
                let data = T::deserialize(deserializer)?;

                // SOUND: we ensured schemas match.
                unsafe {
                    reference.as_ptr().cast::<T>().write(data);
                }

                Ok(())
            },
        }
    }
}
