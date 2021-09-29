//! Specification for the file format which specifies language grammars.  This can be roughly
//! thought of as an 'AST' for the grammar files.
//!
//! When loading a language's grammar, Sapling will perform the following sequence of actions:
//! 1. Load the `*.toml` file containing that language's grammar
//! 2. Read that TOML file into a [`SpecGrammar`]
//! 3. Compile that [`SpecGrammar`] into a full [`Grammar`], which Sapling can use directly
//!
//! All these stages can generate errors, which are all bubbled up to the caller

pub mod convert;

use std::collections::HashMap;

use bimap::BiMap;
use serde::Deserialize;

use crate::Grammar;

type TypeName = String;
type TokenText = String;

/// A simplified version of [`Grammar`] which can be [`Deserialize`]d from any JSON-like data
/// structure (usually TOML).  In fact, it can **only** be generated through [`serde`], and the
/// only exported method is [`into_grammar`](SpecGrammar::into_grammar), which checks the source
/// data and returns a [`Grammar`] specifying the same language as the source `SpecGrammar`.
///
/// This type is implemented very declaratively, with minimal use of [`serde`] features.  To this
/// end, it is designed to be consulted as a reference specification for the TOML files consumed by
/// Sapling.  However, the exact implementation is considered implementation details to the rest of
/// the code, and can easily be changed and iterated on.
#[derive(Debug, Clone, Deserialize)]
pub struct SpecGrammar {
    #[serde(rename = "root")]
    root_type: TypeName,
    whitespace: self::CharSet,
    types: HashMap<TypeName, Type>,
}

impl SpecGrammar {
    pub fn into_grammar(self) -> convert::Result<Grammar> {
        convert::convert(self)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, untagged)]
// It's OK that the `Stringy` variant is quite large (~300 bytes), because this is a temporary
// datatype that's only created once.
#[allow(clippy::large_enum_variant)]
enum Type {
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
        /// A regex against which all **content** strings will be matched.  This is always inside a
        /// `(?x: <validity_regex> )` group, so whitespace and comments (`#` to the `\n`) are
        /// ignored.  Additionally, Sapling will add the necessary `^` and `$` tokens, so these are
        /// also not needed.
        validity_regex: Option<String>,

        #[serde(rename = "escape")]
        escape_rules: Option<self::EscapeRules>,
    },
}

/// A `Pattern` is the right-hand side of a grammar rule
type Pattern = Vec<PatternElement>;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, untagged)]
enum PatternElement {
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

/// See [`grammar::EscapeRules`] for docs.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct EscapeRules {
    /// A non-empty string that all escape sequences must start with.  For example, in JSON strings
    /// this is `\`
    pub(crate) start_sequence: String,
    /// See [`grammar::EscapeRules::rules`] for docs.
    pub(crate) rules: BiMap<char, String>,
    /// The prefix which takes 4 hex symbols and de-escapes them to that unicode code-point.  For
    /// example, in JSON strings this is `u` (i.e. `\uABCD` would turn into the unicode code point
    /// `0xABCD`).
    pub(crate) unicode_hex_4: Option<String>,
    /// A set of `char`s which should not be escaped, even if a rule for them exists.
    pub(crate) dont_escape: self::CharSet,
}

/// A set of `char`s, expressed as the contents of `[`, `]` in a regex (e.g. `a-zA-Z` will
/// correspond to the regex `[a-zA-Z]`).
#[derive(Debug, Clone, Deserialize)]
struct CharSet(String);
