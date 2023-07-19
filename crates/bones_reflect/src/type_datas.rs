use bones_utils::HashMap;

use super::*;

/// Container for storing type datas.
#[derive(Clone, Debug, Default)]
pub struct TypeDatas(pub HashMap<Ulid, SchemaBox>);
impl TypeDatas {
    /// Get a type data out of the store.
    pub fn get<T: TypeData>(&self) -> Option<&T> {
        self.0.get(&T::TYPE_DATA_ID).map(|x| x.cast())
    }
}

/// Trait implemented for types that can produce an instance of themselves from a type.
pub trait FromType<T> {
    /// Return the data for the type.
    fn from_type() -> Self;
}

/// Trait implemented for Rust types that are used as [`Schema::type_data`].
pub trait TypeData: HasSchema {
    /// The unique ID of the type data.
    const TYPE_DATA_ID: Ulid;
}
