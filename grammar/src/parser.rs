use std::rc::Rc;

use crate::{
    tokenizer::{self, ParsedToken, Tokenizer},
    Grammar, PatternElement, TokenId, TypeId, TypeInner,
};

/// `true` if this module should emit debug printing
const DO_DEBUG_PRINT: bool = false;

macro_rules! dbg_println {
    ($s: literal $(, $arg: expr)* ) => {
        if DO_DEBUG_PRINT {
            println!($s $(, $arg)*);
        }
    };
}

type TokenIter<'a, 's> = std::iter::Peekable<std::slice::Iter<'a, (ParsedToken<'s>, &'s str)>>;

pub trait Ast: Sized {
    type Builder: Builder<Node = Self>;

    fn new_stringy(type_id: TypeId, contents: String, display_str: String, ws: &str) -> Self;
}

// TODO: Not Debug
pub trait Builder: Sized + std::fmt::Debug {
    type Node: Ast;

    fn new(type_id: TypeId) -> Self;

    /// Add a child [`Node`](Self::Node) to the [`Node`](Self::Node) being built
    fn add_node(&mut self, type_bound: TypeId, node: Rc<Self::Node>);
    /// Add a static token (and its whitespace) to the [`Node`](Self::Node) being built
    fn add_token(&mut self, token: TokenId, ws: &str);

    /// Start a repeating sequence of sub-patterns
    fn seq_start(&mut self);
    /// Move on to the next sub-pattern in a sequence, adding a delimiter in the process
    fn seq_delim(&mut self, token: TokenId, ws: &str);
    /// Finish a repeating sequence of sub-patterns
    fn seq_end(&mut self);

    /// Build this `Builder` into a corresponding [`Node`](Self::Node)
    fn into_node(self) -> Self::Node;
}

/// Parse a string into an AST node who's [`Type`] is a descendant of a given [`TypeId`], returning
/// an error if that isn't possible.
pub(crate) fn parse<'s, N: Ast>(
    grammar: &Grammar,
    type_id: TypeId,
    s: &'s str,
) -> Result<(&'s str, N), Error> {
    // TODO: Check for & handle grammars with left-recursion

    let (leading_ws, tokenizer) = Tokenizer::new(grammar, s);
    let tokens = tokenizer
        .collect::<Result<Vec<_>, _>>()
        .map_err(Error::Tokenize)?;
    for t in &tokens {
        dbg_println!("    {:?}", t);
    }

    let tree = parse_type_bound::<N>(grammar, type_id, &mut tokens.iter().peekable())
        .ok_or(Error::Parse)?;
    Ok((leading_ws, tree))
}

/// The different ways that parsing could fail
#[derive(Debug, Clone)]
pub enum Error {
    Tokenize(tokenizer::Error),
    Parse,
}

/// Parse a token stream into an AST node who's [`Type`] is a descendant of a given [`TypeId`],
/// returning an error if that isn't possible.
fn parse_type_bound<'a, 's, N: Ast>(
    grammar: &Grammar,
    type_bound: TypeId,
    tokens: &mut TokenIter<'a, 's>,
) -> Option<N> {
    let ty = grammar.get_type(type_bound);

    // Try to parse each descendant type
    for &descendant_id in &ty.descendants {
        // TODO: Handle left-recursion
        //
        // We parse each descendant type on a copy of `tokens` so that, in the case of failure, the
        // next type can be parsed from the same start point.
        let mut tokens_clone = tokens.clone();
        if let Some(node) = parse_concrete_type::<N>(grammar, descendant_id, &mut tokens_clone) {
            *tokens = tokens_clone; // This has parsed correctly, so consume its tokens
            return Some(node);
        }
        // If parsing failed, then try other parse rules
    }

    // If all the descendant types failed to parse, then this node's parsing fails
    None
}

/*
/// Parse a token stream into **shortest** (as in fewest tokens) AST node with a concrete [`Type`].
///
/// This must always terminate, even when given a left-recursive (unambiguous) grammar.  Proof
/// sketch:
///  - Rules which apply left-recursion are always longer than their left-child node
///  - The left-child of a left-recursive rule must have the same type as its parent
/// => If a left-recursive node exists, then its left-child must be smaller and be of the same
///    type
/// => The left-recursive node is not the shortest node of its type given its start point (because
///    its left-child is shorter)
/// => This function can't return a left-recursive node
/// => This function doesn't need to consider left-recursive rules
/// => Termination of this function is independent of left-recursion
fn parse_shortest<'a, 's, N: Ast>(

TODO: Handle left-recursion somehow
*/

/// Attempt to parse a node of a concrete type
fn parse_concrete_type<'a, 's, N: Ast>(
    grammar: &Grammar,
    concrete_type_id: TypeId,
    tokens: &mut TokenIter<'a, 's>,
) -> Option<N> {
    let ty = grammar.get_type(concrete_type_id);

    dbg_println!("Parsing concrete type {:?}", ty.name);
    match &ty.inner {
        TypeInner::Container => {} // Containers can't be parsed as concrete types
        TypeInner::Pattern(pat) => {
            let mut bdr = N::Builder::new(concrete_type_id);
            if parse_pattern::<N>(grammar, &mut bdr, pat, tokens).is_some() {
                return Some(bdr.into_node());
            }
        }
        TypeInner::Stringy(_) => {
            if let Some((ParsedToken::Stringy(id, contents, display_str), ws)) = tokens.next() {
                if *id == concrete_type_id {
                    return Some(N::new_stringy(
                        concrete_type_id,
                        contents.clone(),
                        (*display_str).to_owned(),
                        ws,
                    ));
                }
            }
        }
    }
    dbg_println!("Parsing concrete type {:?} failed", ty.name);
    // If a match wasn't returned, then this type fails to parse
    None
}

/// Parse a full [`Pattern`] via a [`Self::Builder`].  This returns `Option<()>` (as opposed to a
/// [`bool`]) so that the `?` operator can be used.
#[must_use]
fn parse_pattern<'a, 's, N: Ast>(
    grammar: &Grammar,
    bdr: &mut N::Builder,
    pat: &[PatternElement],
    tokens: &mut TokenIter<'a, 's>,
) -> Option<()> {
    dbg_println!("Matching {:?}", pat);
    for elem in pat {
        dbg_println!("Elem: {:?}", elem);
        match elem {
            // If the next element is a static token, then check for the corresponding
            // token
            PatternElement::Token(expected_token_id) => match tokens.next() {
                Some((ParsedToken::Static(token_id), ws)) => {
                    if expected_token_id != token_id {
                        return None;
                    }
                    // If we got the token we expected, then consume it and keep parsing
                    bdr.add_token(*token_id, ws);
                }
                _ => return None,
            },
            // If the next element is a type bound, then attempt to parse a node with that type and
            // add it to the builder
            PatternElement::Type(type_bound) => {
                let node = parse_type_bound::<N>(grammar, *type_bound, tokens)?;
                bdr.add_node(*type_bound, Rc::new(node));
            }
            // If the next element is a sequence, then repeatedly parse the patterns until one of
            // them doesn't end with the given delimiter.  Parsing fails if any of the patterns
            // fail
            PatternElement::Seq { pattern, delimiter } => {
                bdr.seq_start();
                loop {
                    dbg_println!("{{");
                    parse_pattern::<N>(grammar, bdr, pattern, tokens)?;
                    dbg_println!("}}");
                    // Match the delimiter, and continue the loop if it matches
                    if let Some((ParsedToken::Static(token_id), ws)) = tokens.peek() {
                        if token_id == delimiter {
                            bdr.seq_delim(*token_id, ws);
                            tokens.next();
                            continue; // Parse the next element in the sequence
                        }
                    }
                    // If matching the delimiter failed, then the sequence is over
                    bdr.seq_end();
                    break;
                }
            }
        }
    }
    // If we successfully matched all the elements, then parsing succeeded
    Some(())
}
