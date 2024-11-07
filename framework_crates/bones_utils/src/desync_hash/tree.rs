//! Implementation of [`DesyncTree`] trait for hash tree in desync detection.

use std::slice::Iter;

pub use tree_iterators_rs::prelude::BorrowedTreeNode;

/// Tree of desync hashes
pub trait DesyncTree: Clone {
    /// Node type
    type Node;

    /// Get root hash of tree
    fn get_hash(&self) -> Option<u64>;

    /// Get root node
    fn root(&self) -> &Self::Node;

    /// make tree from root node
    fn from_root(root: Self::Node) -> Self;
}

/// [`DesyncTree`] node trait, built from children and hash. A node is effectively a sub-tree,
/// as we build the tree bottom-up.
pub trait DesyncTreeNode: Clone {
    /// Get node hash
    fn get_hash(&self) -> Option<u64>;

    ///  Get children
    fn children(&self) -> &Vec<Self>;

    /// Get children mut
    fn children_mut(&mut self) -> &mut Vec<Self>;
}

/// Implement to allow type to create a [`DesyncTreeNode`] containing hash built from children.
pub trait BuildDesyncNode {
    /// `include_unhashable` sets whether components or resources be included as non-contributing nodes
    /// in tree, to see what could be opted-in.
    fn desync_tree_node<H: std::hash::Hasher + Default>(
        &self,
        include_unhashable: bool,
    ) -> DefaultDesyncTreeNode;
}

/// Metadata optionally included with ['DesyncTreeNode`].
#[derive(Copy, Clone, Default)]
pub enum DesyncNodeMetadata {
    /// No additional metadata
    #[default]
    None,
    /// Node is a component
    Component {
        /// Entity idx of component
        entity_idx: u32,
    },
}

/// Default impl for [`DesyncTreeNode`].
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DefaultDesyncTreeNode {
    name: Option<String>,
    hash: Option<u64>,
    children: Vec<Self>,

    /// Some userdata that can be included in node.
    #[cfg_attr(feature = "serde", serde(skip))]
    metadata: DesyncNodeMetadata,
}

impl DefaultDesyncTreeNode {
    /// Create new node
    pub fn new(
        hash: Option<u64>,
        name: Option<String>,
        children: Vec<Self>,
        metadata: DesyncNodeMetadata,
    ) -> Self {
        Self {
            name,
            hash,
            children,
            metadata,
        }
    }

    /// Get node metadata
    pub fn metadata(&self) -> &DesyncNodeMetadata {
        &self.metadata
    }

    /// Name of node
    pub fn name(&self) -> &Option<String> {
        &self.name
    }

    /// Set the name of node
    pub fn set_name(&mut self, name: String) {
        self.name = Some(name);
    }

    /// Get node hash
    pub fn get_hash(&self) -> Option<u64> {
        self.hash
    }

    /// Get children
    pub fn children(&self) -> &Vec<Self> {
        &self.children
    }

    /// Get children mut
    pub fn children_mut(&mut self) -> &mut Vec<Self> {
        &mut self.children
    }
}

impl PartialEq for DefaultDesyncTreeNode {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}

impl Eq for DefaultDesyncTreeNode {}

impl PartialOrd for DefaultDesyncTreeNode {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.hash.cmp(&other.hash))
    }
}

impl Ord for DefaultDesyncTreeNode {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.hash.cmp(&other.hash)
    }
}

/// Auto impl support for iterating over tree
impl<'a> BorrowedTreeNode<'a> for DefaultDesyncTreeNode {
    type BorrowedValue = &'a Self;

    type BorrowedChildren = Iter<'a, DefaultDesyncTreeNode>;

    fn get_value_and_children_iter(
        &'a self,
    ) -> (Self::BorrowedValue, Option<Self::BorrowedChildren>) {
        if self.children.is_empty() {
            return (self, None);
        }

        (self, Some(self.children.iter()))
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

impl DesyncTree for DefaultDesyncTree {
    type Node = DefaultDesyncTreeNode;

    fn get_hash(&self) -> Option<u64> {
        self.root.get_hash()
    }

    fn root(&self) -> &Self::Node {
        &self.root
    }

    fn from_root(root: Self::Node) -> Self {
        Self { root }
    }
}
