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
    /// The node doesn't exist
    InvalidRef,
}

/// Write a stream of display tokens to a string
pub fn write_tokens<Ref: Reference, Node: ASTSpec<Ref>>(
    root: Ref,
    node_map: &impl NodeMap<Ref, Node>,
    string: &mut String,
    format_style: &Node::FormatStyle,
) {
    let mut indentation_string = String::new();
    // Process the token string
    for (id, token) in flat_tokens(node_map, root, format_style) {
        match token {
            DisplayToken::Text(s) => {
                // Push the string we've been given
                string.push_str(&s);
            }
            DisplayToken::Child(_c) => {
                unreachable!();
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
                string.push_str(&indentation_string);
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
            DisplayToken::InvalidRef => {
                // Add a helpful error string
                string.push_str(&format!("<INVALID REF {:?}>", id));
            }
        }
    }
}

fn flat_tokens_rec<Ref: Reference, Node: ASTSpec<Ref>>(
    node_map: &impl NodeMap<Ref, Node>,
    id: Ref,
    output_vec: &mut Vec<(Ref, DisplayToken<Ref>)>,
    format_style: &Node::FormatStyle,
) {
    if let Some(node) = node_map.get_node(id) {
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

        for tok in token_vec {
            match tok {
                // If a node is a child, we should flatten its tree first
                DisplayToken::Child(c) => {
                    flat_tokens_rec(node_map, c, output_vec, format_style);
                }
                // If it isn't a child, we can just copy it as-is
                x => {
                    output_vec.push((id, x));
                }
            }
        }
    } else {
        output_vec.push((id, DisplayToken::InvalidRef));
    }
}

/// Return a flat vector of [`DisplayToken`] along with references to the nodes that own them
pub fn flat_tokens<Ref: Reference, Node: ASTSpec<Ref>>(
    node_map: &impl NodeMap<Ref, Node>,
    id: Ref,
    format_style: &Node::FormatStyle,
) -> Vec<(Ref, DisplayToken<Ref>)> {
    let mut flat_vec = Vec::new();
    flat_tokens_rec(node_map, id, &mut flat_vec, format_style);
    flat_vec
}
