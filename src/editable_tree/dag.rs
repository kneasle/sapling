use super::reference::Ref;
use super::EditableTree;
use crate::ast_spec::{ASTSpec, NodeMap};

/// An [EditableTree] that stores the history as a DAG (Directed Acyclic Graph).  This means that
/// every node that has ever been created exists somewhere in the DAG, and when changes are made,
/// every node above that is cloned until the root is reached and that root becomes the new root.
/// Therefore, moving back through the history is as simple as reading a different root node from
/// the `roots` vector, and following its descendants through the DAG of nodes.
#[derive(Debug, Clone)]
pub struct DAG<Node: ASTSpec<Ref>> {
    nodes: Vec<Node>,
    roots: Vec<Ref>,
}

impl<Node: ASTSpec<Ref>> DAG<Node> {}

impl<Node: ASTSpec<Ref>> NodeMap<Ref, Node> for DAG<Node> {
    fn get_node<'a>(&'a self, id: Ref) -> Option<&'a Node> {
        self.nodes.get(id.as_usize())
    }
}

impl<Node: ASTSpec<Ref>> EditableTree<Ref, Node> for DAG<Node> {
    fn new() -> Self {
        DAG {
            nodes: vec![Node::default()],
            roots: vec![Ref::new(0)],
        }
    }

    fn root(&self) -> Ref {
        // We can unwrap here because we uphold the invariant that there must always be at least
        // one root in the history.
        *self.roots.last().unwrap()
    }
}
