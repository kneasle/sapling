use super::reference::Index;
use super::EditableTree;
use crate::ast_spec::{ASTSpec, NodeMap};

/// An [EditableTree] that stores the history as a DAG (Directed Acyclic Graph).  This means that
/// every node that has ever been created exists somewhere in the DAG, and when changes are made,
/// every node above that is cloned until the root is reached and that root becomes the new root.
/// Therefore, moving back through the history is as simple as reading a different root node from
/// the `roots` vector, and following its descendants through the DAG of nodes.
#[derive(Debug, Clone)]
pub struct DAG<Node: ASTSpec<Index>> {
    nodes: Vec<Node>,
    roots: Vec<Index>,
}

impl<Node: ASTSpec<Index>> DAG<Node> {}

impl<Node: ASTSpec<Index>> NodeMap<Index, Node> for DAG<Node> {
    /// Create a new `NodeMap` with a given `Node` as root
    fn with_root(root: Node) -> Self {
        DAG {
            nodes: vec![root],
            roots: vec![Index::from(0)],
        }
    }
    
    /// Get the reference of the root node of the tree
    fn root(&self) -> Index {
        // We can unwrap here because we uphold the invariant that there must always be at least
        // one root in the history.
        *self.roots.last().unwrap()
    }

    /// Get the reference of the root node of the tree
    fn set_root(&mut self, new_root: Index) -> bool {
        let is_ref_valid = self.get_node(new_root).is_some();
        if is_ref_valid {
            self.roots.push(new_root);
        }
        is_ref_valid
    }

    /// Gets node from a reference, returning [None] if the reference is invalid.
    fn get_node<'a>(&'a self, id: Index) -> Option<&'a Node> {
        self.nodes.get(id.as_usize())
    }

    /// Gets mutable node from a reference, returning [None] if the reference is invalid.
    fn get_node_mut<'a>(&'a mut self, id: Index) -> Option<&'a mut Node> {
        self.nodes.get_mut(id.as_usize())
    }

    /// Add a new `Node` to the tree, and return its reference
    fn add_node(&mut self, node: Node) -> Index {
        self.nodes.push(node);
        Index::from(self.nodes.len() - 1)
    }
}

impl<Node: ASTSpec<Index>> EditableTree<Index, Node> for DAG<Node> {
    fn new() -> Self {
        DAG {
            nodes: vec![Node::default()],
            roots: vec![Index::from(0)],
        }
    }
}
