use std::collections::{HashMap, HashSet};

use index_vec::IndexSlice;
use itertools::Itertools;
use regex::Regex;
use regex_syntax::hir;

use super::SpecGrammar;
use crate::{char_set, grammar, Grammar, TypeId, TypeVec};

pub type Result<T> = std::result::Result<T, Error>;

use self::utils::{TokenMap, TypeMap};

/// Convert a [`SpecGrammar`] (likely parsed from a TOML file) into a full [`Grammar`], or fail
/// with a [`ConvertError`].  This is a largely straightforward process, since the 'shapes' of
/// [`SpecGrammar`] and [`Grammar`] are (intentionally) similar.
pub(crate) fn convert(grammar: SpecGrammar) -> self::Result<Grammar> {
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
        convert_whitespace(whitespace)?,
        types,
        token_map.into_vec(),
    ))
}

/// The possibly ways that parsing a [`Grammar`] can fail.
#[derive(Debug)]
pub enum Error {
    /// An error occurred whilst generating a [`Regex`] provided by the user
    Regex {
        type_name: String,
        regex: String,
        inner: regex::Error,
    },
    /// An error occurred whilst compiling a set of chars (as a regex)
    CharSet {
        /// The `CharSet` string provided by the user
        source_str: String,
        /// The [`Regex`] string which was generated from `source`
        regex_str: String,
        inner: regex_syntax::Error,
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
) -> self::Result<(TypeVec<grammar::Type>, TypeMap)> {
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
    // Determine which concrete types can actually be parsed (i.e. aren't simply containers)
    let parseable_type_ids = types
        .iter_enumerated()
        .filter(|(_id, (_name, ty))| match ty {
            super::Type::Pattern { pattern, .. } => pattern.is_some(),
            super::Type::Stringy { .. } => true,
        })
        .map(|(id, _)| id)
        .collect::<HashSet<_>>();

    // Construct `grammar::Type`s for each `spec::Type`
    let types: TypeVec<grammar::Type> = types
        .into_iter()
        .zip(descendants)
        .map(|((name, t), descendants)| {
            convert_type(
                t,
                name,
                descendants,
                &parseable_type_ids,
                token_map,
                &type_map,
            )
        })
        .collect::<self::Result<_>>()?;
    Ok((types, type_map))
}

fn convert_type(
    t: super::Type,
    name: String,
    all_descendants: Vec<TypeId>,
    parseable_type_ids: &HashSet<TypeId>,
    token_map: &mut TokenMap,
    type_map: &TypeMap,
) -> self::Result<grammar::Type> {
    let (key, mut keys, inner) = match t {
        super::Type::Pattern {
            key,
            keys,
            children: _, // Already been used to compute descendants
            pattern,
        } => {
            let inner = match pattern {
                Some(p) => {
                    grammar::TypeInner::Pattern(compile_pattern(p, &name, token_map, type_map)?)
                }
                None => grammar::TypeInner::Container,
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
            assert!(stringy); // stringy should always be set to `true`.  TODO: Handle this more nicely

            let regexes = validity_regex
                // Compile two copies of the regex
                .map(|regex_str| convert_validity_regex(&regex_str, &name))
                // Use `?` on the `Result` inside the `Option`.  I.e. convert a
                // `Option<Result<T, E>>` to `Option<T>`, returning `Err(E)` if needed
                .transpose()?;
            let escape_rules = escape_rules.map(convert_escape_rules).transpose()?;

            let inner = grammar::Stringy {
                delim_start,
                delim_end,
                regexes,
                default_content,
                escape_rules,
            };
            (key, keys, grammar::TypeInner::Stringy(Box::new(inner)))
        }
    };

    // Flatten they `key` and `keys` values into one list
    keys.extend(key);
    // Construct type and return
    Ok(grammar::Type {
        name,
        keys,
        all_descendants: all_descendants.iter().copied().collect(),
        parseable_descendants: all_descendants
            .iter()
            .copied()
            .filter(|id| parseable_type_ids.contains(id))
            .collect_vec(),
        inner,
    })
}

fn convert_escape_rules(rules: super::EscapeRules) -> self::Result<grammar::EscapeRules> {
    let super::EscapeRules {
        start_sequence,
        rules,
        unicode_hex_4,
        dont_escape,
    } = rules;
    Ok(grammar::EscapeRules {
        start_sequence,
        rules,
        unicode_hex_4,
        dont_escape: convert_char_set(dont_escape)?,
    })
}

//////////////////////
// TYPE DESCENDANTS //
//////////////////////

/// For every [`Type`], compute a [`HashSet`] of its descendants (i.e. concrete types into which it
/// can be converted).  This also checks for cycles
fn compute_type_descendants(
    types: &IndexSlice<TypeId, [(super::TypeName, super::Type)]>,
    type_map: &TypeMap,
) -> self::Result<TypeVec<Vec<TypeId>>> {
    // For each TypeId, determine the `TypeId`s of its children
    let child_type_ids: TypeVec<Vec<TypeId>> = types
        .iter()
        .map(|(parent_name, ty)| match ty {
            super::Type::Pattern { children, .. } => children
                .iter()
                .map(|child_name| type_map.get(child_name, parent_name))
                .collect::<self::Result<Vec<TypeId>>>(),
            super::Type::Stringy { .. } => Ok(Vec::new()), // Stringy nodes have no children
        })
        .collect::<self::Result<_>>()?;
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
            let mut descendants = Vec::<TypeId>::new();
            enumerate_type_descendants(
                id,
                types,
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
    out: &mut Vec<TypeId>,
) -> self::Result<()> {
    // Check for cycles
    if let Some(idx) = type_stack.iter().position(|&i| i == id) {
        // `type_stack[idx..] + id` forms the cycle (i.e. a cycle which starts and ends with `id`
        let cycle = type_stack[idx..]
            .iter()
            .chain(std::iter::once(&id))
            .map(|&id| types[id].0.to_owned())
            .collect_vec();
        return Err(Error::TypeCycle(cycle));
    }
    // Mark this type as a descendant (if it hasn't been listed already)
    if !out.contains(&id) {
        out.push(id);
    }
    // Recurse over this type's children
    type_stack.push(id);
    for &child_id in &child_type_ids[id] {
        enumerate_type_descendants(child_id, types, child_type_ids, type_stack, out)?;
    }
    assert_eq!(type_stack.pop(), Some(id));
    Ok(())
}

//////////////
// PATTERNS //
//////////////

fn compile_pattern(
    elems: super::Pattern,
    parent_type_name: &str,
    token_map: &mut TokenMap,
    type_map: &TypeMap,
) -> self::Result<grammar::Pattern> {
    elems
        .into_iter()
        .map(|e| compile_pattern_element(e, parent_type_name, token_map, type_map))
        .collect::<self::Result<_>>()
}

fn compile_pattern_element(
    elem: super::PatternElement,
    parent_type_name: &str,
    token_map: &mut TokenMap,
    type_map: &TypeMap,
) -> self::Result<grammar::PatternElement> {
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

///////////////////////////////
// REGEX/WHITESPACE/CHAR SET //
///////////////////////////////

fn convert_validity_regex(regex_str: &str, type_name: &str) -> self::Result<grammar::Regexes> {
    macro_rules! compile_regex {
        ($string: expr) => {
            Regex::new(&$string).map_err(|inner| Error::Regex {
                type_name: type_name.to_owned(),
                regex: $string,
                inner,
            })?;
        };
    }

    let str_unanchored = format!("(?x: {} )", regex_str);
    let str_anchor_start = format!("^{}", str_unanchored);
    let str_anchor_both = format!("^{}$", str_unanchored);

    Ok(grammar::Regexes {
        unanchored: compile_regex!(str_unanchored),
        anchored_start: compile_regex!(str_anchor_start),
        anchored_both: compile_regex!(str_anchor_both),
    })
}

fn convert_whitespace(ws_chars: super::CharSet) -> self::Result<grammar::Whitespace> {
    convert_char_set(ws_chars).map(grammar::Whitespace::from)
}

fn convert_char_set(set: super::CharSet) -> self::Result<char_set::CharSet> {
    // Convert the source `CharSet` into the string for a regex which matches single `char`s from
    // the same set.
    let source_str = set.0;
    let regex_str = format!("[{}]", source_str);
    // Parse that regex
    let regex_hir = regex_syntax::Parser::new()
        .parse(&regex_str)
        .map_err(|inner| Error::CharSet {
            source_str,
            regex_str,
            inner,
        })?;
    // Extract the char ranges from the regex
    let unicode_ranges = match regex_hir.kind() {
        hir::HirKind::Class(hir::Class::Unicode(unicode_class)) => unicode_class.ranges(),
        _ => unreachable!("Regex shouldn't parse as anything other than a char class"),
    };
    Ok(unicode_ranges
        .iter()
        .map(|range| range.start()..=range.end())
        .collect())
}

mod utils {
    use std::collections::HashMap;

    use index_vec::IndexVec;

    use crate::{spec, Token, TokenId, TypeId, TypeVec};

    use super::Error;

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

        pub(super) fn get(&self, name: &str, parent_type_name: &str) -> super::Result<TypeId> {
            self.inner
                .get(name)
                .copied()
                .ok_or_else(|| Error::UnknownChildType {
                    name: name.to_owned(),
                    parent_name: parent_type_name.to_owned(),
                })
        }

        pub(super) fn get_root(&self, name: &str) -> super::Result<TypeId> {
            self.inner
                .get(name)
                .copied()
                .ok_or_else(|| Error::UnknownRootType(name.to_owned()))
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
