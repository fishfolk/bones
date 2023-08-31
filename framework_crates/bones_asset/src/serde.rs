use erased_serde::Deserializer;
use serde::{de::Error, Deserialize};

use crate::prelude::*;

/// Derivable schema [`type_data`][Schema::type_data] for types that implement
/// [`Deserialize`][serde::Deserialize].
///
/// This allows you use serde to implement custom deserialization logic instead of the default one
/// used for `#[repr(C)]` structs that implement [`HasSchema`].
#[derive(HasSchema)]
#[schema(no_clone, no_default)]
pub struct SchemaDeserialize {
    /// The function that may be used to deserialize the type.
    pub deserialize_fn: for<'a, 'de> fn(
        SchemaRefMut<'a, 'a>,
        deserializer: &'a mut dyn Deserializer<'de>,
    ) -> Result<(), erased_serde::Error>,
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
