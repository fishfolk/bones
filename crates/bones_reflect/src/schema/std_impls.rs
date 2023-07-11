use super::*;

macro_rules! impl_primitive {
    ($t:ty, $prim:ident) => {
        unsafe impl HasSchema for $t {
            fn schema() -> Schema {
                Schema::Primitive(Primitive::$prim)
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

unsafe impl<T: HasSchema> HasSchema for Vec<T> {
    fn schema() -> Schema {
        Schema::Vec(Box::new(T::schema()))
    }
}

#[cfg(feature = "glam")]
mod impl_glam {
    use super::*;
    use glam::*;

    macro_rules! impl_glam {
        ($t:ty, $prim:ident, $($field:ident),+) => {
            unsafe impl HasSchema for $t {
                fn schema() -> Schema {
                    Schema::Struct(StructSchema {
                        fields: vec![
                            $(
                                StructField {
                                    name: Some(stringify!($field).to_owned()),
                                    schema: Schema::Primitive(Primitive::$prim),
                                }
                            ),*
                        ],
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
