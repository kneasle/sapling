mod str_iter;

use std::sync::Arc;

use itertools::Itertools;

use crate::{Grammar, TokenId, Whitespace};

use self::str_iter::StrIter;

/// A pre-compiled tokenizer which can tokenize arbitrary strings into a set of arbitrary tokens.
#[derive(Debug, Clone)]
pub struct Tokenizer {
    grammar: Arc<Grammar>,
    /// Mapping from token texts to IDs, stored **in decreasing order** of the text length.  This
    /// makes sure that the tokenizer always consumes the largest possible token (e.g. `"&&"`
    /// should be tokenized into just `&&`, rather than two `&`s).
    tokens_decreasing: Vec<(String, TokenId)>,
}

impl Tokenizer {
    pub fn new(grammar: Arc<Grammar>) -> Self {
        let mut tokens_decreasing = grammar
            .tokens()
            .iter_enumerated()
            .map(|(id, token)| (token.text().to_owned(), id))
            .collect_vec();
        tokens_decreasing.sort_by_key(|(name, _id)| std::cmp::Reverse(name.len()));
        Self {
            grammar,
            tokens_decreasing,
        }
    }

    /// Returns a [`TokenIter`] which will yield the tokens contained in a given [`str`]ing.  Since
    /// whitespace is always 'owned' by the token which precedes it, the leading whitespace can't
    /// be owned by a token and is returned separately.
    pub fn token_iter<'s, 't>(&'t self, string: &'s str) -> (&'s str, TokenIter<'s, 't>) {
        let mut iter = StrIter::new(string);
        // Consume the leading whitespace before creating `self`
        let whitespace = self.grammar.whitespace();
        let leading_whitespace = consume_whitespace(&mut iter, whitespace);

        let iter = TokenIter {
            iter,
            whitespace,
            tokens_decreasing: &self.tokens_decreasing,
        };
        (leading_whitespace, iter)
    }
}

/// An [`Iterator`] over the tokens contained in a [`str`]ing slice.
#[derive(Debug, Clone)]
pub struct TokenIter<'s, 't> {
    /// **Invariant**: Between calls to `next`, `chars` will have just finished consuming a string
    /// of whitespace.
    iter: StrIter<'s>,
    whitespace: &'t Whitespace,
    tokens_decreasing: &'t [(String, TokenId)],
}

impl<'s, 't> Iterator for TokenIter<'s, 't> {
    type Item = Result<(TokenId, &'s str), Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.iter.is_done() {
            return None;
        }
        // A `TokenIter` always starts having just finished consuming whitespace, so the remainder
        // of `chars` must start with a token.
        for (name, id) in self.tokens_decreasing {
            if self.iter.eat(name) {
                // If the string left to consume starts with this token's text, then return it.
                // This is the first token reached, which means it also must be (one of) the
                // longest possible tokens (because Tokenizer.tokens_decreasing is sorted by
                // decreasing order).
                let whitespace = consume_whitespace(&mut self.iter, self.whitespace);
                return Some(Ok((*id, whitespace)));
            }
        }
        // TODO: Handle variable tokens, like identifiers and literals

        // If no identifiers have matched and this is not the end of the string, then there is an
        // error.
        Some(Err(Error::NoToken {
            start_idx: self.iter.consumed_length(),
        }))
    }
}

#[derive(Debug, Clone)]
pub enum Error {
    /// No token matches the string pattern following a given index
    NoToken { start_idx: usize },
}

fn consume_whitespace<'s>(iter: &mut StrIter<'s>, whitespace: &Whitespace) -> &'s str {
    iter.take_chars_while(|ch| whitespace.is(ch))
}
