pub mod dag;

use crate::ast_spec::{ASTSpec, NodeMap, Reference};

pub trait EditableTree<Ref: Reference, Node: ASTSpec<Ref>>: NodeMap<Ref, Node> + Sized {
    /// Build a new EditableTree with a default AST
    fn new() -> Self;
}
