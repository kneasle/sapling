pub mod dag;
pub mod reference;

use crate::ast_spec::{ASTSpec, NodeMap, Reference};

pub trait EditableTree<Ref: Reference, Node: ASTSpec<Ref>>: NodeMap<Ref, Node> + Sized {
    /// Build a new EditableTree with a default AST
    fn new() -> Self;

    /// Return the current root of the tree
    fn root(&self) -> Ref;

    /// Render the text representing the current AST
    fn to_text(&self, format: &Node::FormatStyle) -> String {
        match self.get_node(self.root()) {
            Some(root) => root.to_text(self, &format),
            None => "<INVALID ROOT NODE>".to_string(),
        }
    }
}
