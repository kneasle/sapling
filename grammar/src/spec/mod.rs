//! Specification for the file format which specifies language grammars.  This can be roughly
//! thought of as an 'AST' for the grammar files.
//!
//! When loading a language's grammar, Sapling will perform the following sequence of actions:
//! 1. Load the `*.toml` file containing that language's grammar
//! 2. Read that TOML file into a [`Spec`]
//! 3. Compile that [`Spec`] into a full [`Grammar`], which Sapling can use directly
//!
//! All these stages can generate errors, which are all bubbled up to the caller

pub(crate) mod convert;

use std::collections::HashMap;

use serde::Deserialize;

type TypeName = String;
type TokenValue = String;
type Pattern = Vec<PatternElement>;

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct SpecGrammar {
    #[serde(rename = "root")]
    root_type: TypeName,
    #[serde(rename = "whitespace")]
    whitespace_chars: String,
    types: HashMap<TypeName, Type>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, untagged)]
pub(crate) enum Type {
    Pattern {
        key: Option<String>,
        #[serde(default = "Vec::new")]
        keys: Vec<String>,

        #[serde(default = "Vec::new")]
        children: Vec<TypeName>,
        pattern: Option<Pattern>,
    },
    Stringy {
        key: Option<String>,
        #[serde(default = "Vec::new")]
        keys: Vec<String>,

        /// Expected to be true.  TODO: Is there a better way to handle different node types
        stringy: bool,

        /// String appended before the escaped contents
        #[serde(default = "String::new")]
        delim_start: String,
        /// String appended after the escaped contents
        #[serde(default = "String::new")]
        delim_end: String,

        /// Default node **contents** (i.e. unescaped string).  This must match `validity_regex`.
        #[serde(default = "String::new", rename = "default")]
        default_content: String,
        /// A regex against which all **content** strings will be matched.  This implicitly has `^`
        /// and `$` appended to the start and end (respectively).
        #[serde(default = "regex_any")]
        validity_regex: String,
        /// Maps **escaped** strings to **content** strings (since characters can often be escaped
        /// in multiple different ways - take the newline character escaping to either `\n` or
        /// `\u000A`).  This could be better named `deescape_rules`.
        #[serde(default = "HashMap::new")]
        escape_rules: HashMap<String, String>,
        /// The prefix prepended to 4-character hex unicode escape sequences.  For example, in JSON
        /// this is `\u`.  Empty signifies that unicode escaping is not possible.
        #[serde(default = "String::new")]
        unicode_escape_prefix: String,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, untagged)]
pub(crate) enum PatternElement {
    /// A single, unchanging piece of non-whitespace text
    Token(TokenValue),
    /// A position where a sub-node will be placed
    Type {
        #[serde(rename = "type")]
        name: TypeName,
    },
    /// A sequence of repeating instances of a `pattern`, separated by instances of a `delimiter`.
    /// This does not allow trailing delimiters.  For example,
    /// ```text
    /// Seq {
    ///     pattern: [Type { name: "value" }],
    ///     delimiter: ",",
    /// }
    /// ```
    /// matches `<value>` or `<value>, <value>, <value>` but **not** `<value>, <value>, <value>,`
    /// (note the trailing comma).
    Seq {
        #[serde(rename = "seq")]
        pattern: Pattern,
        delimiter: TokenValue,
    },
}

/// A regex which matches every possible string
fn regex_any() -> String {
    ".*".to_owned()
}
