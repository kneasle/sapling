//! A module to contain Rust representations of ASTs in a format that sapling can work with.

pub mod display_token;
pub mod json;
pub mod size;
pub mod test_json;

use display_token::{write_tokens, DisplayToken, RecTok};
use size::Size;

/// The specification of an AST that sapling can edit
pub trait Ast<'arena>: std::fmt::Debug + Clone + Eq + Default + std::hash::Hash {
    /// A type parameter that will represent the different ways this AST can be rendered
    type FormatStyle;
    /// Error type returned by [Ast::insert_child].
    type InsertError: std::error::Error;
    /// Error type returned by [Ast::delete_child].
    type DeleteError: std::error::Error;

    /* FORMATTING FUNCTIONS */

    /// Returns an iterator of all the items that need to be rendered to the screen to make up this
    /// node, along with their on-screen locations.
    fn display_tokens_rec(
        &'arena self,
        format_style: &Self::FormatStyle,
    ) -> Vec<RecTok<'arena, Self>>;

    fn display_tokens(
        &'arena self,
        format_style: &Self::FormatStyle,
    ) -> Vec<(&'arena Self, DisplayToken)> {
        let mut tok_pairs: Vec<(&'arena Self, DisplayToken)> = Vec::new();
        for i in self.display_tokens_rec(format_style) {
            match i {
                RecTok::Tok(t) => {
                    tok_pairs.push((self, t));
                }
                RecTok::Child(c) => {
                    tok_pairs.extend(c.display_tokens(format_style));
                }
            }
        }
        tok_pairs
    }

    /// Determine the space on the screen occupied by this node in an AST
    fn size(&self, format_style: &Self::FormatStyle) -> Size;

    /// Write the textual representation of this AST to a string
    fn write_text(&'arena self, string: &mut String, format_style: &Self::FormatStyle) {
        write_tokens(self, string, format_style);
    }

    /// Make a [`String`] representing this AST.
    /// Same as [`write_text`](ASTSpec::write_text) but creates a new [`String`].
    fn to_text(&'arena self, format_style: &Self::FormatStyle) -> String {
        let mut s = String::new();
        self.write_text(&mut s, format_style);
        s
    }

    /* DEBUG VIEW FUNCTIONS */

    /// Get a slice over the direct children of this node.  This operation is expected to be
    /// cheap - it will be used a lot of times without caching the results.
    fn children<'s>(&'s self) -> &'s [&'arena Self];

    /// Get a mutable slice over the direct children of this node.  Like
    /// [`children`](ASTSpec::children), this operation is expected to be
    /// cheap - it will be used a lot of times without caching the results.
    fn children_mut<'s>(&'s mut self) -> &'s mut [&'arena Self];

    /// Removes the child at a given index from the children of this node, if possible.  If the
    /// removal was not possible, then we return a custom error type.
    fn delete_child(&mut self, index: usize) -> Result<(), Self::DeleteError>;

    /// Insert an extra child into the children of this node at a given index.
    fn insert_child(
        &mut self,
        new_node: &'arena Self,
        index: usize,
    ) -> Result<(), Self::InsertError>;

    /// Get the display name of this node
    fn display_name(&self) -> String;

    fn write_tree_view_recursive(
        &'arena self,
        string: &mut String,
        indentation_string: &mut String,
    ) {
        // Push the node's display name with indentation and a newline
        string.push_str(indentation_string);
        string.push_str(&self.display_name());
        string.push('\n');
        // Indent by two spaces
        indentation_string.push_str("  ");
        // Write all the children
        for child in self.children().iter() {
            child.write_tree_view_recursive(string, indentation_string);
        }
        // Reset indentation
        for _ in 0..2 {
            indentation_string.pop();
        }
    }

    /// Render a tree view of this node, similar to the output of the Unix command 'tree'
    fn write_tree_view(&'arena self, string: &mut String) {
        let mut indentation_string = String::new();
        self.write_tree_view_recursive(string, &mut indentation_string);
        // Pop the unnecessary newline at the end
        let popped_char = string.pop();
        debug_assert_eq!(Some('\n'), popped_char);
    }

    /// Build a string of the a tree view of this node, similar to the output of the Unix command
    /// 'tree'.  This is the same as [`write_tree_view`](ASTSpec::write_tree_view), except that it
    /// returns a [`String`] rather than appending to an existing [`String`].
    fn tree_view(&'arena self) -> String {
        let mut s = String::new();
        self.write_tree_view(&mut s);
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
