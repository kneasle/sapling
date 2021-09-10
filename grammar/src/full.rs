use std::collections::HashMap;

use index_vec::IndexVec;
use regex::Regex;

/// A complete specification for how to parse files of any particular language.
#[derive(Debug, Clone)]
pub struct Grammar {
    root_type: TypeId,
    whitespace_chars: Vec<char>,
    types: IndexVec<TypeId, Type>,
    tokens: IndexVec<TokenId, Token>,
}

impl Grammar {
    pub fn new(
        root_type: TypeId,
        whitespace_chars: Vec<char>,
        types: IndexVec<TypeId, Type>,
        tokens: IndexVec<TokenId, Token>,
    ) -> Self {
        Self {
            root_type,
            whitespace_chars,
            types,
            tokens,
        }
    }

    pub fn token_text(&self, id: TokenId) -> &str {
        &self.tokens[id].text
    }
}

#[derive(Debug, Clone)]
pub struct Type {
    // TODO: make fields less public
    /// The name given to this `Type`.  Must be unique within the [`Grammar`]
    pub(crate) name: String,
    /// The different sequences of keystrokes which refer to this `Type`.
    ///
    /// This can be empty, in which case this `Type` can never be created explicitly; take, for
    /// example, 'node class' types like expressions (which can never be instantiated directly) or
    /// JSON fields (which are only created implicitly to contain other nodes).
    pub(crate) keys: Vec<String>,
    pub(crate) inner: TypeInner,
}

#[derive(Debug, Clone)]
pub enum TypeInner {
    Pattern {
        /// A set of types to which this type can be implicitly converted.
        child_types: Vec<TypeId>,
        pattern: Option<Pattern>,
    },
    /// A node which store a string value, editable by the user.  These nodes always correspond to
    /// precisely one token.
    ///
    /// Note that this is not limited to string literals: for example, identifiers and other
    /// literals also use this node type.  Accordingly, the definition `Stringy` type is extremely
    /// flexible to accommodate these different use cases.
    Stringy(Stringy),
}

#[derive(Debug, Clone)]
pub struct Stringy {
    // TODO: make fields less public
    /// String appended before the escaped contents
    pub(crate) delim_start: String,
    /// String appended after the escaped contents
    pub(crate) delim_end: String,

    /// A [`Regex`] which all node **contents** must match.  This always starts and ends with `^`
    /// and `$` to force the engine to match the whole string.
    pub(crate) validity_regex: Regex,
    /// Default **contents** of new nodes.  This must match `validity_regex`.
    pub(crate) default_content: String,

    /// Maps **content** substrings to **escaped** substrings.  TODO: Replace this with a
    /// pre-compiled match regex
    pub(crate) escape_rules: HashMap<String, String>,
    /// Maps **escaped** substrings to **content** substrings.  TODO: Replace this with a
    /// pre-compiled match regex
    // TODO: deescape_rules: HashMap<String, String>,
    /// The prefix prepended to 4-character hex unicode escape sequences.  For example, in JSON
    /// this is `\u`.  Empty signifies that unicode escaping is not possible.
    pub(crate) unicode_escape_prefix: String,
    // TODO: Allow encoding of invalid escaped strings - e.g. in JSON a `\` must be succeeded by
    // one of `\/bfntru"`
    //
    // TODO: Specify syntax highlighting groups
}

//////////////
// PATTERNS //
//////////////

pub type Pattern = Vec<PatternElement>;

#[derive(Debug, Clone)]
pub enum PatternElement {
    /// A single, unchanging piece of non-whitespace text
    Token(TokenId),
    /// A position where a sub-node will be placed.  The sub-node's type must be a descendant of
    /// the specified [`TypeId`].
    Type(TypeId),
    /// A sequence of repeating instances of a `pattern`, separated by instances of a `delimiter`.
    /// This does not allow trailing delimiters.  For example,
    /// ```text
    /// Seq {
    ///     pattern: [Type(<value>)],
    ///     delimiter: ",",
    /// }
    /// ```
    /// matches `<value>` or `<value>, <value>, <value>` but **not** `<value>, <value>, <value>,`
    /// (note the trailing comma in the last example).
    Seq {
        pattern: Pattern,
        delimiter: TokenId,
    },
}

#[derive(Debug, Clone)]
pub struct Token {
    text: String,
    // TODO: Syntax highlighting groups
}

impl Token {
    pub fn new(text: String) -> Self {
        Self { text }
    }
}

//////////////////
// HELPER TYPES //
//////////////////

// TODO: Tag these indices with which `Grammar` created them
index_vec::define_index_type! { pub struct TypeId = usize; }
index_vec::define_index_type! { pub struct TokenId = usize; }
