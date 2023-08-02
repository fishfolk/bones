use serde::Deserialize;

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
