/// An iterator similar to [`CharIndices`](std::str::CharIndices) which is heavily geared towards
/// tokenizing.
#[derive(Debug, Clone)]
pub(super) struct StrIter<'s> {
    /// The number of bytes of string that have been consumed so far
    consumed_length: usize,
    /// The string segment that's left to be consumed
    str_left: &'s str,
}

impl<'s> StrIter<'s> {
    pub fn new(s: &'s str) -> Self {
        Self {
            consumed_length: 0,
            str_left: s,
        }
    }

    /// Consumes [`char`]s from `self` until the predicate returns false or the end of the source
    /// string is reached.  This returns the [`str`] slice which contained these chars.
    pub fn take_chars_while(&mut self, mut p: impl FnMut(char) -> bool) -> &'s str {
        // Repeatedly consume chars which satisfy `p`
        let mut char_iter = self.str_left.char_indices().peekable();
        while char_iter.next_if(|(_idx, ch)| p(*ch)).is_some() {}
        // Split `self` at the point where `char_iter` has finished iterating
        let len_consumed = char_iter
            .next()
            .map_or(self.str_left.len(), |(idx, _ch)| idx);
        let (str_consumed, str_left) = self.str_left.split_at(len_consumed);
        self.consumed_length += len_consumed;
        self.str_left = str_left;
        str_consumed
    }

    /// If the remainder of `self` starts with `s`, consume it and return `true`.  Otherwise, do
    /// nothing and return `false`.
    #[must_use]
    pub fn eat(&mut self, s: &str) -> bool {
        let starts_with_s = self.str_left.starts_with(s);
        if starts_with_s {
            self.consumed_length += s.len();
            self.str_left = &self.str_left[s.len()..];
        }
        starts_with_s
    }

    /// The number of bytes of `self` which have been consumed
    pub fn consumed_length(&self) -> usize {
        self.consumed_length
    }

    /// Returns `true` if `self` has no more string to consume
    pub fn is_done(&self) -> bool {
        self.str_left.is_empty()
    }
}
