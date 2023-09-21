use std::any::type_name;

use erased_serde::Deserializer;
use serde::{
    de::{DeserializeSeed, Error},
    Deserialize, Serialize,
};

use crate::prelude::*;

pub use serializer_deserializer::*;
mod serializer_deserializer {
    use bones_utils::{ustr, Ustr};
    use serde::{
        de::{VariantAccess, Visitor},
        ser::{SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant},
    };

    use super::*;

    /// A struct that implements [`Serialize`] and wraps around a [`SchemaRef`] to serialize the value
    /// using it's schema.
    ///
    /// This will error if there are opaque types in the schema ref that cannot be serialized.
    pub struct SchemaSerializer<'a>(pub SchemaRef<'a>);

    impl<'a> Serialize for SchemaSerializer<'a> {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            // Specifically handle `Ustr`
            if let Ok(u) = self.0.try_cast::<Ustr>() {
                return serializer.serialize_str(u);
            }

            match &self.0.schema().kind {
                SchemaKind::Struct(s) => {
                    if s.fields.len() == 1 {
                        // Serialize just the inner field
                        // SOUND: it is safe to cast a struct with one field to it's inner type.
                        SchemaSerializer(unsafe {
                            SchemaRef::from_ptr_schema(self.0.as_ptr(), s.fields[0].schema)
                        })
                        .serialize(serializer)
                    } else {
                        let named = s.fields.first().map(|x| x.name.is_some()).unwrap_or(false);

                        if named {
                            let mut ser_struct = serializer
                                .serialize_struct(&self.0.schema().name, s.fields.len())?;
                            for (i, field) in s.fields.iter().enumerate() {
                                ser_struct.serialize_field(
                                    field.name.as_ref().unwrap(),
                                    &SchemaSerializer(self.0.field(i)),
                                )?;
                            }
                            ser_struct.end()
                        } else {
                            let mut seq = serializer.serialize_seq(Some(s.fields.len()))?;
                            for i in 0..s.fields.len() {
                                seq.serialize_element(&SchemaSerializer(self.0.field(i)))?;
                            }
                            seq.end()
                        }
                    }
                }
                SchemaKind::Vec(_) => {
                    // SOUND: schema asserts this is a schema vec.
                    let v = unsafe { self.0.deref::<SchemaVec>() };
                    let mut seq = serializer.serialize_seq(Some(v.len()))?;
                    for item in v {
                        seq.serialize_element(&SchemaSerializer(item))?;
                    }
                    seq.end()
                }
                SchemaKind::Enum(e) => {
                    let variant_idx = match e.tag_type {
                        EnumTagType::U8 => self.0.as_ptr().cast::<u8>() as u32,
                        EnumTagType::U16 => self.0.as_ptr().cast::<u16>() as u32,
                        EnumTagType::U32 => self.0.as_ptr().cast::<u32>() as u32,
                    };

                    let variant_info = &e.variants[variant_idx as usize];
                    let struct_info = variant_info.schema.kind.as_struct().unwrap();

                    if struct_info.fields.is_empty() {
                        serializer.serialize_unit_variant(
                            &self.0.schema().name,
                            variant_idx,
                            &variant_info.name,
                        )
                    } else {
                        let value_offset = self.0.schema().field_offsets()[0].1;
                        // SOUND: we are returning a reference to the enum variants value
                        // offset by the size of it's discriminant, which is valid.
                        let value_ref = unsafe {
                            SchemaRef::from_ptr_schema(
                                self.0.as_ptr().add(value_offset),
                                variant_info.schema,
                            )
                        };

                        if struct_info.fields.len() == 1 {
                            serializer.serialize_newtype_variant(
                                &self.0.schema().name,
                                variant_idx,
                                &variant_info.name,
                                &SchemaSerializer(value_ref),
                            )
                        } else {
                            let mut ser_struct = serializer.serialize_struct_variant(
                                &self.0.schema().name,
                                variant_idx,
                                &variant_info.name,
                                struct_info.fields.len(),
                            )?;

                            for (i, field) in struct_info.fields.iter().enumerate() {
                                ser_struct.serialize_field(
                                    field.name.as_ref().unwrap(),
                                    &SchemaSerializer(value_ref.field(i)),
                                )?;
                            }

                            ser_struct.end()
                        }
                    }
                }
                SchemaKind::Map { .. } => {
                    // SOUND: schema asserts this is a schema vec.
                    let m = unsafe { self.0.deref::<SchemaMap>() };
                    let mut map = serializer.serialize_map(Some(m.len()))?;
                    for (key, value) in m {
                        map.serialize_entry(&SchemaSerializer(key), &SchemaSerializer(value))?;
                    }
                    map.end()
                }
                SchemaKind::Box(_) => {
                    // SOUND: schema asserts this is a schema box.
                    let b = unsafe { self.0.deref::<SchemaBox>() };
                    SchemaSerializer(b.as_ref()).serialize(serializer)
                }
                SchemaKind::Primitive(p) => match p {
                    Primitive::Bool => serializer.serialize_bool(*self.0.cast::<bool>()),
                    Primitive::U8 => serializer.serialize_u8(*self.0.cast::<u8>()),
                    Primitive::U16 => serializer.serialize_u16(*self.0.cast::<u16>()),
                    Primitive::U32 => serializer.serialize_u32(*self.0.cast::<u32>()),
                    Primitive::U64 => serializer.serialize_u64(*self.0.cast::<u64>()),
                    Primitive::U128 => serializer.serialize_u128(*self.0.cast::<u128>()),
                    Primitive::I8 => serializer.serialize_i8(*self.0.cast::<i8>()),
                    Primitive::I16 => serializer.serialize_i16(*self.0.cast::<i16>()),
                    Primitive::I32 => serializer.serialize_i32(*self.0.cast::<i32>()),
                    Primitive::I64 => serializer.serialize_i64(*self.0.cast::<i64>()),
                    Primitive::I128 => serializer.serialize_i128(*self.0.cast::<i128>()),
                    Primitive::F32 => serializer.serialize_f32(*self.0.cast::<f32>()),
                    Primitive::F64 => serializer.serialize_f64(*self.0.cast::<f64>()),
                    Primitive::String => serializer.serialize_str(self.0.cast::<String>()),
                    Primitive::Opaque { .. } => {
                        use serde::ser::Error;
                        Err(S::Error::custom("Cannot serialize opaque types"))
                    }
                },
            }
        }
    }

    /// A struct that implements [`DeserializeSeed`] and can be used to deserialize values matching a
    /// given [`Schema`].
    ///
    /// This will error if there are opaque types in the schema that cannot be deserialized.
    pub struct SchemaDeserializer(pub &'static Schema);

    impl<'de> DeserializeSeed<'de> for SchemaDeserializer {
        type Value = SchemaBox;

        fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            // Allocate the object.
            let mut ptr = SchemaBox::default(self.0);

            // Deserialize into it
            ptr.as_mut().deserialize(deserializer)?;

            Ok(ptr)
        }
    }

    impl<'a, 'b, 'de> DeserializeSeed<'de> for SchemaRefMut<'a, 'b> {
        type Value = ();

        fn deserialize<D>(mut self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            // Use custom deserializer if present.
            if let Some(schema_deserialize) = self.schema().type_data.get::<SchemaDeserialize>() {
                return schema_deserialize.deserialize(self, deserializer);
            }

            match &self.schema().kind {
                SchemaKind::Struct(s) => {
                    // If this is a newtype struct
                    if s.fields.len() == 1 {
                        // Deserialize it as the inner type
                        // SOUND: it is safe to cast a struct with one field to it's field type
                        unsafe { SchemaRefMut::from_ptr_schema(self.as_ptr(), s.fields[0].schema) }
                            .deserialize(deserializer)?
                    } else {
                        deserializer.deserialize_any(StructVisitor(self))?
                    }
                }
                SchemaKind::Vec(_) => deserializer.deserialize_seq(VecVisitor(self))?,
                SchemaKind::Map { .. } => deserializer.deserialize_map(MapVisitor(self))?,
                SchemaKind::Enum(_) => {
                    deserializer.deserialize_enum(&self.schema().name, &[], EnumVisitor(self))?
                }
                SchemaKind::Box(_) => {
                    // SOUND: schema asserts this is a `SchemaBox`
                    let b = unsafe { self.deref_mut::<SchemaBox>() };

                    b.as_mut().deserialize(deserializer)?
                }
                SchemaKind::Primitive(p) => {
                    match p {
                        Primitive::Bool => *self.cast_mut() = bool::deserialize(deserializer)?,
                        Primitive::U8 => *self.cast_mut() = u8::deserialize(deserializer)?,
                        Primitive::U16 => *self.cast_mut() = u16::deserialize(deserializer)?,
                        Primitive::U32 => *self.cast_mut() = u32::deserialize(deserializer)?,
                        Primitive::U64 => *self.cast_mut() = u64::deserialize(deserializer)?,
                        Primitive::U128 => *self.cast_mut() = u128::deserialize(deserializer)?,
                        Primitive::I8 => *self.cast_mut() = i8::deserialize(deserializer)?,
                        Primitive::I16 => *self.cast_mut() = i16::deserialize(deserializer)?,
                        Primitive::I32 => *self.cast_mut() = i32::deserialize(deserializer)?,
                        Primitive::I64 => *self.cast_mut() = i64::deserialize(deserializer)?,
                        Primitive::I128 => *self.cast_mut() = i128::deserialize(deserializer)?,
                        Primitive::F32 => *self.cast_mut() = f32::deserialize(deserializer)?,
                        Primitive::F64 => *self.cast_mut() = f64::deserialize(deserializer)?,
                        Primitive::String => *self.cast_mut() = String::deserialize(deserializer)?,
                        Primitive::Opaque { .. } => {
                            return Err(D::Error::custom(
                                "Opaque types must be #[repr(C)] or have `SchemaDeserialize` type \
                                data in order to be deserialized.",
                            ));
                        }
                    };
                }
            };

            Ok(())
        }
    }

    struct StructVisitor<'a, 'b>(SchemaRefMut<'a, 'b>);
    impl<'a, 'b, 'de> Visitor<'de> for StructVisitor<'a, 'b> {
        type Value = ();
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(
                formatter,
                "asset metadata matching the schema: {:#?}",
                self.0.schema()
            )
        }

        fn visit_seq<A>(mut self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            let field_count = self.0.schema().kind.as_struct().unwrap().fields.len();

            for i in 0..field_count {
                let field = self.0.get_field(i).unwrap();
                if seq.next_element_seed(field)?.is_none() {
                    break;
                }
            }

            Ok(())
        }

        fn visit_map<A>(mut self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::MapAccess<'de>,
        {
            while let Some(key) = map.next_key::<String>()? {
                match self.0.get_field(&key) {
                    Ok(field) => {
                        map.next_value_seed(field)?;
                    }
                    Err(_) => {
                        let fields = &self.0.schema().kind.as_struct().unwrap().fields;
                        let mut msg = format!("unknown field `{key}`, ");
                        if !fields.is_empty() {
                            msg += "expected one of ";
                            for (i, field) in fields.iter().enumerate() {
                                msg += &field
                                    .name
                                    .as_ref()
                                    .map(|x| format!("`{x}`"))
                                    .unwrap_or_else(|| format!("`{i}`"));
                                if i < fields.len() - 1 {
                                    msg += ", "
                                }
                            }
                        } else {
                            msg += "there are no fields"
                        }
                        return Err(A::Error::custom(msg));
                    }
                }
            }

            Ok(())
        }
    }

    struct VecVisitor<'a, 'b>(SchemaRefMut<'a, 'b>);
    impl<'a, 'b, 'de> Visitor<'de> for VecVisitor<'a, 'b> {
        type Value = ();
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(
                formatter,
                "asset metadata matching the schema: {:#?}",
                self.0.schema()
            )
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            // SOUND: schema asserts this is a SchemaVec.
            let v = unsafe { &mut *(self.0.as_ptr() as *mut SchemaVec) };
            loop {
                let item_schema = v.schema();
                let mut item = SchemaBox::default(item_schema);
                let item_ref = item.as_mut();
                if seq.next_element_seed(item_ref)?.is_none() {
                    break;
                }
                v.push_box(item);
            }

            Ok(())
        }
    }
    struct MapVisitor<'a, 'b>(SchemaRefMut<'a, 'b>);
    impl<'a, 'b, 'de> Visitor<'de> for MapVisitor<'a, 'b> {
        type Value = ();
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(
                formatter,
                "asset metadata matching the schema: {:#?}",
                self.0.schema()
            )
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::MapAccess<'de>,
        {
            // SOUND: schema asserts this is a SchemaMap.
            let v = unsafe { &mut *(self.0.as_ptr() as *mut SchemaMap) };
            let is_ustr = v.key_schema() == Ustr::schema();
            if v.key_schema() != String::schema() && !is_ustr {
                return Err(A::Error::custom(
                    "Can only deserialize maps with `String` or `Ustr` keys.",
                ));
            }
            while let Some(key) = map.next_key::<String>()? {
                let key = if is_ustr {
                    SchemaBox::new(ustr(&key))
                } else {
                    SchemaBox::new(key)
                };
                let mut value = SchemaBox::default(v.value_schema());
                map.next_value_seed(value.as_mut())?;

                v.insert_box(key, value);
            }
            Ok(())
        }
    }
    struct EnumVisitor<'a, 'b>(SchemaRefMut<'a, 'b>);
    impl<'a, 'b, 'de> Visitor<'de> for EnumVisitor<'a, 'b> {
        type Value = ();
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(
                formatter,
                "asset metadata matching the schema: {:#?}",
                self.0.schema()
            )
        }

        fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::EnumAccess<'de>,
        {
            let (value_ptr, var_access) = data.variant_seed(EnumLoad(self.0))?;
            var_access.newtype_variant_seed(value_ptr)?;
            Ok(())
        }
    }

    struct EnumLoad<'a, 'b>(SchemaRefMut<'a, 'b>);
    impl<'a, 'b, 'de> DeserializeSeed<'de> for EnumLoad<'a, 'b> {
        type Value = SchemaRefMut<'a, 'b>;

        fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let var_name = String::deserialize(deserializer)?;
            let enum_info = self.0.schema().kind.as_enum().unwrap();
            let value_offset = self.0.schema().field_offsets()[0].1;
            let (var_idx, var_schema) = enum_info
                .variants
                .iter()
                .enumerate()
                .find_map(|(idx, info)| (info.name == var_name).then_some((idx, info.schema)))
                .ok_or_else(|| {
                    D::Error::custom(format!(
                        "Unknown enum variant `{var_name}`, expected one of: {}",
                        enum_info
                            .variants
                            .iter()
                            .map(|x| format!("`{}`", x.name))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ))
                })?;

            // Write the enum variant
            // SOUND: the schema asserts that the write to the enum discriminant is valid
            match enum_info.tag_type {
                EnumTagType::U8 => unsafe { self.0.as_ptr().write(var_idx as u8) },
                EnumTagType::U16 => unsafe { self.0.as_ptr().cast::<u16>().write(var_idx as u16) },
                EnumTagType::U32 => unsafe { self.0.as_ptr().cast::<u32>().write(var_idx as u32) },
            }

            if var_schema.kind.as_struct().is_none() {
                return Err(D::Error::custom(
                    "All enum variant types must have a struct Schema",
                ));
            }

            unsafe {
                Ok(SchemaRefMut::from_ptr_schema(
                    self.0.as_ptr().add(value_offset),
                    var_schema,
                ))
            }
        }
    }
}

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

#[cfg(test)]
mod test {
    use super::*;
    use bones_schema_macros::HasSchema;

    #[derive(HasSchema, Clone, Default)]
    #[schema_module(crate)]
    #[repr(C)]
    struct MyData {
        name: String,
        age: Age,
        favorite_things: SVec<String>,
        map: SMap<String, String>,
    }

    #[derive(HasSchema, Clone, Default)]
    #[schema_module(crate)]
    #[repr(C)]
    struct Age(u32);

    const DEMO_YAML: &str = r"name: John
age: 8
favorite_things:
- jelly
- beans
map:
  hello: world
";

    #[test]
    fn schema_deserializer() {
        let deserializer = serde_yaml::Deserializer::from_str(DEMO_YAML);

        let data = SchemaDeserializer(MyData::schema())
            .deserialize(deserializer)
            .unwrap()
            .into_inner::<MyData>();

        assert_eq!(data.name, "John");
        assert_eq!(data.age.0, 8);
        assert_eq!(
            data.favorite_things,
            ["jelly".to_string(), "beans".to_string()]
                .into_iter()
                .collect::<SVec<_>>()
        );
        assert_eq!(
            data.map.into_iter().next().unwrap(),
            (&"hello".to_string(), &"world".to_string())
        );
    }

    #[test]
    fn schema_serializer() {
        let mut data = Vec::new();
        let mut serializer = serde_yaml::Serializer::new(&mut data);

        SchemaSerializer(
            MyData {
                name: "John".into(),
                age: Age(8),
                favorite_things: ["jelly".to_string(), "beans".to_string()]
                    .into_iter()
                    .collect(),
                map: [("hello".to_string(), "world".to_string())]
                    .into_iter()
                    .collect(),
            }
            .as_schema_ref(),
        )
        .serialize(&mut serializer)
        .unwrap();

        assert_eq!(DEMO_YAML, String::from_utf8(data).unwrap());
    }
}
