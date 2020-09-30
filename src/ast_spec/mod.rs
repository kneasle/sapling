//! A module to contain Rust representations of ASTs in a format that sapling can work with.

pub mod json;
pub mod size;
pub mod test_json;

use crate::node_map::{NodeMap, Reference};
use size::Size;

// Import used only for doc comments
#[allow(unused_imports)]
use crate::editable_tree::EditableTree;

/// A single piece of a node that can be rendered to the screen
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum DisplayToken<Ref: Reference> {
    /// Some text should be rendered to the screen
    Text(String),
    /// A child of this node should be rendered to the screen
    Child(Ref),
    /// Add some number of spaces worth of whitespace
    Whitespace(usize),
    /// Put the next token onto a new line
    Newline,
    /// Add another indent level to the code
    Indent,
    /// Remove an indent level from the code
    Dedent,
}

/// How many spaces corespond to one indentation level
const INDENT_WIDTH: usize = 4;

/// Write a stream of display tokens to a string, given an indentation string
fn write_tokens_with_indent<Ref: Reference, Node: ASTSpec<Ref>>(
    node: &Node,
    node_map: &impl NodeMap<Ref, Node>,
    string: &mut String,
    indentation_string: &mut String,
    format_style: &Node::FormatStyle,
) {
    let mut token_vec = node.display_tokens(format_style);
    // If we every encounter a newline followed by an indent/dedent, we should swap them round so
    // that the indent/dedent is always first.
    for i in 0..token_vec.len() - 1 {
        if token_vec[i] == DisplayToken::Newline
            && (token_vec[i + 1] == DisplayToken::Indent
                || token_vec[i + 1] == DisplayToken::Dedent)
        {
            token_vec.swap(i, i + 1);
        }
    }
    // Process the token string
    for token in token_vec {
        match token {
            DisplayToken::Text(s) => {
                // Push the string we've been given
                string.push_str(&s);
            }
            DisplayToken::Child(c) => {
                if let Some(child) = node_map.get_node(c) {
                    // If the child reference is valid, then recursively write that child
                    write_tokens_with_indent(
                        child,
                        node_map,
                        string,
                        indentation_string,
                        format_style,
                    );
                } else {
                    // If the child reference isn't valid, then write an error message
                    string.push_str(&format!("<INVALID NODE {:?}>", c));
                }
            }
            DisplayToken::Whitespace(n) => {
                // Push 'n' many spaces
                for _ in 0..n {
                    string.push(' ');
                }
            }
            DisplayToken::Newline => {
                // Push a newline and keep indentation
                string.push('\n');
                string.push_str(indentation_string);
            }
            DisplayToken::Indent => {
                // Add `INDENT_WIDTH` spaces to the indentation_string
                for _ in 0..INDENT_WIDTH {
                    indentation_string.push(' ');
                }
            }
            DisplayToken::Dedent => {
                // Remove `INDENT_WIDTH` spaces to the indentation_string
                for _ in 0..INDENT_WIDTH {
                    let popped_char = indentation_string.pop();
                    debug_assert_eq!(popped_char, Some(' '));
                }
            }
        }
    }
}

/// Write a stream of display tokens to a string
fn write_tokens<Ref: Reference, Node: ASTSpec<Ref>>(
    root: &Node,
    node_map: &impl NodeMap<Ref, Node>,
    string: &mut String,
    format_style: &Node::FormatStyle,
) {
    let mut indentation_string = String::new();
    write_tokens_with_indent(
        root,
        node_map,
        string,
        &mut indentation_string,
        format_style,
    );
}

/// The specification of an AST that sapling can edit
pub trait ASTSpec<Ref: Reference>: std::fmt::Debug + Clone + Eq + Default {
    /// A type parameter that will represent the different ways this AST can be rendered
    type FormatStyle;

    /* FORMATTING FUNCTIONS */

    /// Returns an iterator of all the items that need to be rendered to the screen to make up this
    /// node, along with their on-screen locations.
    fn display_tokens(&self, format_style: &Self::FormatStyle) -> Vec<DisplayToken<Ref>>;

    /// Determine the space on the screen occupied by this node in an AST
    fn size(&self, node_map: &impl NodeMap<Ref, Self>, format_style: &Self::FormatStyle) -> Size;

    /// Write the textual representation of this AST to a string
    fn write_text(
        &self,
        node_map: &impl NodeMap<Ref, Self>,
        string: &mut String,
        format_style: &Self::FormatStyle,
    ) {
        write_tokens(self, node_map, string, format_style);
    }

    /// Make a [`String`] representing this AST.
    /// Same as [`write_text`](ASTSpec::write_text) but creates a new [`String`].
    fn to_text(
        &self,
        node_map: &impl NodeMap<Ref, Self>,
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
        node_map: &impl NodeMap<Ref, Self>,
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
    fn write_tree_view(&self, node_map: &impl NodeMap<Ref, Self>, string: &mut String) {
        let mut indentation_string = String::new();
        self.write_tree_view_recursive(node_map, string, &mut indentation_string);
        // Pop the unnecessary newline at the end
        let popped_char = string.pop();
        debug_assert_eq!(Some('\n'), popped_char);
    }

    /// Build a string of the a tree view of this node, similar to the output of the Unix command
    /// 'tree'.  This is the same as [`write_tree_view`](ASTSpec::write_tree_view), except that it
    /// returns a [`String`] rather than appending to an existing [`String`].
    fn tree_view(&self, node_map: &impl NodeMap<Ref, Self>) -> String {
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
