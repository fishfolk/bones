use bones_utils::HashMap;
use parking_lot::RwLock;

use super::*;

use std::{any::TypeId, sync::OnceLock};

macro_rules! impl_primitive {
    ($t:ty, $prim:ident) => {
        unsafe impl HasSchema for $t {
            fn schema() -> &'static Schema {
                static S: OnceLock<Schema> = OnceLock::new();
                S.get_or_init(|| Schema {
                    kind: SchemaKind::Primitive(Primitive::$prim),
                    type_id: Some(TypeId::of::<$t>()),
                    type_data: Default::default(),
                })
            }
        }
    };
}

impl_primitive!(String, String);
impl_primitive!(u8, U8);
impl_primitive!(u16, U16);
impl_primitive!(u32, U32);
impl_primitive!(u64, U64);
impl_primitive!(u128, U128);
impl_primitive!(i8, I8);
impl_primitive!(i16, I16);
impl_primitive!(i32, I32);
impl_primitive!(i64, I64);
impl_primitive!(i128, I128);
impl_primitive!(f32, F32);
impl_primitive!(f64, F64);

unsafe impl<T: HasSchema + 'static> HasSchema for Vec<T> {
    fn schema() -> &'static Schema {
        static STORE: OnceLock<RwLock<HashMap<TypeId, &'static Schema>>> = OnceLock::new();
        let store = STORE.get_or_init(Default::default);
        let read = store.read();
        let type_id = TypeId::of::<Self>();

        if let Some(schema) = read.get(&type_id) {
            schema
        } else {
            drop(read);
            let kind = SchemaKind::Vec(T::schema().into());
            let schema = Schema {
                kind,
                type_id: Some(type_id),
                type_data: Default::default(),
            };
            let schema: &'static Schema = Box::leak(Box::new(schema));
            let mut write = store.write();
            write.insert(type_id, schema);
            schema
        }
    }
}

#[cfg(feature = "glam")]
mod impl_glam {
    use super::*;
    use glam::*;

    macro_rules! impl_glam {
        ($t:ty, $prim:ident, $($field:ident),+) => {
            unsafe impl HasSchema for $t {
                fn schema() -> &'static Schema {
                    static S: OnceLock<Schema> = OnceLock::new();

                    S.get_or_init(|| {
                        let type_id = Some(TypeId::of::<Self>());
                        let kind = SchemaKind::Struct(StructSchema {
                            fields: vec![
                                $(
                                    StructField {
                                        name: Some(stringify!($field).to_owned()),
                                        schema: Schema {
                                            kind: SchemaKind::Primitive(Primitive::$prim),
                                            type_id: Some(TypeId::of::<$t>()),
                                            type_data: Default::default(),
                                        }
                                    }
                                ),*
                            ],
                        });
                        Schema {
                            type_id,
                            kind,
                            type_data: Default::default(),
                        }
                    })
                }
            }
        };
    }

    macro_rules! impl_glam_vecs {
        ($prim:ident, $id:ident) => {
            paste::paste! {
                impl_glam!( [< $id 2 >], $prim, x, y);
                impl_glam!( [< $id 3 >], $prim, x, y, z);
                impl_glam!( [< $id 4 >], $prim, x, y, z, w);
            }
        };
    }

    impl_glam_vecs!(Bool, BVec);
    impl_glam_vecs!(U32, UVec);
    impl_glam_vecs!(I32, IVec);
    impl_glam_vecs!(F32, Vec);
    impl_glam_vecs!(F64, DVec);

    // TODO: matrix types.
}
