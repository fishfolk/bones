use super::*;

impl<'de> Deserialize<'de> for NestedSchema {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = Schema::deserialize(deserializer)?;
        Ok(NestedSchema::Boxed(Box::new(s)))
    }
}

impl<'de> serde::Deserialize<'de> for StructSchema {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(StructSchemaVisitor)
    }
}

struct StructSchemaVisitor;
impl<'de> serde::de::Visitor<'de> for StructSchemaVisitor {
    type Value = StructSchema;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a struct definition")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut struct_schema = StructSchema {
            fields: Vec::with_capacity(seq.size_hint().unwrap_or(0)),
        };
        while let Some(schema) = seq.next_element()? {
            struct_schema
                .fields
                .push(StructField { name: None, schema });
        }
        Ok(struct_schema)
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut struct_schema = StructSchema {
            fields: Vec::with_capacity(map.size_hint().unwrap_or(0)),
        };
        while let Some((name, schema)) = map.next_entry()? {
            struct_schema.fields.push(StructField {
                name: Some(name),
                schema,
            });
        }
        Ok(struct_schema)
    }
}
