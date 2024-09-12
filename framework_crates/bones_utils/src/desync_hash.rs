//! [`DesyncHash`] trait and impls, for detecting net desync.
//
// In order to use [`DesyncHash`] with [`glam`] types, the "glam" feature flag must be used.

use std::time::Duration;

use ustr::Ustr;

/// [`DesyncHash`] is used to hash type and compare over network to detect desyncs.
///
/// In order to opt in a [`HasSchema`] Component or Resource to be included in hash of World in networked session,
/// `#[net]` or `#[derive_type_data(SchemaDesyncHas)]` must also be included.
pub trait DesyncHash {
    /// Update hasher from type's values
    fn hash(&self, hasher: &mut dyn std::hash::Hasher);
}

impl DesyncHash for Duration {
    fn hash(&self, hasher: &mut dyn std::hash::Hasher) {
        self.as_nanos().hash(hasher);
    }
}

impl DesyncHash for () {
    fn hash(&self, _hasher: &mut dyn std::hash::Hasher) {}
}

impl DesyncHash for bool {
    fn hash(&self, hasher: &mut dyn std::hash::Hasher) {
        hasher.write_u8(*self as u8)
    }
}

impl<T: DesyncHash> DesyncHash for Vec<T> {
    fn hash(&self, hasher: &mut dyn std::hash::Hasher) {
        for value in self {
            value.hash(hasher);
        }
    }
}
macro_rules! desync_hash_impl_int {
    ($ty:ident) => {
        impl DesyncHash for $ty {
            ::paste::paste! {
                fn hash(&self, hasher: &mut dyn std::hash::Hasher) {
                        hasher.[<write_ $ty>](*self);
                }
            }
        }
    };
}

macro_rules! desync_hash_impl_float {
    ($ty:ident) => {
        impl DesyncHash for $ty {
            fn hash(&self, hasher: &mut dyn std::hash::Hasher) {
                if self.is_nan() {
                    // Ensure all NaN representations hash to the same value
                    hasher.write(&Self::to_ne_bytes(Self::NAN));
                } else if *self == 0.0 {
                    // Ensure both zeroes hash to the same value
                    hasher.write(&Self::to_ne_bytes(0.0));
                } else {
                    hasher.write(&Self::to_ne_bytes(*self));
                }
            }
        }
    };
}

macro_rules! desync_hash_impl_as_bytes {
    ($ty:ident) => {
        impl DesyncHash for $ty {
            fn hash(&self, hasher: &mut dyn std::hash::Hasher) {
                hasher.write(self.as_bytes());
            }
        }
    };
}

desync_hash_impl_float!(f32);
desync_hash_impl_float!(f64);

desync_hash_impl_int!(i8);
desync_hash_impl_int!(i16);
desync_hash_impl_int!(i32);
desync_hash_impl_int!(i64);
desync_hash_impl_int!(i128);
desync_hash_impl_int!(isize);
desync_hash_impl_int!(u8);
desync_hash_impl_int!(u16);
desync_hash_impl_int!(u32);
desync_hash_impl_int!(u64);
desync_hash_impl_int!(u128);
desync_hash_impl_int!(usize);

desync_hash_impl_as_bytes!(String);
desync_hash_impl_as_bytes!(str);
desync_hash_impl_as_bytes!(Ustr);

#[cfg(feature = "glam")]
mod impl_glam {
    use glam::*;

    use super::DesyncHash;

    macro_rules! desync_hash_impl_glam_vecs {
        ($id:ident) => {
            paste::paste! {
                desync_hash_impl_glam!( [< $id 2 >], x, y);
                desync_hash_impl_glam!( [< $id 3 >], x, y, z);
                desync_hash_impl_glam!( [< $id 4 >], x, y, z, w);
            }
        };
    }

    macro_rules! desync_hash_impl_glam {
        ($t:ty, $($field:ident),+) => {
            impl DesyncHash for $t {
                fn hash(&self, hasher: &mut dyn std::hash::Hasher) {
                     // $(self.$field.hash(hasher);)*
                     // $(hasher.(self.$field);)*
                     $(DesyncHash::hash(&self.$field, hasher);)*
                }
            }
        };
    }

    desync_hash_impl_glam_vecs!(BVec);
    desync_hash_impl_glam_vecs!(UVec);
    desync_hash_impl_glam_vecs!(IVec);
    desync_hash_impl_glam_vecs!(Vec);
    desync_hash_impl_glam_vecs!(DVec);

    impl DesyncHash for Quat {
        fn hash(&self, hasher: &mut dyn std::hash::Hasher) {
            self.x.hash(hasher);
            self.y.hash(hasher);
            self.z.hash(hasher);
            self.w.hash(hasher);
        }
    }
}
