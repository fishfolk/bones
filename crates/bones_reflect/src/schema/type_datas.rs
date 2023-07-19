use super::*;

use std::sync::OnceLock;

/// Container for storing type datas.
#[derive(Clone, Debug, Default)]
pub struct TypeDatas(pub HashMap<Ulid, SchemaBox>);
impl TypeDatas {
    pub fn get<T: TypeData>(&self) -> &T {
        self.0.get(&T::TYPE_DATA_ID).unwrap().cast()
    }
}

pub trait FromType<T> {
    /// Return the data for the type.
    fn from_type() -> Self;
}

/// Trait implemented for Rust types that are used as [`Schema::type_data`].
pub trait TypeData: HasSchema {
    /// The unique ID of the type data.
    const TYPE_DATA_ID: Ulid;
}

/// Helper to implement an opaque schema for a struct
macro_rules! opaque_schema {
    (
        $(#[$attrs:meta])*
        pub struct $name:ident {
            $($fields:tt)*
        }
    ) => {
        $(#[$attrs])*
        pub struct $name {
            $($fields)*
        }
        unsafe impl HasSchema for $name {
            fn schema() -> &'static Schema {
                static S: OnceLock<Schema> = OnceLock::new();
                let layout = Layout::new::<Self>();
                S.get_or_init(|| Schema {
                    kind: SchemaKind::Primitive(Primitive::Opaque {
                        size: layout.size(),
                        align: layout.align(),
                    }),
                    type_id: Some(TypeId::of::<Self>()),
                    type_data: Default::default(),
                })
            }
        }
    };
}

pub use type_id::*;
mod type_id {

    use std::sync::OnceLock;

    use super::*;

    opaque_schema! {
        /// Type data that stores the Rust type ID.
        pub struct SchemaRustTypeId {
            /// The [`TypeId`] of the Rust type represented by this schema.
            pub id: TypeId,
        }
    }
    impl<T: 'static> FromType<T> for SchemaRustTypeId {
        fn from_type() -> Self {
            Self {
                id: TypeId::of::<T>(),
            }
        }
    }
    impl TypeData for SchemaRustTypeId {
        const TYPE_DATA_ID: Ulid = Ulid(2042739562331224454970423095469294652);
    }
}

pub use default::*;
mod default {
    use super::*;

    opaque_schema! {
        /// Type data for producing a default value of a type.
        pub struct SchemaDefault {
            /// The default function, which takes a mutable pointer that the default value of the data
            /// will be written to.
            pub default_fn: unsafe extern "C-unwind" fn(ptr: *mut u8),
        }
    }
    impl<T: Default> FromType<T> for SchemaDefault {
        fn from_type() -> Self {
            SchemaDefault {
                default_fn: <T as RawDefault>::raw_default,
            }
        }
    }
    impl TypeData for SchemaDefault {
        const TYPE_DATA_ID: Ulid = Ulid(2042759988895161038537609275676869026);
    }
    trait RawDefault {
        unsafe extern "C-unwind" fn raw_default(dst: *mut u8);
    }
    impl<T: Default> RawDefault for T {
        unsafe extern "C-unwind" fn raw_default(dst: *mut u8) {
            let d = T::default();
            (dst as *mut T).write(d)
        }
    }
}

pub use drop::*;
mod drop {
    use super::*;

    opaque_schema! {
        /// Type data for cloning a type.
        pub struct SchemaDrop {
            /// The function to use to drop the type.
            pub drop_fn: unsafe extern "C-unwind" fn(ptr: *mut u8),
        }
    }
    impl<T> FromType<T> for SchemaDrop {
        fn from_type() -> Self {
            Self {
                drop_fn: <T as RawDrop>::raw_drop,
            }
        }
    }
    impl TypeData for SchemaDrop {
        const TYPE_DATA_ID: Ulid = Ulid(2042757515742035456533581765450524891);
    }
    trait RawDrop {
        unsafe extern "C-unwind" fn raw_drop(ptr: *mut u8);
    }
    impl<T> RawDrop for T {
        unsafe extern "C-unwind" fn raw_drop(ptr: *mut u8) {
            if std::mem::needs_drop::<T>() {
                (ptr as *mut T).drop_in_place()
            }
        }
    }
}

pub use clone::*;
mod clone {
    use super::*;

    opaque_schema! {
        /// Type data for cloning a type.
        pub struct SchemaClone {
            /// The function to use to clone the type.
            pub clone_fn: unsafe extern "C-unwind" fn(src: *const u8, dst: *mut u8),
        }
    }
    impl<T: Clone> FromType<T> for SchemaClone {
        fn from_type() -> Self {
            Self {
                clone_fn: <T as RawClone>::raw_clone,
            }
        }
    }
    impl TypeData for SchemaClone {
        const TYPE_DATA_ID: Ulid = Ulid(2042747092199054873714370416906498640);
    }
    trait RawClone {
        unsafe extern "C-unwind" fn raw_clone(src: *const u8, dst: *mut u8);
    }
    impl<T: Clone> RawClone for T {
        unsafe extern "C-unwind" fn raw_clone(src: *const u8, dst: *mut u8) {
            let t = &*(src as *const T);
            let t = t.clone();
            (dst as *mut T).write(t)
        }
    }
}
