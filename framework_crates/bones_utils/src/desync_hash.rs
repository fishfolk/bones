//! [`DesyncHash`] trait and impls, for detecting net desync.
//!
//! In order to use [`DesyncHash`] with [`glam`] types, the "glam" feature flag must be used.

use std::time::Duration;

use ustr::Ustr;

/// [`DesyncHash`] is used to hash type and compare over network to detect desyncs.
///
/// In order to opt in a `HasSchema` Component or Resource to be included in hash of World in networked session,
/// `#[net]` or `#[derive_type_data(SchemaDesyncHas)]` must also be included.
pub trait DesyncHash {
    /// Update hasher from type's values
    fn hash(&self, hasher: &mut dyn std::hash::Hasher);
}

/// Extension of [`DesyncHash`] that is automatically implemented for `T: DesyncHash`.
/// Adds helper to compute standalone hash instead of updating a hasher.
pub trait DesyncHashImpl {
    /// Compute hash of type with provided hasher.
    fn compute_hash<H: std::hash::Hasher + Default>(&self) -> u64;
}

impl<T: DesyncHash> DesyncHashImpl for T {
    fn compute_hash<H: std::hash::Hasher + Default>(&self) -> u64 {
        let mut hasher = H::default();
        self.hash(&mut hasher);
        hasher.finish()
    }
}

/// Tree of desync hashes
pub trait DesyncTree<V>: Clone {
    type Node;

    fn get_hash(&self) -> V;

    fn name(&self) -> &Option<String>;

    fn from_root(root: Self::Node) -> Self;
}

/// [`DesyncTree`] node trait, built from children and hash. A node is effectively a sub-tree,
/// as we build the tree bottom-up.
pub trait DesyncTreeNode<V>: Clone + PartialEq + Eq {
    fn new(hash: u64, name: Option<String>, children: Vec<DefaultDesyncTreeNode>) -> Self;

    fn get_hash(&self) -> V;
}

/// Implement to allow type to create a [`DesyncTreeNode`] containing hash built from children.
pub trait BuildDesyncNode<N, V>
where
    N: DesyncTreeNode<V>,
{
    fn desync_tree_node<H: std::hash::Hasher + Default>(&self) -> N;
}

/// Default impl for [`DesyncTreeNode`].
#[derive(Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DefaultDesyncTreeNode {
    name: Option<String>,
    hash: u64,
    children: Vec<DefaultDesyncTreeNode>,
}

impl PartialOrd for DefaultDesyncTreeNode {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DefaultDesyncTreeNode {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.hash.cmp(&other.hash)
    }
}

impl DesyncTreeNode<u64> for DefaultDesyncTreeNode {
    fn new(hash: u64, name: Option<String>, children: Vec<DefaultDesyncTreeNode>) -> Self {
        Self {
            name,
            hash,
            children,
        }
    }

    fn get_hash(&self) -> u64 {
        self.hash
    }
}

/// Tree of desync hashes, allows storing hash of world and children such as components and resources.
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DefaultDesyncTree {
    root: DefaultDesyncTreeNode,
}

impl From<DefaultDesyncTreeNode> for DefaultDesyncTree {
    fn from(value: DefaultDesyncTreeNode) -> Self {
        Self::from_root(value)
    }
}

impl DesyncTree<u64> for DefaultDesyncTree {
    type Node = DefaultDesyncTreeNode;

    fn get_hash(&self) -> u64 {
        self.root.get_hash()
    }

    fn name(&self) -> &Option<String> {
        &self.root.name
    }

    fn from_root(root: Self::Node) -> Self {
        Self { root }
    }
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
