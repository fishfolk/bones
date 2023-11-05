#[cfg(feature = "serde")]
use crate::ser_de::SchemaDeserialize;
use bones_utils::{fxhash::FxHasher, Ustr};
#[cfg(feature = "serde")]
use serde::{de::Error, Deserialize};

use std::ffi::c_void;

use crate::{alloc::TypeDatas, prelude::*, raw_fns::*};

use std::{alloc::Layout, any::TypeId, hash::Hasher, sync::OnceLock, time::Duration};

macro_rules! impl_primitive {
    ($t:ty, $prim:ident) => {
        unsafe impl HasSchema for $t {
            fn schema() -> &'static Schema {
                static S: OnceLock<&'static Schema> = OnceLock::new();
                S.get_or_init(|| {
                    SCHEMA_REGISTRY.register(SchemaData {
                        name: stringify!($t).into(),
                        full_name: concat!("std::", stringify!($t)).into(),
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
                        name: stringify!($t).into(),
                        full_name: concat!("std::", stringify!($t)).into(),
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
                name: "usize".into(),
                full_name: "std::usize".into(),
                kind: SchemaKind::Primitive({
                    #[cfg(target_pointer_width = "32")]
                    let p = Primitive::U32;
                    #[cfg(target_pointer_width = "64")]
                    let p = Primitive::U64;
                    p
                }),
                type_id: Some(TypeId::of::<usize>()),
                clone_fn: Some(<Self as RawClone>::raw_clone),
                drop_fn: Some(<Self as RawDrop>::raw_drop),
                default_fn: Some(<Self as RawDefault>::raw_default),
                hash_fn: Some(<Self as RawHash>::raw_hash),
                eq_fn: Some(<Self as RawEq>::raw_eq),
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
                name: "isize".into(),
                full_name: "std::isize".into(),
                kind: SchemaKind::Primitive({
                    #[cfg(target_pointer_width = "32")]
                    let p = Primitive::I32;
                    #[cfg(target_pointer_width = "64")]
                    let p = Primitive::I64;
                    p
                }),
                type_id: Some(TypeId::of::<Self>()),
                clone_fn: Some(<Self as RawClone>::raw_clone),
                drop_fn: Some(<Self as RawDrop>::raw_drop),
                default_fn: Some(<Self as RawDefault>::raw_default),
                hash_fn: Some(<Self as RawHash>::raw_hash),
                eq_fn: Some(<Self as RawEq>::raw_eq),
                type_data: Default::default(),
            })
        })
    }
}

unsafe impl HasSchema for Ustr {
    fn schema() -> &'static Schema {
        static S: OnceLock<&'static Schema> = OnceLock::new();
        let layout = Layout::new::<Self>();
        S.get_or_init(|| {
            SCHEMA_REGISTRY.register(SchemaData {
                name: "Ustr".into(),
                full_name: "ustr::Ustr".into(),
                kind: SchemaKind::Primitive(Primitive::Opaque {
                    size: layout.size(),
                    align: layout.align(),
                }),
                type_id: Some(TypeId::of::<Self>()),
                clone_fn: Some(<Self as RawClone>::raw_clone),
                drop_fn: Some(<Self as RawDrop>::raw_drop),
                default_fn: Some(<Self as RawDefault>::raw_default),
                hash_fn: Some(<Self as RawHash>::raw_hash),
                eq_fn: Some(<Self as RawEq>::raw_eq),
                type_data: {
                    let td = TypeDatas::default();
                    #[cfg(feature = "serde")]
                    td.insert(SchemaDeserialize {
                        deserialize_fn: |reference, deserializer| {
                            Self::schema()
                                .ensure_match(reference.schema())
                                .map_err(|e| erased_serde::Error::custom(e.to_string()))?;

                            let s = String::deserialize(deserializer)?;
                            let us = Ustr::from(&s);
                            *reference.cast_into_mut() = us;

                            Ok(())
                        },
                    })
                    .unwrap();
                    td
                },
            })
        })
    }
}

unsafe impl HasSchema for Duration {
    fn schema() -> &'static Schema {
        static S: OnceLock<&'static Schema> = OnceLock::new();
        let layout = Layout::new::<Self>();
        S.get_or_init(|| {
            SCHEMA_REGISTRY.register(SchemaData {
                name: "Duration".into(),
                full_name: "std::Duration".into(),
                kind: SchemaKind::Primitive(Primitive::Opaque {
                    size: layout.size(),
                    align: layout.align(),
                }),
                type_id: Some(TypeId::of::<Self>()),
                clone_fn: Some(<Self as RawClone>::raw_clone),
                drop_fn: Some(<Self as RawDrop>::raw_drop),
                default_fn: Some(<Self as RawDefault>::raw_default),
                hash_fn: Some(<Self as RawHash>::raw_hash),
                eq_fn: Some(<Self as RawEq>::raw_eq),
                type_data: {
                    let td = TypeDatas::default();
                    #[cfg(feature = "serde")]
                    td.insert(SchemaDeserialize {
                        deserialize_fn: |reference, deserializer| {
                            Self::schema()
                                .ensure_match(reference.schema())
                                .map_err(|e| erased_serde::Error::custom(e.to_string()))?;

                            #[cfg(feature = "humantime")]
                            {
                                let s = String::deserialize(deserializer)?;
                                let d: Duration = s
                                    .parse::<humantime::Duration>()
                                    .map_err(|e| erased_serde::Error::custom(e.to_string()))?
                                    .into();
                                *reference.cast_into_mut() = d;
                            }

                            #[cfg(not(feature = "humantime"))]
                            {
                                let d = Duration::deserialize(deserializer)?;
                                *reference.cast_into_mut() = d;
                            }

                            Ok(())
                        },
                    })
                    .unwrap();
                    td
                },
            })
        })
    }
}

#[cfg(feature = "glam")]
mod impl_glam {
    use super::*;
    use glam::*;

    unsafe impl HasSchema for Quat {
        fn schema() -> &'static Schema {
            static S: OnceLock<&'static Schema> = OnceLock::new();
            let layout = std::alloc::Layout::new::<Quat>();
            S.get_or_init(|| {
                SCHEMA_REGISTRY.register(SchemaData {
                    name: "Quat".into(),
                    full_name: "glam::Quat".into(),
                    kind: SchemaKind::Primitive(Primitive::Opaque {
                        size: layout.size(),
                        align: layout.align(),
                    }),
                    type_id: Some(TypeId::of::<usize>()),
                    clone_fn: Some(<Self as RawClone>::raw_clone),
                    drop_fn: Some(<Self as RawDrop>::raw_drop),
                    default_fn: Some(<Self as RawDefault>::raw_default),
                    // TODO: Get the schema `hash_fn` and `eq_fn` for the `Quat` type.
                    // Quats don't implement hash and eq by default because of floating point number
                    // issues, so we'll have to use a workaround like `CustomRawFns` below to create
                    // valid implementations of Hash and Eq over the floating points inside the
                    // Quat.
                    hash_fn: None,
                    eq_fn: None,
                    type_data: Default::default(),
                })
            })
        }
    }

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
                            name: stringify!($t).into(),
                            full_name: concat!("glam::", stringify!($t)).into(),
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

    // TODO: Implement `HasSchema` for glam matrix types.
    // We need to implement `HasSchema` for the matrix types, just like we did with the vector
    // types.

    macro_rules! custom_fns_impl_bvec {
        ($ty:ident) => {
            impl CustomRawFns for glam::$ty {
                unsafe extern "C-unwind" fn raw_hash(ptr: *const c_void) -> u64 {
                    <Self as RawHash>::raw_hash(ptr)
                }
                unsafe extern "C-unwind" fn raw_eq(a: *const c_void, b: *const c_void) -> bool {
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
                unsafe extern "C-unwind" fn raw_hash(ptr: *const c_void) -> u64 {
                    let this = unsafe { &*(ptr as *const Self) };
                    let mut hasher = FxHasher::default();
                    $(
                        hasher.write_u64($prim::raw_hash(&this.$field as *const $prim as *const c_void));
                    )+
                    hasher.finish()
                }

                unsafe extern "C-unwind" fn raw_eq(a: *const c_void, b: *const c_void) -> bool {
                    let a = unsafe { &*(a as *const Self) };
                    let b = unsafe { &*(b as *const Self) };

                    $(
                        $prim::raw_eq(
                            &a.$field as *const $prim as *const c_void,
                            &b.$field as *const $prim as *const c_void,
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
    unsafe extern "C-unwind" fn raw_hash(ptr: *const c_void) -> u64;
    unsafe extern "C-unwind" fn raw_eq(a: *const c_void, b: *const c_void) -> bool;
}

macro_rules! custom_fns_impl_float {
    ($ty:ident) => {
        impl CustomRawFns for $ty {
            unsafe extern "C-unwind" fn raw_hash(ptr: *const c_void) -> u64 {
                let this = unsafe { &*(ptr as *const Self) };

                let mut hasher = FxHasher::default();
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

            unsafe extern "C-unwind" fn raw_eq(a: *const c_void, b: *const c_void) -> bool {
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
