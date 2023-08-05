use bones_utils::ahash::AHasher;

use crate::{prelude::*, raw_fns::*};

use std::{any::TypeId, hash::Hasher, sync::OnceLock};

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
                        hash_fn: Some(<$t as RawHash>::raw_hash),
                        eq_fn: Some(<$t as RawEq>::raw_eq),
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

macro_rules! schema_impl_float {
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
                        hash_fn: Some(<$t as CustomRawFns>::raw_hash),
                        eq_fn: Some(<$t as CustomRawFns>::raw_eq),
                        type_data: Default::default(),
                    })
                })
            }
        }
    };
}

schema_impl_float!(f32, F32);
schema_impl_float!(f64, F64);

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
                hash_fn: Some(<usize as RawHash>::raw_hash),
                eq_fn: Some(<usize as RawEq>::raw_eq),
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
                hash_fn: Some(<isize as RawHash>::raw_hash),
                eq_fn: Some(<isize as RawEq>::raw_eq),
                type_data: Default::default(),
            })
        })
    }
}

#[cfg(feature = "glam")]
mod impl_glam {
    use super::*;
    use glam::*;

    macro_rules! schema_impl_glam {
        ($t:ty, $prim:ident, $nprim:ident, $($field:ident),+) => {
            unsafe impl HasSchema for $t {
                fn schema() -> &'static Schema {
                    static S: OnceLock<&'static Schema> = OnceLock::new();

                    S.get_or_init(|| {
                        let type_id = Some(TypeId::of::<Self>());
                        let kind = SchemaKind::Struct(StructSchemaInfo {
                            fields: vec![
                                $(
                                    StructFieldInfo {
                                        name: Some(stringify!($field).into()),
                                        schema: $nprim::schema(),
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
                            hash_fn: Some(<Self as CustomRawFns>::raw_hash),
                            eq_fn: Some(<Self as CustomRawFns>::raw_eq),
                        })
                    })
                }
            }
        };
    }

    macro_rules! schema_impl_glam_vecs {
        ($prim:ident, $nprim:ident, $id:ident) => {
            paste::paste! {
                schema_impl_glam!( [< $id 2 >], $prim, $nprim, x, y);
                schema_impl_glam!( [< $id 3 >], $prim, $nprim, x, y, z);
                schema_impl_glam!( [< $id 4 >], $prim, $nprim, x, y, z, w);
            }
        };
    }

    schema_impl_glam_vecs!(Bool, bool, BVec);
    schema_impl_glam_vecs!(U32, u32, UVec);
    schema_impl_glam_vecs!(I32, i32, IVec);
    schema_impl_glam_vecs!(F32, f32, Vec);
    schema_impl_glam_vecs!(F64, f64, DVec);
    schema_impl_glam!(Quat, F32, f32, x, y, z, w);

    // TODO: matrix types.

    macro_rules! custom_fns_impl_bvec {
        ($ty:ident) => {
            impl CustomRawFns for glam::$ty {
                unsafe extern "C-unwind" fn raw_hash(ptr: *const u8) -> u64 {
                    <Self as RawHash>::raw_hash(ptr)
                }
                unsafe extern "C-unwind" fn raw_eq(a: *const u8, b: *const u8) -> bool {
                    <Self as RawEq>::raw_eq(a, b)
                }
            }
        };
    }
    custom_fns_impl_bvec!(BVec2);
    custom_fns_impl_bvec!(BVec3);
    custom_fns_impl_bvec!(BVec4);

    macro_rules! custom_fns_impl_glam {
        ($t:ty, $prim:ident, $($field:ident),+) => {
            impl CustomRawFns for $t {
                unsafe extern "C-unwind" fn raw_hash(ptr: *const u8) -> u64 {
                    let this = unsafe { &*(ptr as *const Self) };
                    let mut hasher = AHasher::default();
                    $(
                        hasher.write_u64($prim::raw_hash(&this.$field as *const $prim as *const u8));
                    )+
                    hasher.finish()
                }

                unsafe extern "C-unwind" fn raw_eq(a: *const u8, b: *const u8) -> bool {
                    let a = unsafe { &*(a as *const Self) };
                    let b = unsafe { &*(b as *const Self) };

                    $(
                        $prim::raw_eq(
                            &a.$field as *const $prim as *const u8,
                            &b.$field as *const $prim as *const u8,
                        )
                    )&&+
                }
            }
        };
    }
    custom_fns_impl_glam!(Vec2, f32, x, y);
    custom_fns_impl_glam!(Vec3, f32, x, y, z);
    custom_fns_impl_glam!(Vec4, f32, x, y, z, w);
    custom_fns_impl_glam!(DVec2, f64, x, y);
    custom_fns_impl_glam!(DVec3, f64, x, y, z);
    custom_fns_impl_glam!(DVec4, f64, x, y, z, w);
    custom_fns_impl_glam!(UVec2, u32, x, y);
    custom_fns_impl_glam!(UVec3, u32, x, y, z);
    custom_fns_impl_glam!(UVec4, u32, x, y, z, w);
    custom_fns_impl_glam!(IVec2, i32, x, y);
    custom_fns_impl_glam!(IVec3, i32, x, y, z);
    custom_fns_impl_glam!(IVec4, i32, x, y, z, w);
    custom_fns_impl_glam!(Quat, f32, x, y, z, w);
}

/// Trait for types that require specific implementations of eq and hash fns, for use in this module only.
trait CustomRawFns {
    unsafe extern "C-unwind" fn raw_hash(ptr: *const u8) -> u64;
    unsafe extern "C-unwind" fn raw_eq(a: *const u8, b: *const u8) -> bool;
}

macro_rules! custom_fns_impl_float {
    ($ty:ident) => {
        impl CustomRawFns for $ty {
            unsafe extern "C-unwind" fn raw_hash(ptr: *const u8) -> u64 {
                let this = unsafe { &*(ptr as *const Self) };

                let mut hasher = AHasher::default();
                if this.is_nan() {
                    // Ensure all NaN representations hash to the same value
                    hasher.write(&$ty::to_ne_bytes($ty::NAN));
                } else if *this == 0.0 {
                    // Ensure both zeroes hash to the same value
                    hasher.write(&$ty::to_ne_bytes(0.0));
                } else {
                    hasher.write(&$ty::to_ne_bytes(*this));
                }
                hasher.finish()
            }

            unsafe extern "C-unwind" fn raw_eq(a: *const u8, b: *const u8) -> bool {
                let a = unsafe { &*(a as *const Self) };
                let b = unsafe { &*(b as *const Self) };
                if a.is_nan() && a.is_nan() {
                    true
                } else {
                    a == b
                }
            }
        }
    };
}
custom_fns_impl_float!(f32);
custom_fns_impl_float!(f64);
