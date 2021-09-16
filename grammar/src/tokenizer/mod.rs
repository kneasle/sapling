mod str_iter;

use itertools::Itertools;

use crate::{Grammar, Stringy, TokenId, TypeId, TypeInner, Whitespace};

use self::str_iter::StrIter;

/// An [`Iterator`] over the tokens contained in a [`str`]ing slice.
#[derive(Debug, Clone)]
pub struct Tokenizer<'s, 'g> {
    /// **Invariant**: Between calls to `next`, `chars` will have just finished consuming a string
    /// of whitespace.
    iter: StrIter<'s>,
    whitespace: &'g Whitespace,
    tokens_decreasing: &'g [(String, TokenId)],
    stringy_types: Vec<(TypeId, &'g Stringy)>,
}

impl<'s, 'g> Tokenizer<'s, 'g> {
    /// Creates a new [`TokenIter`] which tokenizes a given [`str`]ing slice according to a
    /// [`Grammar`].
    pub fn new(grammar: &'g Grammar, string: &'s str) -> (&'s str, Self) {
        let mut iter = StrIter::new(string);
        // Consume the leading whitespace before creating `self`
        let whitespace = grammar.whitespace();
        let leading_whitespace = consume_whitespace(&mut iter, whitespace);
        let stringy_types = grammar
            .types
            .iter_enumerated()
            .filter_map(|(id, ty)| match &ty.inner {
                TypeInner::Stringy(s) => Some((id, s)),
                TypeInner::Pattern { .. } => None,
            })
            .collect_vec();

        let iter = Self {
            iter,
            whitespace,
            stringy_types,
            tokens_decreasing: grammar.static_tokens_decreasing(),
        };
        (leading_whitespace, iter)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum ParsedToken {
    Static(TokenId),
    Stringy(TypeId, String),
}

#[derive(Debug, Clone)]
pub enum Error {
    /// No token matches the string pattern following a given index
    NoToken { start_idx: usize },
    /// No end delimiter was found for the start delimiter of a stringy node starting at
    /// `start_idx`
    UnfinishedStringy { start_idx: usize, type_id: TypeId },
    /// An escape sequence was started but not followed by any legal value
    BadEscape { start_idx: usize, type_id: TypeId },
    /// An unicode escape sequence was started but not followed by a legal numeral
    BadUnicodeValue {
        start_idx: usize,
        type_id: TypeId,
        inner: BadUnicodeEscapeError,
    },
}

/// Reasons why parsing a unicode escape sequence could fail
#[derive(Debug, Clone)]
pub enum BadUnicodeEscapeError {
    /// Couldn't slice the correct number of bytes.  There are two root causes (which [`str`]'s API
    /// doesn't let me distinguish):
    /// - there wasn't enough input string left
    /// - the expected end of the bytes was not on the border between two UTF-8 code-points.
    SliceFailed,
    /// The string sliced correctly, but the bytes could not be parsed as an integer in the
    /// designated radix
    IntParse(String, std::num::ParseIntError),
    /// The number parsed correctly, but does not correspond to a unicode code-point
    NoSuchChar(u32),
}

////////////////////////////
// TOKENIZATION ITERATION //
////////////////////////////

impl<'s, 't> Iterator for Tokenizer<'s, 't> {
    type Item = Result<(ParsedToken, &'s str), Error>;

    fn next(&mut self) -> Option<Self::Item> {
        // A `TokenIter` always starts having just finished consuming whitespace so, if the string
        // is valid token string, the remainder of `chars` must start with a token.

        if self.iter.is_done() {
            return None;
        }

        // Test stringy tokens (string literal, numbers, identifiers, etc.) for the longest match
        let mut longest_token_match: Option<(StrIter, ParsedToken)> = None;
        for &(id, stringy_type) in &self.stringy_types {
            // Speculatively consume this token on a **copy** of `self.iter`, since we want to
            // test multiple stringy types starting from the same position.  If one of these
            // tokens becomes the longest match, then `self.iter` is set to the value of `iter`
            // for that token.  Incidentally, `StrIter` is very cheap to clone (being two
            // `usize`s) - it could be `Copy`, but copy semantics would be dangerous.
            let mut iter = self.iter.clone();
            let token = match eat_stringy(&mut iter, stringy_type, id) {
                Ok(Some(contents)) => ParsedToken::Stringy(id, contents), // The token was matched
                Ok(None) => continue,                                     // The token doesn't match
                Err(e) => return Some(Err(e)), // The token started matching, but caused the whole
                                               // tokenization pass to abort
            };

            // `longest_token_match` if this is the new longest match
            if Some(iter.consumed_length())
                > longest_token_match
                    .as_ref()
                    .map(|(iter, _)| iter.consumed_length())
            {
                longest_token_match = Some((iter, token));
            }
        }
        // The length of the longest stringy token (or `0` if none were matched)
        let longest_stringy_len = longest_token_match.as_ref().map_or(0, |(iter, _content)| {
            iter.consumed_length() - self.iter.consumed_length()
        });

        // Test static tokens.  Static tokens take precidence over stringy tokens, so we return
        // immediately if a static token at least as long as the longest stringy node.
        for (text, id) in self.tokens_decreasing {
            // We are only interested in the longest token, so if we've matched a stringy token
            // then we don't need to check any shorter static tokens.
            if text.len() < longest_stringy_len {
                break;
            }
            if self.iter.eat(text) {
                // If the string left to consume starts with this token's text, then return it.
                // This is the first token reached, which means it also must be the longest
                // matching token (because Tokenizer.tokens_decreasing is sorted by decreasing
                // order, and two unique tokens of the same length can't match the same string).
                let whitespace = consume_whitespace(&mut self.iter, self.whitespace);
                return Some(Ok((ParsedToken::Static(*id), whitespace)));
            }
        }

        // If no static tokens matched, then either:
        // - a stringy token was matched, and that becomes our token
        // or
        // - no stringy tokens were matched, and the tokenization fails (we've already checked for
        //   the end of the source string)
        match longest_token_match {
            Some((iter, token)) => {
                // Update `self.iter` to mark that this token has been consumed
                self.iter = iter;
                let whitespace = consume_whitespace(&mut self.iter, self.whitespace);
                Some(Ok((token, whitespace)))
            }
            None => Some(Err(Error::NoToken {
                start_idx: self.iter.consumed_length(),
            })),
        }
    }
}

fn consume_whitespace<'s>(iter: &mut StrIter<'s>, whitespace: &Whitespace) -> &'s str {
    iter.take_chars_while(|ch| whitespace.is(ch))
}

/// Eat as much of the [`StrIter`] as possible whilst still generating a valid
/// [`ParsedToken::Stringy`].
fn eat_stringy(
    iter: &mut StrIter<'_>,
    stringy: &Stringy,
    type_id: TypeId,
) -> Result<Option<String>, Error> {
    let start_idx = iter.consumed_length();
    match (
        stringy.delim_start.as_str(),
        stringy.delim_end.as_str(),
        &stringy.regex,
        &stringy.escape_rules,
    ) {
        // Cases with regexes but no delimiters/escapes can just be computed by running the
        // start-anchored regex on the remaining string.  This case will handle non-string literals
        // and identifiers.
        ("", "", Some(regexes), None) => {
            regexes
                .anchored_start
                .find(iter.str_remaining())
                .map(|r#match| {
                    // The regex matched, so there must be a token at this location
                    assert_eq!(r#match.start(), 0);
                    iter.eat_len(r#match.end()).unwrap();
                    Ok(r#match.as_str().to_owned())
                })
                .transpose()
        }
        // Beyond this case, delim_start and delim_end must be non-empty
        ("", _, _, _) | (_, "", _, _) => todo!(),
        // Cases with start/end delimiters and escape rules can be handled with a DFA-style state
        // machine.  This case will handle string literals.
        (delim_start, delim_end, None, Some(esc_rules)) => {
            if delim_end.chars().count() != 1 {
                // To do this properly, we should probably use the Regex library
                todo!();
            }
            let end_char = delim_end.chars().exactly_one().unwrap_or_else(|_| {
                todo!("We should do some regex magic for end delims with len > 1")
            });
            let esc_start_char = esc_rules
                .start_sequence
                .chars()
                .exactly_one()
                .unwrap_or_else(|_| {
                    todo!("We should do some regex magic for escape starts with len > 1")
                });

            // Consume the first token.  If it doesn't exist, then try other tokens
            if !iter.eat(delim_start) {
                return Ok(None);
            }
            // Consume the rest of the iterator char-by-char
            let mut contents = String::new();
            'outer: while let Some((idx, ch)) = iter.next_char() {
                // Check if we've finished matching the token.  The range will be inferred from
                // how much has been consumed by `iter`
                if ch == end_char {
                    return Ok(Some(contents));
                }
                // Check if we have an escape string
                if ch == esc_start_char {
                    // Consume explicit escapes if needed
                    for (escaped, content) in &esc_rules.rules {
                        if iter.eat(escaped) {
                            contents.push_str(content);
                            continue 'outer;
                        }
                    }
                    // Consume 4-digit hex unicode
                    if let Some(seq) = &esc_rules.unicode_hex_4 {
                        if iter.eat(seq) {
                            let deescaped_char = iter.eat_numeral(4, 16).map_err(|inner| {
                                Error::BadUnicodeValue {
                                    start_idx: idx,
                                    type_id,
                                    inner,
                                }
                            })?;
                            contents.push(deescaped_char);
                            continue 'outer;
                        }
                    }

                    // If no escape was found, then tokenization fails (we could probably note this
                    // error and continue, but for now, this is easiest).
                    return Err(Error::BadEscape {
                        start_idx: idx,
                        type_id,
                    });
                }
                // Otherwise, add this char to `contents`
                contents.push(ch);
            }
            // If we consumed the entire source string without finding an end delimiter, then
            // return an error
            Err(Error::UnfinishedStringy { start_idx, type_id })
        }
        // I don't know what language constructs would be handled by this case, but it's so easy to
        // handle that I've implemented it anyway
        (delim_start, delim_end, None, None) => {
            if !iter.eat(delim_start) {
                return Ok(None); // If the first delimiter isn't matched, try other tokens
            }
            // Search for the end delimiter in the string.  If it isn't found, then return an error
            match iter.str_remaining().find(delim_end) {
                Some(idx) => {
                    iter.eat_len(idx + delim_end.len()).unwrap(); // Consume this token
                    Ok(Some(iter.str_remaining()[..idx].to_owned())) // Return its contents
                }
                None => Err(Error::UnfinishedStringy { start_idx, type_id }),
            }
        }
        // We don't need any other cases yet
        _ => todo!(),
    }
}
