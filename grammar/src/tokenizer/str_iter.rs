use super::BadUnicodeEscapeError;
use std::str::CharIndices;

/// An iterator similar to [`CharIndices`](std::str::CharIndices), but providing utility methods
/// (like [`eat`](StrIter::eat)) which are used heavily while tokenizing.
// TODO: I think we can implement this with only one state
#[derive(Debug, Clone)]
pub(super) struct StrIter<'s> {
    source_str: &'s str,
    /// The number of bytes consumed before `char_iter` started
    len_before_current_char_iter: usize,
    char_iter: CharIndices<'s>,
}

impl<'s> StrIter<'s> {
    pub fn new(s: &'s str) -> Self {
        Self {
            source_str: s,
            len_before_current_char_iter: 0,
            char_iter: s.char_indices(),
        }
    }

    /// Pops the next [`char`] from `self`, along with its index in the source string
    pub fn next_char(&mut self) -> Option<(usize, char)> {
        self.char_iter
            .next()
            .map(|(idx, ch)| (idx + self.len_before_current_char_iter, ch))
    }

    /// Consumes [`char`]s from `self` until the predicate returns false or the end of the source
    /// string is reached.  This returns the [`str`] slice which contained these chars.
    pub fn take_chars_while(&mut self, mut p: impl FnMut(char) -> bool) -> &'s str {
        let initial_len_consumed = self.consumed_length();
        // Consume the chars on a **copy** of `self.char_iter`
        let mut peekable = self.char_iter.clone().peekable();
        while peekable.next_if(|(_idx, ch)| p(*ch)).is_some() {}
        let absolute_idx_of_last_char =
            peekable.next().map_or(self.source_str.len(), |(idx, _ch)| {
                self.len_before_current_char_iter + idx
            });
        self.eat_len(absolute_idx_of_last_char - initial_len_consumed)
            .unwrap() // We can't out-of-range, because we've already consumed these chars
    }

    /// If the remainder of `self` starts with `s`, consume it and return `true`.  Otherwise,
    /// return `false` without modifying `self`.
    #[must_use]
    pub fn eat(&mut self, s: &str) -> bool {
        if !self.str_remaining().starts_with(s) {
            return false;
        }
        self.eat_len(s.len()).unwrap();
        true
    }

    /// Moves `self` on by `len` bytes.
    ///
    /// Returns `None` if either `self` doesn't have this many bytes left to consume, or `len`
    /// bytes doesn't land on the boundary between two UTF-8 code-points.
    #[must_use]
    pub fn eat_len(&mut self, len: usize) -> Option<&'s str> {
        let str_consumed = self.str_remaining().get(..len)?;
        let new_str_remaining = self.str_remaining().get(len..)?;

        self.char_iter = new_str_remaining.char_indices();
        self.len_before_current_char_iter = self.source_str.len() - new_str_remaining.len();
        Some(str_consumed)
    }

    /// Attempts to parse the next `len` bytes as a number in a given `radix`.
    pub fn eat_numeral(&mut self, len: usize, radix: u32) -> Result<char, BadUnicodeEscapeError> {
        // Read the next `len` bytes
        let radix_str = self
            .eat_len(len)
            .ok_or(BadUnicodeEscapeError::SliceFailed)?;
        // Read them as a `u32` in the appropriate radix
        let codepoint_u32 = u32::from_str_radix(radix_str, radix)
            .map_err(|e| BadUnicodeEscapeError::IntParse(radix_str.to_owned(), e))?;
        // Generate a `char` with that code-point
        let codepoint_char = char::from_u32(codepoint_u32)
            .ok_or(BadUnicodeEscapeError::NoSuchChar(codepoint_u32))?;
        Ok(codepoint_char)
    }

    /// The number of bytes of `self` which have been consumed
    pub fn consumed_length(&self) -> usize {
        self.source_str.len() - self.char_iter.as_str().len()
    }

    /// Returns `true` if `self` has no more string to consume
    pub fn is_done(&self) -> bool {
        self.str_remaining().is_empty()
    }

    /// The full string slice being tokenized
    pub fn source_str(&self) -> &'s str {
        self.source_str
    }

    /// The string slice yet to be consumed
    pub fn str_remaining(&self) -> &'s str {
        self.char_iter.as_str()
    }
}
