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

use crate::Grammar;

use self::convert::ConvertResult;

type TypeName = String;
type TokenText = String;
type Pattern = Vec<PatternElement>;

/// A simplified version of [`Grammar`] which can be [`Deserialize`]d from any JSON-like data
/// structure (usually TOML).  In fact, it can **only** be generated through [`serde`], and the
/// only exported method is [`to_grammar`](SpecGrammar::to_grammar), which checks the source data
/// and returns a [`Grammar`] specifying the same language as the source `SpecGrammar`.
///
/// This type is implemented very declaratively, with minimal use of [`serde`] features.  To this
/// end, it is designed to be consulted as a reference specification for the TOML files consumed by
/// Sapling.  However, the exact implementation is considered implementation details to the rest of
/// the code, and can easily be changed and iterated on.
#[derive(Debug, Clone, Deserialize)]
pub struct SpecGrammar {
    #[serde(rename = "root")]
    root_type: TypeName,
    whitespace: Whitespace,
    types: HashMap<TypeName, Type>,
}

impl SpecGrammar {
    #[inline]
    pub fn into_grammar(self) -> ConvertResult<Grammar> {
        convert::convert(self)
    }
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
    Token(TokenText),
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
        delimiter: TokenText,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, untagged)]
pub enum Whitespace {
    Chars(String),
}

/// A regex which matches every possible string
fn regex_any() -> String {
    ".*".to_owned()
}
