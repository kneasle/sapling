pub mod dag;
pub mod cursor_path;

use crate::ast_spec::{ASTSpec, ReadableNodeMap, Reference};

pub trait EditableTree<Ref: Reference, Node: ASTSpec<Ref>>:
    ReadableNodeMap<Ref, Node> + Sized
{
    /* CONSTRUCTOR METHODS */

    /// Build a new `EditableTree` with the default AST of the given type
    fn new() -> Self;

    /* EDIT METHODS */

    /// Returns a reference to the node that is currently under the cursor.  This reference must
    /// point to a valid node.  I.e. `self.get_node(self.cursor())` should return [None].  Doing so
    /// is quite likely to cause a panic.
    fn cursor(&self) -> Ref;

    /// Returns the node referenced by the cursor.
    #[inline]
    fn cursor_node(&self) -> &Node {
        self.get_node(self.cursor()).unwrap()
    }

    /// Updates the internal state so that the tree now contains `new_node` in the position of the
    /// `cursor`.
    fn replace_cursor(&mut self, new_node: Node);

    /// Updates the internal state so that the tree now contains `new_node` inserted as the first
    /// child of the selected node.  Also moves the cursor so that the new node is selected.
    fn insert_child(&mut self, new_node: Node);

    /* DISPLAY METHODS */

    /// Build the text representation of the current tree into the given [`String`]
    fn write_text(&self, string: &mut String, format: &Node::FormatStyle);

    /// Build and return a [`String`] of the current tree
    fn to_text(&self, format: &Node::FormatStyle) -> String {
        let mut s = String::new();
        self.write_text(&mut s, format);
        s
    }
}
