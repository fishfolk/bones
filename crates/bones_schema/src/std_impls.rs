use bones_utils::HashMap;

use super::*;

use std::{
    any::TypeId,
    sync::{OnceLock, RwLock},
};

macro_rules! impl_primitive {
    ($t:ty, $prim:ident) => {
        unsafe impl HasSchema for $t {
            fn schema() -> &'static Schema {
                static S: OnceLock<&'static Schema> = OnceLock::new();
                S.get_or_init(|| {
                    SCHEMA_REGISTRY.register(SchemaData {
                        kind: SchemaKind::Primitive(Primitive::$prim),
                        type_id: Some(TypeId::of::<$t>()),
                        clone_fn: Some(<$t as RawClone>::raw_clone),
                        drop_fn: Some(<$t as RawDrop>::raw_drop),
                        default_fn: Some(<$t as RawDefault>::raw_default),
                        type_data: Default::default(),
                    })
                })
            }
        }
    };
}

impl_primitive!(String, String);
impl_primitive!(bool, Bool);
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

unsafe impl HasSchema for usize {
    fn schema() -> &'static Schema {
        static S: OnceLock<&'static Schema> = OnceLock::new();
        S.get_or_init(|| {
            SCHEMA_REGISTRY.register(SchemaData {
                kind: SchemaKind::Primitive({
                    #[cfg(target_pointer_width = "32")]
                    let p = Primitive::U32;
                    #[cfg(target_pointer_width = "64")]
                    let p = Primitive::U64;
                    p
                }),
                type_id: Some(TypeId::of::<usize>()),
                clone_fn: Some(<usize as RawClone>::raw_clone),
                drop_fn: Some(<usize as RawDrop>::raw_drop),
                default_fn: Some(<usize as RawDefault>::raw_default),
                type_data: Default::default(),
            })
        })
    }
}
unsafe impl HasSchema for isize {
    fn schema() -> &'static Schema {
        static S: OnceLock<&'static Schema> = OnceLock::new();
        S.get_or_init(|| {
            SCHEMA_REGISTRY.register(SchemaData {
                kind: SchemaKind::Primitive({
                    #[cfg(target_pointer_width = "32")]
                    let p = Primitive::I32;
                    #[cfg(target_pointer_width = "64")]
                    let p = Primitive::I64;
                    p
                }),
                type_id: Some(TypeId::of::<usize>()),
                clone_fn: Some(<usize as RawClone>::raw_clone),
                drop_fn: Some(<usize as RawDrop>::raw_drop),
                default_fn: Some(<usize as RawDefault>::raw_default),
                type_data: Default::default(),
            })
        })
    }
}

// TODO: since we don't have specialization, we have to require this `Clone` bound. :/ :/ :/ Figure
// out if there is some tricky way we can get around this limitation, and have a separate impl for
// `Vec<T: Clone>` and `Vec<T: !Clone>`.
unsafe impl<T: Clone + HasSchema + 'static> HasSchema for Vec<T> {
    fn schema() -> &'static Schema {
        static STORE: OnceLock<RwLock<HashMap<TypeId, &'static Schema>>> = OnceLock::new();
        let store = STORE.get_or_init(Default::default);
        let read = store.read().unwrap();
        let type_id = TypeId::of::<Self>();

        if let Some(schema) = read.get(&type_id) {
            schema
        } else {
            drop(read);
            let kind = SchemaKind::Vec(T::schema());
            let schema = SCHEMA_REGISTRY.register(SchemaData {
                kind,
                type_data: Default::default(),
                type_id: Some(type_id),
                clone_fn: Some(<Self as RawClone>::raw_clone),
                drop_fn: Some(<Self as RawDrop>::raw_drop),
                default_fn: Some(<Self as RawDefault>::raw_default),
            });
            let mut write = store.write().unwrap();
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
                    static S: OnceLock<&'static Schema> = OnceLock::new();

                    S.get_or_init(|| {
                        let type_id = Some(TypeId::of::<Self>());
                        let kind = SchemaKind::Struct(StructSchema {
                            fields: vec![
                                $(
                                    StructField {
                                        name: Some(stringify!($field).into()),
                                        schema: SCHEMA_REGISTRY.register(SchemaData {
                                            kind: SchemaKind::Primitive(Primitive::$prim),
                                            type_id: Some(TypeId::of::<$t>()),
                                            type_data: Default::default(),
                                            clone_fn: Some(<Self as RawClone>::raw_clone),
                                            drop_fn: Some(<Self as RawDrop>::raw_drop),
                                            default_fn: Some(<Self as RawDefault>::raw_default),
                                        })
                                    }
                                ),*
                            ],
                        });
                        SCHEMA_REGISTRY.register(SchemaData {
                            type_id,
                            kind,
                            type_data: Default::default(),
                            clone_fn: Some(<Self as RawClone>::raw_clone),
                            drop_fn: Some(<Self as RawDrop>::raw_drop),
                            default_fn: Some(<Self as RawDefault>::raw_default),
                        })
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
    impl_glam!(Quat, F32, x, y, z, w);

    // TODO: matrix types.
}
