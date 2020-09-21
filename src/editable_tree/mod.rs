pub mod dag;

use crate::ast_spec::{ASTSpec, Reference};

pub trait EditableTree<Ref: Reference, Node: ASTSpec<Ref>>: Sized {
    /* CONSTRUCTOR METHODS */

    /// Build a new EditableTree with the default AST of the given type
    fn new() -> Self;

    /* DISPLAY METHODS */

    /// Build the text representation of the current tree into the given [String]
    fn write_text(&self, string: &mut String, format: &Node::FormatStyle);

    /// Build and return a [String] of the current tree
    fn to_text(&self, format: &Node::FormatStyle) -> String {
        let mut s = String::new();
        self.write_text(&mut s, format);
        s
    }
}
