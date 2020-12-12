use super::Ast;
use std::borrow::Cow;

/// How many spaces corespond to one indentation level
const INDENT_WIDTH: usize = 4;

pub mod syntax_category {
    /// Text that shouldn't be highlighted a specific colour: used for things like punctuation.
    pub const DEFAULT: &'static str = "default";
    /// Constant values like 'true', 'false'
    pub const CONST: &'static str = "const";
    /// Literal values like strings and integers
    pub const LITERAL: &'static str = "literal";
    /// Non-documentation comments
    pub const COMMENT: &'static str = "comment";
    /// Code identifier, such as variables or function names
    pub const IDENT: &'static str = "ident";
    /// A name that's reserved by the language for a specific purpose (e.g. `if`, `while` in nearly
    /// every language; `use`, `pub`, `const` in Rust)
    pub const KEYWORD: &'static str = "keyword";
    /// A pre-processor directive.  For example: `#if`, `#define` in C/C++ or `#[derive(...)]` in
    /// Rust
    pub const PRE_PROC: &'static str = "pre-proc";
    /// A datatype, e.g. `int`, `long` from C or `usize`, `f64`, `String` in Rust
    pub const TYPE: &'static str = "type";
    /// Special pieces of text, such as escaped characters (`\n`, `\t`, etc.) in string literals
    pub const SPECIAL: &'static str = "special";
    /// Copied from Vim (do we really need this?)
    pub const UNDERLINED: &'static str = "underlined";
    /// Any code that is an error
    pub const ERROR: &'static str = "error";
}

/// A category of text that should be syntax highlighted the same color.
///
/// See [`syntax_category`] for common values
pub type SyntaxCategory = &'static str;

/// A single piece of a node that can be rendered to the screen
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum DisplayToken {
    /// Some text (as either a `&'static str` or a [`String`]) that should be rendered verbatim to
    /// the screen
    Text(Cow<'static, str>, SyntaxCategory),
    /// Add some number of spaces worth of whitespace
    Whitespace(usize),
    /// Put the next token onto a new line
    Newline,
    /// Add another indent level to the code
    Indent,
    /// Remove an indent level from the code
    Dedent,
}

/// A wrapper for [`DisplayToken`] that will be returned by [`Ast::display_tokens`] and allows for
/// child references to be recursively expanded.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RecTok<'arena, Node> {
    Tok(DisplayToken),
    Child(&'arena Node),
}

impl<'arena, Node> RecTok<'arena, Node> {
    /// Creates a new `RecTok` from a static [`str`] and a [`SyntaxCategory`]
    pub fn from_str(text: &'static str, syntax_category: SyntaxCategory) -> Self {
        RecTok::Tok(DisplayToken::Text(Cow::from(text), syntax_category))
    }

    /// Creates a new `RecTok` from an owned [`String`] and a [`SyntaxCategory`]
    pub fn from_string(text: String, syntax_category: SyntaxCategory) -> Self {
        RecTok::Tok(DisplayToken::Text(Cow::from(text), syntax_category))
    }
}

/// Write a stream of display tokens to a string
pub fn write_tokens<'arena, Node: Ast<'arena>>(
    root: &'arena Node,
    string: &mut String,
    format_style: &Node::FormatStyle,
) {
    let mut indentation_string = String::new();

    // Process the token string
    for (_id, tok) in root.display_tokens(format_style) {
        match tok {
            DisplayToken::Text(s, _) => {
                // Push the string we've been given
                string.push_str(&s);
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
        }
    }
}
