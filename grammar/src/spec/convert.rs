use std::collections::{HashMap, HashSet};

use index_vec::IndexSlice;
use itertools::Itertools;
use regex::Regex;

use super::SpecGrammar;
use crate::{grammar, Grammar, TypeId, TypeVec};

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
    let (types, type_map) = convert_types(types, &mut token_map)?;

    Ok(Grammar::new(
        type_map.get_root(&root_type)?,
        convert_whitespace(whitespace),
        types,
        token_map.into_vec(),
    ))
}

/// The possibly ways that parsing a [`Grammar`] can fail.
#[derive(Debug)]
pub enum ConvertError {
    Regex {
        type_name: String,
        regex: String,
        inner: regex::Error,
    },
    UnknownChildType {
        name: String,
        parent_name: String,
    },
    UnknownRootType(String),
    /// There is a cycle in the type dependency graph (i.e. a type which is a descendant of itself)
    TypeCycle(Vec<String>),
}

////////////////
// NODE TYPES //
////////////////

fn convert_types(
    types: HashMap<super::TypeName, super::Type>,
    token_map: &mut TokenMap,
) -> ConvertResult<(TypeVec<grammar::Type>, TypeMap)> {
    // Before generating types, assign all names to type IDs (because types may refer to child
    // types which appear after themselves in the HashMap iterator).  If we see a `TypeName` which
    // is not in the `TypeMap`, then we know it must be invalid and we can generate an error.
    let (types, type_map) = TypeMap::new(types);
    // Compute the descendants of each type
    let descendants = compute_type_descendants(&types, &type_map)?;
    if false {
        // Pretty print the descendants for each node
        for (id, desc) in descendants.iter_enumerated() {
            print!("{} -> [", types[id].0);
            let mut is_first_desc = true;
            for &desc_id in desc {
                if !is_first_desc {
                    print!(", ");
                }
                print!("{}", types[desc_id].0);
                is_first_desc = false;
            }
            println!("]");
        }
    }
    // Construct `grammar::Type`s for each `spec::Type`
    let types: TypeVec<grammar::Type> = types
        .into_iter()
        .zip(descendants)
        .map(|((name, t), descendants)| convert_type(t, name, descendants, token_map, &type_map))
        .collect::<Result<_, _>>()?;
    Ok((types, type_map))
}

fn convert_type(
    t: super::Type,
    name: String,
    descendants: HashSet<TypeId>,
    token_map: &mut TokenMap,
    type_map: &TypeMap,
) -> ConvertResult<grammar::Type> {
    let (key, mut keys, inner) = match t {
        super::Type::Pattern {
            key,
            keys,
            children: _, // Already been used to compute descendants
            pattern,
        } => {
            let inner = grammar::TypeInner::Pattern {
                descendants,
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
            validity_regex,

            escape_rules,
        } => {
            assert!(stringy); // stringy should always be set to `true`

            let validity_regex = validity_regex
                // Compile two copies of the regex
                .map(|regex_str| {
                    // Generate anchored regex strings.
                    let str_anchor_start = format!("^(?x: {} )", regex_str);
                    let str_anchor_both = format!("^(?x: {} )$", regex_str);

                    // Compile regexes
                    let anchored_start =
                        Regex::new(&str_anchor_start).map_err(|inner| ConvertError::Regex {
                            type_name: name.to_owned(),
                            regex: str_anchor_start,
                            inner,
                        })?;
                    let anchored_both =
                        Regex::new(&str_anchor_both).map_err(|inner| ConvertError::Regex {
                            type_name: name.to_owned(),
                            regex: str_anchor_both,
                            inner,
                        })?;

                    Ok(grammar::Regexes {
                        anchored_start,
                        anchored_both,
                    })
                })
                // Convert the `Option<Result<R, E>>` into a `Result<Option<R>, E>`
                .transpose()?;
            let inner = grammar::Stringy {
                delim_start,
                delim_end,
                regex: validity_regex,
                default_content,
                escape_rules,
            };
            (key, keys, grammar::TypeInner::Stringy(inner))
        }
    };

    // Flatten they `key` and `keys` values into one list
    keys.extend(key);
    // Construct type and return
    Ok(grammar::Type { name, keys, inner })
}

//////////////////////
// TYPE DESCENDANTS //
//////////////////////

/// For every [`Type`], compute a [`HashSet`] of its descendants (i.e. concrete types into which it
/// can be converted).  This also checks for cycles
fn compute_type_descendants(
    types: &IndexSlice<TypeId, [(super::TypeName, super::Type)]>,
    type_map: &TypeMap,
) -> ConvertResult<TypeVec<HashSet<TypeId>>> {
    // For each TypeId, determine the `TypeId`s of its children
    let child_type_ids: TypeVec<Vec<TypeId>> = types
        .iter()
        .map(|(parent_name, ty)| match ty {
            super::Type::Pattern { children, .. } => children
                .iter()
                .map(|child_name| type_map.get(child_name, parent_name))
                .collect::<ConvertResult<Vec<TypeId>>>(),
            super::Type::Stringy { .. } => Ok(Vec::new()), // Stringy nodes have no children
        })
        .collect::<Result<_, _>>()?;
    // For each node flatten its descendant tree, terminating if any cycles are found.
    //
    // PERF: This does a lot of duplicated work expanding nodes, which could be sped up by
    // memoising the results of `enumerate_type_descendants`.  However, I don't expect the type
    // hierarchies to be that large, so I think the cleaner but slower code is preferable
    types
        .iter_enumerated()
        .map(|(id, _type)| {
            let mut type_stack = Vec::<TypeId>::new(); // This will store which types are being
                                                       // expanded further up the call stack, used
                                                       // for detecting cycles
            let mut descendants = HashSet::<TypeId>::new();
            enumerate_type_descendants(
                id,
                &types,
                &child_type_ids,
                &mut type_stack,
                &mut descendants,
            )?;
            assert!(type_stack.is_empty());
            Ok(descendants)
        })
        .collect()
}

/// Enumerate the descendants of a type by recursively walking its child types, inserting the
/// results into `out`.  This also detects cycles in this graph (i.e. a type which is its own
/// descendant), and creates the appropriate error messages.
fn enumerate_type_descendants(
    id: TypeId,
    types: &IndexSlice<TypeId, [(super::TypeName, super::Type)]>,
    child_type_ids: &IndexSlice<TypeId, [Vec<TypeId>]>,
    type_stack: &mut Vec<TypeId>,
    out: &mut HashSet<TypeId>,
) -> ConvertResult<()> {
    // Check for cycles
    if let Some(idx) = type_stack.iter().position(|&i| i == id) {
        // `type_stack[idx..] + id` forms the cycle (i.e. a cycle which starts and ends with `id`
        let cycle = type_stack[idx..]
            .iter()
            .chain(std::iter::once(&id))
            .map(|&id| types[id].0.to_owned())
            .collect_vec();
        return Err(ConvertError::TypeCycle(cycle));
    }
    // Mark this type as a descendant
    out.insert(id);
    // Recurse over this type's children
    type_stack.push(id);
    for &child_id in &child_type_ids[id] {
        enumerate_type_descendants(child_id, types, child_type_ids, type_stack, out)?;
    }
    assert_eq!(type_stack.pop(), Some(id));
    Ok(())
}

/////////////////////////
// PATTERNS/WHITESPACE //
/////////////////////////

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

    use crate::{spec, Token, TokenId, TypeId, TypeVec};

    use super::{ConvertError, ConvertResult};

    /// Maps [`TypeName`]s to [`TypeId`]s, providing `get` methods which generate error messages
    #[derive(Debug, Clone)]
    pub(super) struct TypeMap {
        inner: HashMap<spec::TypeName, TypeId>,
    }

    impl TypeMap {
        pub(super) fn new(
            types: HashMap<spec::TypeName, spec::Type>,
        ) -> (TypeVec<(spec::TypeName, spec::Type)>, Self) {
            let mut inner_map = HashMap::new();
            let types = types
                .into_iter()
                .enumerate()
                .map(|(idx, (name, ty))| {
                    inner_map.insert(name.clone(), TypeId::new(idx));
                    (name, ty)
                })
                .collect();
            (types, Self { inner: inner_map })
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
