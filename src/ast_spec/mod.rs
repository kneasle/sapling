//! A module to contain Rust representations of ASTs in a format that sapling can work with.

pub mod json;
pub mod test_json;

use crate::node_map::{NodeMapMut, Reference};

#[allow(unused_imports)]
use crate::editable_tree::EditableTree;

/// The specification of an AST that sapling can edit
pub trait ASTSpec<Ref: Reference>: std::fmt::Debug + Clone + Eq + Default {
    /// A type parameter that will represent the different ways this AST can be rendered
    type FormatStyle;

    /* FORMATTING FUNCTIONS */

    /// Write the textual representation of this AST to a string
    fn write_text(
        &self,
        node_map: &impl NodeMapMut<Ref, Self>,
        string: &mut String,
        format_style: &Self::FormatStyle,
    );

    /// Make a [`String`] representing this AST.
    /// Same as [`write_text`](ASTSpec::write_text) but creates a new [`String`].
    fn to_text(
        &self,
        node_map: &impl NodeMapMut<Ref, Self>,
        format_style: &Self::FormatStyle,
    ) -> String {
        let mut s = String::new();
        self.write_text(node_map, &mut s, format_style);
        s
    }

    /* DEBUG VIEW FUNCTIONS */

    /// Get a slice over the direct children of this node.  This operation is expected to be
    /// cheap - it will be used a lot of times without caching the results.
    fn children(&self) -> &[Ref];

    /// Get a mutable slice over the direct children of this node.  Like
    /// [`children`](ASTSpec::children), this operation is expected to be
    /// cheap - it will be used a lot of times without caching the results.
    fn children_mut(&mut self) -> &mut [Ref];

    /// Get the display name of this node
    fn display_name(&self) -> String;

    fn write_tree_view_recursive(
        &self,
        node_map: &impl NodeMapMut<Ref, Self>,
        string: &mut String,
        indentation_string: &mut String,
    ) {
        // Push the node's display name with indentation and a newline
        string.push_str(indentation_string);
        string.push_str(&self.display_name());
        string.push('\n');
        // Indent by two spaces
        indentation_string.push_str("  ");
        for child_ref in self.children().iter() {
            if let Some(child) = node_map.get_node(*child_ref) {
                child.write_tree_view_recursive(node_map, string, indentation_string);
            }
        }
        // Reset indentation
        for _ in 0..2 {
            indentation_string.pop();
        }
    }

    /// Render a tree view of this node, similar to the output of the Unix command 'tree'
    fn write_tree_view(&self, node_map: &impl NodeMapMut<Ref, Self>, string: &mut String) {
        let mut indentation_string = String::new();
        self.write_tree_view_recursive(node_map, string, &mut indentation_string);
        // Pop the unnecessary newline at the end
        debug_assert_eq!(Some('\n'), string.pop());
    }

    /// Build a string of the a tree view of this node, similar to the output of the Unix command
    /// 'tree'.  This is the same as [`write_tree_view`](ASTSpec::write_tree_view), except that it
    /// returns a [`String`] rather than appending to an existing [`String`].
    fn tree_view(&self, node_map: &impl NodeMapMut<Ref, Self>) -> String {
        let mut s = String::new();
        self.write_tree_view(node_map, &mut s);
        s
    }

    /* AST EDITING FUNCTIONS */

    /// Generate an iterator over the possible shorthand [`char`]s that a user could type to replace
    /// this node with something else.
    fn replace_chars(&self) -> Box<dyn Iterator<Item = char>>;

    /// Returns whether or not a given [`char`] is in [`Self::replace_chars`]
    fn is_replace_char(&self, c: char) -> bool {
        self.replace_chars().any(|x| x == c)
    }

    /// Generate a new node from a [`char`] that a user typed as part of the `r` command.  If `c` is
    /// an element of [`get_replace_chars`](ASTSpec::replace_chars), this must return [`Some`] node,
    /// if it isn't, then this should return [`None`].
    fn from_char(&self, c: char) -> Option<Self>;

    /// Generate an iterator over the possible shorthand [`char`]s that a user could type to insert
    /// other nodes into this one
    fn insert_chars(&self) -> Box<dyn Iterator<Item = char>>;

    /// Returns whether or not a given [`char`] is in [`Self::insert_chars`]
    fn is_insert_char(&self, c: char) -> bool {
        self.insert_chars().any(|x| x == c)
    }
}
