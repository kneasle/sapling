//! Specification of an editable, undoable buffer of trees and some implementations thereof.

pub mod cursor_path;
pub mod dag;
pub mod spec;

use crate::ast_spec::ASTSpec;
use crate::node_map::{NodeMap, Reference};

/// The possible ways you can move the cursor
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Direction {
    Up,
    Down,
    Prev,
    Next,
}

/// A trait specifying an editable, undoable buffer of trees
pub trait EditableTree<Ref: Reference, Node: ASTSpec<Ref>>: NodeMap<Ref, Node> + Sized {
    /* CONSTRUCTOR METHODS */

    /// Build a new `EditableTree` with the default AST of the given type
    fn new() -> Self;

    /* HISTORY METHODS */

    /// Move one step back in the tree history, returning `false` if there are no more changes
    fn undo(&mut self) -> bool;

    /// Move one step forward in the tree history, return `false` if there was no change to be
    /// redone
    fn redo(&mut self) -> bool;

    /* NAVIGATION METHODS */

    /// Returns a reference to the node that is currently under the cursor.  This reference must
    /// point to a valid node.  I.e. `self.get_node(self.cursor())` should return [None].  Doing so
    /// is quite likely to cause a panic.
    fn cursor(&self) -> Ref;

    /// Returns the node referenced by the cursor.
    #[inline]
    fn cursor_node(&self) -> &Node {
        self.get_node(self.cursor()).unwrap()
    }

    /// Move the cursor in a given direction across the tree.  Returns [`Some`] error string if an
    /// error is found, or [`None`] if the movement was possible.
    fn move_cursor(&mut self, direction: Direction) -> Option<String>;

    /* EDIT METHODS */

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
