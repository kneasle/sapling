use super::{ASTSpec, Reference};
use crate::node_map::NodeMap;

/// How many spaces corespond to one indentation level
const INDENT_WIDTH: usize = 4;

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
pub fn write_tokens<Ref: Reference, Node: ASTSpec<Ref>>(
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
