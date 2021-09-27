use std::{
    collections::HashSet,
    fmt::{Debug, Formatter},
};

use bimap::BiMap;
use index_vec::{IndexSlice, IndexVec};
use itertools::Itertools;
use regex::Regex;
use serde::Deserialize;

use crate::{
    char_set::{self, CharSet},
    parser::{self, Ast},
};

/// A complete specification for how to parse files of any particular language.
#[derive(Debug, Clone)]
pub struct Grammar {
    root_type: TypeId,
    whitespace: Whitespace,
    pub(crate) types: TypeVec<Type>,
    tokens: IndexVec<TokenId, Token>,

    /* LOOK-UP TABLES FOR THE TOKENIZER/PARSER */
    /// Mapping from token texts to IDs, stored **in decreasing order** of the text length.  This
    /// makes sure that the tokenizer always consumes the largest possible token (e.g. `"&&"`
    /// should be tokenized into just `&&`, rather than two `&`s).
    static_tokens_decreasing: Vec<(String, TokenId)>,
}

impl Grammar {
    pub fn new(
        root_type: TypeId,
        whitespace: Whitespace,
        types: TypeVec<Type>,
        tokens: IndexVec<TokenId, Token>,
    ) -> Self {
        // Sort static tokens by decreasing order of length
        let mut static_tokens_decreasing = tokens
            .iter_enumerated()
            .map(|(id, token)| (token.text().to_owned(), id))
            .collect_vec();
        static_tokens_decreasing.sort_by_key(|(name, _id)| std::cmp::Reverse(name.len()));

        Self {
            root_type,
            whitespace,
            types,
            tokens,
            static_tokens_decreasing,
        }
    }

    /// Construct a concrete AST representing a [`str`]ing of the root type according to this [`Grammar`].
    pub fn parse_root<'s, N: Ast>(&self, s: &'s str) -> Result<(&'s str, N), parser::Error> {
        parser::parse(self, self.root_type, s)
    }

    /// Construct a concrete AST representing a [`str`]ing according to this [`Grammar`].
    pub fn parse<'s, N: Ast>(
        &self,
        type_id: TypeId,
        s: &'s str,
    ) -> Result<(&'s str, N), parser::Error> {
        parser::parse(self, type_id, s)
    }

    ///////////
    // TYPES //
    ///////////

    pub fn root_type(&self) -> TypeId {
        self.root_type
    }

    pub fn get_type(&self, id: TypeId) -> &Type {
        &self.types[id]
    }

    pub fn type_name(&self, id: TypeId) -> &str {
        &self.types[id].name
    }

    pub fn types(&self) -> &IndexSlice<TypeId, [Type]> {
        &self.types
    }

    ////////////
    // TOKENS //
    ////////////

    pub fn tokens(&self) -> &IndexSlice<TokenId, [Token]> {
        &self.tokens
    }

    pub fn num_tokens(&self) -> usize {
        self.tokens.len()
    }

    pub fn token_text(&self, id: TokenId) -> &str {
        &self.tokens[id].text
    }

    pub fn whitespace(&self) -> &Whitespace {
        &self.whitespace
    }

    /// Returns the static tokens in `self`, in decreasing order of length
    pub fn static_tokens_decreasing(&self) -> &[(String, TokenId)] {
        self.static_tokens_decreasing.as_slice()
    }
}

#[derive(Debug, Clone)]
pub struct Type {
    // TODO: make fields less public
    /// The name given to this `Type`.  Must be unique within the [`Grammar`]
    pub(crate) name: String,
    /// The different sequences of keystrokes which refer to this `Type`.
    ///
    /// This can be empty, in which case this `Type` can never be created explicitly: take, for
    /// example, 'node class' types like expressions (which can never be instantiated directly) or
    /// JSON fields (which are only created implicitly to contain other nodes).
    pub(crate) keys: Vec<String>,
    /// The complete set of types to which this type can be implicitly converted in order of
    /// parsing precedence, **including** itself.  For [`Stringy`] types, this will only contain
    /// `self`.
    pub(crate) all_descendants: HashSet<TypeId>,
    /// See [`Self::parseable_descendants`] for docs
    pub(crate) parseable_descendants: Vec<TypeId>,
    pub(crate) inner: TypeInner,
}

impl Type {
    /// The descendant types of this type which are either a [`TypeInner::Pattern`] or a
    /// [`TypeInner::Stringy`], listed in parsing precedence, **including** itself.  For
    /// [`Stringy`] types, this will only contain `self`.
    pub fn parseable_descendants(&self) -> &[TypeId] {
        self.parseable_descendants.as_slice()
    }

    pub fn inner(&self) -> &TypeInner {
        &self.inner
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }
}

#[derive(Debug, Clone)]
pub enum TypeInner {
    /// A [`Type`] which can't be instantiated, but can contain child nodes
    Container,
    /// The pattern describing which token sequences are valid instances of this [`Type`].
    Pattern(Pattern),
    /// A node which store a string value, editable by the user.  These nodes always correspond to
    /// precisely one token.
    ///
    /// Note that this is not limited to string literals: for example, identifiers and other
    /// literals also use this node type.  Accordingly, the definition `Stringy` type is extremely
    /// flexible to accommodate these different use cases.
    Stringy(Stringy),
}

/// A [`Type`] where the contents of each node is an arbitrary string (which can be edited
/// separately).  This is used for nodes like identifiers or any type of literal (strings, numbers,
/// etc.).
#[derive(Debug, Clone)]
pub struct Stringy {
    // TODO: make fields less public
    /// String appended before the escaped contents
    pub(crate) delim_start: String,
    /// String appended after the escaped contents
    pub(crate) delim_end: String,

    /// A [`Regex`] which all node **contents** must match.  This always starts and ends with `^`
    /// and `$` to force the engine to match the whole string.
    pub(crate) regexes: Option<Regexes>,
    /// Default **contents** of new nodes.  This must match `validity_regex`.
    pub(crate) default_content: String,

    pub(crate) escape_rules: Option<EscapeRules>,
    // TODO: Specify syntax highlighting group
}

impl Stringy {
    /// Return the [`Regex`] which the contents of `self` must match
    pub fn unanchored_content_regex(&self) -> Option<&Regex> {
        self.regexes.as_ref().map(|regexes| &regexes.unanchored)
    }

    /// Creates a canonical `display_str` for some contents.
    pub fn create_display_string(&self, contents: &str) -> String {
        let mut display_str = String::with_capacity(contents.len() + 20);
        display_str.push_str(&self.delim_start);
        for c in contents.chars() {
            // TODO: handle escaping
            display_str.push(c);
        }
        display_str.push_str(&self.delim_end);
        display_str
    }
}

/// The [`Regex`]es required to specify the valid strings of a [`Stringy`] node
#[derive(Debug, Clone)]
pub struct Regexes {
    pub(crate) unanchored: Regex,
    pub(crate) anchored_start: Regex,
    pub(crate) anchored_both: Regex,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EscapeRules {
    /// A non-empty string that all escape sequences must start with.  For example, in JSON strings
    /// this is `\`
    pub(crate) start_sequence: String,
    /// Maps escape sequences (to go after `start_sequence`) to the de-escaped [`String`].  For
    /// example, for JSON strings this is:
    /// ```text
    /// `\` -> '\\' (i.e. `\\` de-escapes to `\`)
    /// `"` -> '"'  (i.e. `\"` de-escapes to `"`)
    /// `/` -> '/'
    /// `n` -> '\n'
    /// `t` -> '\t'
    /// `b` -> '\u{8}'
    /// `f` -> '\u{c}'
    /// `r` -> '\r'
    /// ```
    pub(crate) rules: BiMap<String, String>,
    /// The prefix which takes 4 hex symbols and de-escapes them to that unicode code-point.  For
    /// example, in JSON strings this is `u` (i.e. `\uABCD` would turn into the unicode code point
    /// `0xABCD`).
    pub(crate) unicode_hex_4: Option<String>,
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

#[derive(Clone)]
pub struct Token {
    text: String,
    // TODO: Syntax highlighting groups
}

impl Token {
    pub fn new(text: String) -> Self {
        Self { text }
    }

    #[inline]
    pub fn text(&self) -> &str {
        &self.text
    }
}

impl Debug for Token {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Token({})", self.text)
    }
}

#[derive(Debug, Clone)]
pub struct Whitespace {
    /// Which [`char`]s are considered 'whitespace'
    chars: CharSet,
}

impl Whitespace {
    /// Returns `true` if `c` should be considered whitespace
    pub fn is(&self, c: char) -> bool {
        self.chars.contains(c)
    }

    pub fn sampler(&self) -> char_set::Sampler {
        self.chars.sampler()
    }
}

impl From<CharSet> for Whitespace {
    fn from(chars: CharSet) -> Self {
        Self { chars }
    }
}

//////////////////
// HELPER TYPES //
//////////////////

// TODO: Tag these indices with which `Grammar` created them
index_vec::define_index_type! { pub struct TypeId = usize; }
index_vec::define_index_type! { pub struct TokenId = usize; }

pub type TypeVec<T> = IndexVec<TypeId, T>;
