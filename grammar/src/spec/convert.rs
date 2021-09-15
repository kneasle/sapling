use index_vec::IndexVec;
use regex::Regex;

use super::SpecGrammar;
use crate::{grammar, Grammar, TypeId};

pub type ConvertResult<T> = Result<T, ConvertError>;

use self::utils::{TokenMap, TypeMap};

/// Convert a [`SpecGrammar`] (likely parsed from a TOML file) into a full [`Grammar`], or fail
/// with a [`ConvertError`].  This is a largely straightforward process, since the 'shapes' of
/// [`SpecGrammar`] and [`Grammar`] are (intentionally) similar.
pub(crate) fn convert(grammar: SpecGrammar) -> ConvertResult<Grammar> {
    let SpecGrammar {
        root_type,
        whitespace,
        types,
    } = grammar;

    // Tokens will be added to this table as discovered
    let mut token_map = TokenMap::new();
    // Before generating types, assign all names to type IDs (because types may refer to child
    // types which appear after themselves in the HashMap iterator).
    let type_map = TypeMap::new(types.keys().cloned());
    let types: IndexVec<TypeId, grammar::Type> = types
        .into_iter()
        .map(|(name, t)| convert_type(t, name, &mut token_map, &type_map))
        .collect::<Result<_, _>>()?;

    Ok(Grammar::new(
        type_map.get_root(&root_type)?,
        convert_whitespace(whitespace),
        types,
        token_map.into_vec(),
    ))
}

/// The possibly ways that conversion from [`SpecGrammar`] to [`Grammar`] could fail.
#[derive(Debug)]
pub enum ConvertError {
    Regex {
        type_name: String,
        inner: regex::Error,
    },
    UnknownChildType {
        name: String,
        parent_name: String,
    },
    UnknownRootType(String),
}

fn convert_type(
    t: super::Type,
    name: String,
    token_map: &mut TokenMap,
    type_map: &TypeMap,
) -> ConvertResult<grammar::Type> {
    let (key, mut keys, inner) = match t {
        super::Type::Pattern {
            key,
            keys,
            children,
            pattern,
        } => {
            // Look up `TypeId`s of all child types
            let child_type_ids = children
                .iter()
                .map(|child_name| type_map.get(child_name, &name))
                .collect::<Result<_, _>>()?;
            let inner = grammar::TypeInner::Pattern {
                child_types: child_type_ids,
                pattern: match pattern {
                    Some(p) => Some(compile_pattern(p, &name, token_map, type_map)?),
                    None => None,
                },
            };
            (key, keys, inner)
        }

        super::Type::Stringy {
            key,
            keys,
            stringy,

            delim_start,
            delim_end,

            default_content,
            mut validity_regex,
            escape_rules,
            unicode_escape_prefix,
        } => {
            assert!(stringy); // stringy should always be set to `true`

            // Add `^` and `$` to either end of the regex, to force the regex engine to match the
            // entire node contents
            validity_regex.insert(0, '^');
            validity_regex.push('$');
            let validity_regex =
                Regex::new(&validity_regex).map_err(|inner| ConvertError::Regex {
                    type_name: name.to_owned(),
                    inner,
                })?;
            let inner = full::Stringy {
                delim_start,
                delim_end,
                validity_regex,
                default_content,
                escape_rules,
                unicode_escape_prefix,
                // deescape_rules: (),
            };
            (key, keys, grammar::TypeInner::Stringy(inner))
        }
    };

    // Flatten they `key` and `keys` values into one list
    keys.extend(key);
    // Construct type and return
    Ok(grammar::Type { name, keys, inner })
}

fn compile_pattern(
    elems: super::Pattern,
    parent_type_name: &str,
    token_map: &mut TokenMap,
    type_map: &TypeMap,
) -> ConvertResult<grammar::Pattern> {
    elems
        .into_iter()
        .map(|e| compile_pattern_element(e, parent_type_name, token_map, type_map))
        .collect::<Result<_, _>>()
}

fn compile_pattern_element(
    elem: super::PatternElement,
    parent_type_name: &str,
    token_map: &mut TokenMap,
    type_map: &TypeMap,
) -> ConvertResult<grammar::PatternElement> {
    use super::PatternElement::*;
    use grammar::PatternElement as PE;
    Ok(match elem {
        Token(name) => PE::Token(token_map.get_id(name)),
        Seq { pattern, delimiter } => PE::Seq {
            pattern: compile_pattern(pattern, parent_type_name, token_map, type_map)?,
            delimiter: token_map.get_id(delimiter),
        },
        Type { name } => PE::Type(type_map.get(&name, parent_type_name)?),
    })
}

fn convert_whitespace(whitespace: super::Whitespace) -> grammar::Whitespace {
    match whitespace {
        super::Whitespace::Chars(string) => {
            grammar::Whitespace::from_chars(string.chars().collect())
        }
    }
}

mod utils {
    use std::collections::HashMap;

    use index_vec::IndexVec;

    use crate::{spec, Token, TokenId, TypeId};

    use super::{ConvertError, ConvertResult};

    /// Maps [`TypeName`]s to [`TypeId`]s, providing `get` methods which generate error messages
    #[derive(Debug, Clone)]
    pub(super) struct TypeMap {
        inner: HashMap<spec::TypeName, TypeId>,
    }

    impl TypeMap {
        pub(super) fn new(names: impl IntoIterator<Item = spec::TypeName>) -> Self {
            let mut next_id = TypeId::new(0);
            let inner_map = names
                .into_iter()
                .map(|name| {
                    let id = next_id;
                    next_id += 1;
                    (name, id)
                })
                .collect::<HashMap<_, _>>();
            Self { inner: inner_map }
        }

        pub(super) fn get(&self, name: &str, parent_type_name: &str) -> ConvertResult<TypeId> {
            self.inner
                .get(name)
                .copied()
                .ok_or_else(|| ConvertError::UnknownChildType {
                    name: name.to_owned(),
                    parent_name: parent_type_name.to_owned(),
                })
        }

        pub(super) fn get_root(&self, name: &str) -> ConvertResult<TypeId> {
            self.inner
                .get(name)
                .copied()
                .ok_or_else(|| ConvertError::UnknownRootType(name.to_owned()))
        }
    }

    /// A mapping from string representations of tokens to the corresponding [`TokenId`]s.
    #[derive(Debug, Clone, Default)]
    pub(super) struct TokenMap {
        str_to_id: HashMap<String, TokenId>,
        tokens: IndexVec<TokenId, Token>,
    }

    impl TokenMap {
        pub(super) fn new() -> Self {
            Self::default()
        }

        /// Gets the [`TokenId`] corresponding to the token representing the given string.
        pub(super) fn get_id(&mut self, v: String) -> TokenId {
            match self.str_to_id.get(&v).copied() {
                Some(id) => id,
                None => {
                    let new_id = self.tokens.push(Token::new(v.clone()));
                    self.str_to_id.insert(v, new_id);
                    new_id
                }
            }
        }

        /// Yields an [`IndexVec`] which corresponds to the inverse of this [`TokenMap`] (i.e. maps
        /// [`TokenId`]s back to [`Token`]s).
        pub(super) fn into_vec(self) -> IndexVec<TokenId, Token> {
            self.tokens
        }
    }
}
