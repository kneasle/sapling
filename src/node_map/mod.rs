//! A module to house the traits and implementations for `NodeMap`s.

pub mod vec;

use crate::ast_spec::ASTSpec;

// Imports used solely for doc-comments
#[allow(unused_imports)]
use crate::editable_tree::EditableTree;

/// A trait bound that specifies what types can be used as a reference to a Node in an [`NodeMap`]
pub trait Reference: Copy + Eq + std::fmt::Debug + std::hash::Hash {}

/// A trait bound for a type that can be used to access nodes (used to give [`NodeMap`]-like
/// attributes to [`EditableTree`]s).  If you need to be able to change nodes, see [`NodeMapMut`].
pub trait NodeMap<Ref: Reference, Node: ASTSpec<Ref>> {
    /// Gets node from a reference, returning [`None`] if the reference is invalid.
    fn get_node(&self, id: Ref) -> Option<&Node>;

    /// Get the reference of the root node of the tree.  This is required to be a valid reference,
    /// i.e. `self.get_node(self.root())` should never return [`None`].
    fn root(&self) -> Ref;

    /// Get the node that is the root of the current tree
    fn root_node(&self) -> &Node {
        // We can unwrap here, because self.root() is required to be a valid reference.
        self.get_node(self.root()).unwrap()
    }
}

/// A trait bound for a type that can store `Node`s, accessible by references.
pub trait NodeMapMut<Ref: Reference, Node: ASTSpec<Ref>>: NodeMap<Ref, Node> {
    /// Create a new `NodeMap` with a given `Node` as root
    fn with_root(root: Node) -> Self;

    /// Create a new `NodeMap` containing only the default node as root
    fn with_default_root() -> Self
    where
        Self: Sized,
    {
        Self::with_root(Node::default())
    }

    /// Set the root of the tree to be the node at a given reference, returning `true` if the
    /// reference was valid.  If the reference was invalid, the root will not be replaced and
    /// `false` will be returned.
    fn set_root(&mut self, new_root: Ref) -> bool;

    /// Adds a new node and set it to the tree's root
    fn add_as_root(&mut self, new_root_node: Node) -> Ref {
        let r = self.add_node(new_root_node);
        let is_valid = self.set_root(r);
        debug_assert!(is_valid);
        r
    }

    /// Writes the text rendering of the root node to a string (same as calling
    /// [`to_text`](ASTSpec::to_text) on the [`root`](NodeMap::root)).
    fn write_text(&self, string: &mut String, format_style: &Node::FormatStyle)
    where
        Self: Sized,
    {
        match self.get_node(self.root()) {
            Some(root) => {
                root.write_text(self, string, format_style);
            }
            None => {
                string.push_str("<INVALID ROOT NODE>");
            }
        }
    }

    /// Generates the text rendering of the root node (same as calling [`to_text`](ASTSpec::to_text)
    /// on the [`root`](NodeMap::root)).
    fn to_text(&self, format_style: &Node::FormatStyle) -> String
    where
        Self: Sized,
    {
        match self.get_node(self.root()) {
            Some(root) => root.to_text(self, format_style),
            None => "<INVALID ROOT NODE>".to_string(),
        }
    }

    /// Gets mutable node from a reference, returning [`None`] if the reference is invalid.
    fn get_node_mut(&mut self, id: Ref) -> Option<&mut Node>;

    /// Get the node that is the root of the current tree
    fn root_node_mut(&mut self) -> &mut Node {
        // We can unwrap here, because self.root() is required to be a valid reference.
        self.get_node_mut(self.root()).unwrap()
    }

    /// Add a new `Node` to the tree, and return its reference
    fn add_node(&mut self, node: Node) -> Ref;

    /// Overwrite a node currently in the tree with another one.  Returns 'true' if `id` points to
    /// an existing node, if not it will return 'false' and not do the subsitution.
    fn overwrite_node(&mut self, id: Ref, node: Node) -> bool;
}
