use super::EditableTree;
use crate::ast_spec::{ASTSpec, NodeMap, Reference};

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Ref(usize);

impl Reference for Ref {}

impl Ref {
    pub fn new(val: usize) -> Ref {
        Ref(val)
    }

    pub fn as_usize(self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone)]
pub struct Persistent<Node: ASTSpec<Ref>> {
    nodes: Vec<Node>,
    roots: Vec<Ref>,
}

impl<Node: ASTSpec<Ref>> Persistent<Node> {}

impl<Node: ASTSpec<Ref>> NodeMap<Ref, Node> for Persistent<Node> {
    fn get_node<'a>(&'a self, id: Ref) -> Option<&'a Node> {
        self.nodes.get(id.as_usize())
    }
}

impl<Node: ASTSpec<Ref>> EditableTree<Ref, Node> for Persistent<Node> {
    fn new() -> Self {
        Persistent {
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
