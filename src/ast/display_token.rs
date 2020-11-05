use super::Ast;

/// How many spaces corespond to one indentation level
const INDENT_WIDTH: usize = 4;

/// A single piece of a node that can be rendered to the screen
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum DisplayToken {
    /// Some text should be rendered to the screen
    Text(String),
    /// Add some number of spaces worth of whitespace
    Whitespace(usize),
    /// Put the next token onto a new line
    Newline,
    /// Add another indent level to the code
    Indent,
    /// Remove an indent level from the code
    Dedent,
}

/// Write a stream of display tokens to a string
pub fn write_tokens<Node: Ast>(root: &Node, string: &mut String, format_style: &Node::FormatStyle) {
    let mut indentation_string = String::new();
    // Process the token string
    for (id, token) in root.display_tokens(format_style) {
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
