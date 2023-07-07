use crate::prelude::HasSchema;

pub trait Typed: HasSchema {
    /// Returns the compile-time [info] for the underlying type.
    ///
    /// [info]: TypeInfo
    fn type_info() -> &'static TypeInfo;
}

// ===================================== Type Info ===================================== //

#[derive(Debug, Clone)]
pub enum TypeInfo {}

impl TypeInfo {}
