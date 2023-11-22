use append_only_vec::AppendOnlyVec;

use crate::prelude::*;

/// A `TypeMap`-like structure, that does not allow removing entries or updating exisintg
/// entries.
///
/// This structure doesn't require a mutable reference to insert records
#[derive(Debug)]
// TODO: Evaluate the possibility of using a `OnceMap` instead of an `AppendOnlyVec` for `TypeDatas`.
// The only reason we would use a `OnceMap` would be to improve the lookup performance and avoid
// having to iterate over the entire vec each time we need to find a type data with a particular
// schema. Since there are not usually a large number of type datas for any single type, this
// probably isn't a concern, but maybe we should do some benchmarking.
pub struct TypeDatas(AppendOnlyVec<SchemaBox>);
impl Default for TypeDatas {
    fn default() -> Self {
        Self(AppendOnlyVec::new())
    }
}
impl Clone for TypeDatas {
    fn clone(&self) -> Self {
        let clone = TypeDatas::default();
        for entry in self.0.iter() {
            clone.insert(entry.clone()).unwrap();
        }
        clone
    }
}

impl TypeDatas {
    /// Insert data into the store.
    pub fn insert<T: HasSchema>(&self, data: T) -> Result<(), TypeDataAlreadyInserted> {
        self.insert_box(SchemaBox::new(data))
    }

    /// Insert boxed data into the store.
    pub fn insert_box(&self, data: SchemaBox) -> Result<(), TypeDataAlreadyInserted> {
        let schema = data.schema();
        for entry in self.0.iter() {
            if entry.schema() == schema {
                return Err(TypeDataAlreadyInserted(schema));
            }
        }
        self.0.push(data);
        Ok(())
    }

    /// Borrow data from the store, if it exists.
    pub fn get<T: HasSchema>(&self) -> Option<&T> {
        let id = T::schema().id();
        for data in self.0.iter() {
            if data.schema().id() == id {
                return Some(data.cast_ref());
            }
        }
        None
    }

    /// Borrow data from the store, if it exists.
    pub fn get_ref(&self, id: SchemaId) -> Option<SchemaRef> {
        for data in self.0.iter() {
            if data.schema().id() == id {
                return Some(data.as_ref());
            }
        }
        None
    }

    /// Iterate over type datas.
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &SchemaBox> {
        self.0.iter()
    }
}

/// Error type for [`TypeDatas`]
#[derive(Debug)]
pub struct TypeDataAlreadyInserted(&'static Schema);

impl std::fmt::Display for TypeDataAlreadyInserted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "Type data already contains an entry of type: {}",
            self.0.full_name,
        ))
    }
}
impl std::error::Error for TypeDataAlreadyInserted {}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn smoke() {
        let tds = TypeDatas::default();

        tds.insert(String::from("hi")).unwrap();
        assert_eq!(Some("hi"), tds.get::<String>().map(|x| x.as_str()));

        tds.insert(7u32).unwrap();
        assert_eq!(Some(&7), tds.get::<u32>());

        let result = tds.insert(String::from("bye"));
        assert!(matches!(result, Err(TypeDataAlreadyInserted(_))));
    }
}
