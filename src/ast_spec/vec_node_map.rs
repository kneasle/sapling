use super::{ASTSpec, NodeMap};
use crate::editable_tree::reference::Ref;

pub struct VecNodeMap<Node> {
    nodes: Vec<Node>,
    root: Ref,
}

impl<Node: ASTSpec<Ref>> NodeMap<Ref, Node> for VecNodeMap<Node> {
    /// Create a new `NodeMap` with a given `Node` as root
    fn with_root(node: Node) -> Self {
        VecNodeMap {
            nodes: vec![node],
            root: Ref::from(0),
        }
    }

    /// Get the reference of the root node of the tree
    #[inline]
    fn root(&self) -> Ref {
        self.root
    }

    /// Gets node from a reference, returning [None] if the reference is invalid.
    #[inline]
    fn get_node<'a>(&'a self, id: Ref) -> Option<&'a Node> {
        self.nodes.get(id.as_usize())
    }

    /// Gets mutable node from a reference, returning [None] if the reference is invalid.
    #[inline]
    fn get_node_mut<'a>(&'a mut self, id: Ref) -> Option<&'a mut Node> {
        self.nodes.get_mut(id.as_usize())
    }

    /// Add a new `Node` to the tree, and return its reference
    #[inline]
    fn add_node(&mut self, node: Node) -> Ref {
        self.nodes.push(node);
        Ref::from(self.nodes.len() - 1)
    }
}
