use std::{convert::identity, iter::FromIterator, ops::RangeInclusive};

use rand::Rng;

const FIRST_NON_ASCII_CHAR: char = '\u{128}';

/// An set of [`char`], optimised for sets where adjacent [`char`]s are very likely to either both
/// be included or both excluded.
#[derive(Debug, Clone)]
pub struct CharSet {
    /// A bit-mask for the first 128 code-points, i.e. ASCII values
    ascii_bitmask: u128,
    /// List of ranges of [`char`]s which this set includes.
    ///
    /// **Invariants**:
    /// - These are sorted and non-overlapping, i.e. the flattened sequence of starts/ends is
    ///   sorted in strictly increasing order.
    /// - No range can start below 256, since the first 256 code-points are handled by the
    ///   `ascii_bitmask`.
    /// Note that regions are not necessarily compacted (i.e. we could have `'\u{400}'..'\u{500}'`
    /// just before `'\u{501}'..'\u{600}'`).  This is because combining a single pair of ranges is
    /// a linear-time operation (in the number of ranges) - the same time complexity as combining
    /// all the possible adjacent pairs simultaneously.
    ranges: Vec<RangeInclusive<char>>,
    /// Cached value for the number of [`char`]s in `self`
    len: usize,
}

impl CharSet {
    /// Creates a `CharSet` which contains no [`char`]s.  Same as [`CharSet::none`].
    pub fn empty() -> Self {
        Self::none()
    }

    /// Creates a `CharSet` which contains no [`char`]s.
    pub fn none() -> Self {
        Self {
            ascii_bitmask: 0,
            ranges: vec![],
            len: 0,
        }
    }

    /// Creates a `CharSet` which contains every [`char`].
    pub fn all() -> Self {
        Self {
            ascii_bitmask: !0,
            ranges: vec![FIRST_NON_ASCII_CHAR..=char::MAX],
            len: char::MAX as usize + 1, // char::MAX + 1 because `'\0'` is included
        }
    }

    /// Returns `true` if this set contains a given [`char`].
    pub fn contains(&self, ch: char) -> bool {
        if let Some(bit_idx) = ascii_bit_idx(ch) {
            // Contained in the set if the `codepoint`th bit of the `ascii_bitmask` is set
            self.ascii_bitmask & (1 << bit_idx) != 0
        } else {
            match self.ranges.get(self.range_idx_including_char(ch)) {
                // We don't need to check the end range, since the binary search guarantees that
                // `ch <= range.end()` for this value of `range`
                Some(range) => *range.start() >= ch,
                // If `ch` is larger than the largest `range_end`, then it isn't contained
                None => false,
            }
        }
    }

    /// Inserts a [`char`] into this set.  Returns `true` if the value was already in the set.
    pub fn insert(&mut self, ch: char) -> bool {
        if let Some(bit_idx) = ascii_bit_idx(ch) {
            let bit = 1 << bit_idx;
            let is_contained = self.ascii_bitmask & bit != 0;
            if !is_contained {
                self.ascii_bitmask |= bit;
                self.len += 1;
            }
            is_contained
        } else {
            // When inserting into the ranges, we want to avoid a linear-time insertion wherever
            // possible.  Therefore, we try to extend either the left or right ranges to include
            // this `char`.  If this isn't possible, we have to fall back on the linear-time
            // insertion.
            //
            // PERF: We could potentially amortize this linear cost by storing these ranges as a
            // binary tree, or keeping a queue of single inserted chars.
            let range_idx = self.range_idx_including_char(ch);
            if let Some(r) = self.ranges.get_mut(range_idx) {
                // `(range before r).end() < ch <= r.end()`.  Check if `ch` is included in `r`
                if *r.start() <= ch {
                    return true;
                }
                // `(range before r).end() < ch < r.start()`.  Try to extend `r` to include `ch`
                if ch as u32 + 1 == *r.start() as u32 {
                    *r = ch..=*r.end();
                    self.len += 1;
                    return false;
                }
                drop(r);
                // `(range before r).end() < ch < r.start() - 1`.  Try to extend `(range before r)`
                // to include `ch`
                if let Some(range_idx_before_r) = range_idx.checked_sub(1) {
                    let range_before_r = &mut self.ranges[range_idx_before_r];
                    if *range_before_r.end() as u32 + 1 == ch as u32 {
                        *range_before_r = *range_before_r.start()..=ch;
                        self.len += 1;
                        return false;
                    }
                }
                // `(range before r).end() + 1 < ch < r.start() - 1`.  `ch` can't be merged into
                // either of `(range_before_r)` or `r`.
            }
            // `ch` can't be merged into any ranges, so insert a new range for it.  This sadly is a
            // linear-time operation
            self.ranges.insert(range_idx, ch..=ch);
            self.len += 1;
            false
        }
    }

    /// Insert all `char`s from a [`RangeInclusive`] into `self`
    pub fn insert_range(&mut self, range: RangeInclusive<char>) {
        let (ascii_range, non_ascii_range) = split_range(&range, FIRST_NON_ASCII_CHAR);
        if let Some(r) = ascii_range {
            // We want to set all the bits who's indices are `start <= idx <= end`
            let start = *r.start() as u32;
            let end = *r.end() as u32;

            // 1 bits up to but not including the `start`th bit
            let ones_up_to_start = (1u128 << start) - 1;
            // 1 bits up to and including the `end`th bit
            let ones_to_end = (1u128 << (end + 1)) - 1;
            // 1 bits from the `start`th bit to the `end`th bit.  These are the bits we want to set
            // in `self.ascii_bitmask`
            let ones_in_range = ones_to_end & !ones_up_to_start;
            // Set the bits and compute the new length
            let prev_num_ascii_chars = self.ascii_bitmask.count_ones() as usize;
            self.ascii_bitmask |= ones_in_range;
            let num_ascii_chars = self.ascii_bitmask.count_ones() as usize;
            self.len += num_ascii_chars - prev_num_ascii_chars; // Can't underflow, because the
                                                                // number of 1s can only have
                                                                // increased
        }
        if let Some(_r) = non_ascii_range {
            todo!()
        }
    }

    /// Removes a [`char`] from `self`, returning `true` if that [`char`] was in the set.
    #[deprecated(note = "Not yet implemented.")]
    pub fn remove(&mut self, _ch: char) -> bool {
        todo!()
    }

    pub fn sampler(&self) -> Sampler {
        Sampler::new(self)
    }

    /// Returns the index of the [`RangeInclusive`] `r` for which `ch <= r.end()`, or
    /// `self.ranges.len()` if ch is too big to fit into any range.
    fn range_idx_including_char(&self, ch: char) -> usize {
        self.ranges
            .binary_search_by_key(&ch, |range| *range.end())
            .map_or_else(identity, identity)
    }

    /// The number of [`char`]s in this [`CharSet`].  This is `O(1)` and compiles down to a single
    /// memory load.
    pub fn len(&self) -> usize {
        self.len
    }
}

impl FromIterator<char> for CharSet {
    fn from_iter<I: IntoIterator<Item = char>>(chars: I) -> Self {
        let mut set = CharSet::empty();
        for ch in chars {
            set.insert(ch);
        }
        set
    }
}

impl FromIterator<RangeInclusive<char>> for CharSet {
    fn from_iter<I: IntoIterator<Item = RangeInclusive<char>>>(ranges: I) -> Self {
        let mut set = CharSet::empty();
        for r in ranges {
            set.insert_range(r);
        }
        set
    }
}

/// Persistent state for uniformly sampling [`char`]s from a [`CharSet`]
#[derive(Debug, Clone)]
pub struct Sampler<'s> {
    /// The [`CharSet`] who's contents we're sampling.
    set: &'s CharSet,
    /// The [`char`]s contained within the `self.set.ascii_bitmask`
    ascii_values: Vec<char>,
    /// The cumulative index of the first `char` of each range in `self.set` (assuming that the
    /// chars are listed from left-to-right).
    cumulative_start_indices: Vec<usize>,
}

impl Sampler<'_> {
    pub fn new(set: &CharSet) -> Sampler {
        let ascii_values = ascii_values_from_bitmask(set.ascii_bitmask);
        let cumulative_start_indices = {
            let mut start_idx = ascii_values.len(); // The first range starts after all the ASCII chars
            set.ranges
                .iter()
                .map(|r| {
                    let range_len = *r.end() as usize + 1 - *r.start() as usize;
                    let idx = start_idx;
                    start_idx += range_len;
                    idx
                })
                .collect()
        };
        Sampler {
            set,
            ascii_values,
            cumulative_start_indices,
        }
    }

    /// Pick a new [`char`] uniformly from the [`CharSet`], or [`None`] if the [`CharSet`] is
    /// empty.
    pub fn sample(&self, rng: &mut impl Rng) -> Option<char> {
        if self.set.len == 0 {
            return None;
        }
        let char_idx = rng.gen_range(0..self.set.len);
        Some(
            match self.cumulative_start_indices.binary_search(&char_idx) {
                // Err(0) means that `char_idx` is lower than the first cumulative_start_index,
                // meaning it must be within `ascii_values`
                Err(0) => self.ascii_values[char_idx],
                // Ok(idx) means that `char_idx` is precisely at the start of
                // `self.set.ranges[idx]`
                Ok(idx) => *self.set.ranges[idx].start(),
                // Err(idx != 0) means that
                // `cum_start_idxs[idx - 1] < char_idx < cum_start_idxs[idx]`, so `char_idx` is
                // contained within the range at `idx - 1`
                Err(idx) => {
                    let idx = idx - 1; // Can't underflow, because `Err(0)` is a separate match arm
                    let idx_within_range = char_idx - self.cumulative_start_indices[idx];
                    saturating_add_char(*self.set.ranges[idx].start(), idx_within_range as u32)
                }
            },
        )
    }
}

//////////////////////
// HELPER FUNCTIONS //
//////////////////////

/// Returns the `(u64_idx, bit_idx)` of a [`char`], given that it is within
/// `'\0'..FIRST_NON_ASCII_CHAR`.
fn ascii_bit_idx(ch: char) -> Option<u32> {
    let codepoint = ch as u32;
    (codepoint < 128).then(|| codepoint)
}

/// Add some offset to the unicode code-point of a [`char`], saturating at [`char::MAX`]
fn saturating_add_char(ch: char, offset: u32) -> char {
    u32::checked_add(ch as u32, offset)
        .and_then(char::from_u32)
        .unwrap_or(ch)
}

fn ascii_values_from_bitmask(mut mask: u128) -> Vec<char> {
    let mut chars = Vec::new();
    // Repeatedly remove `char`s from `mask` until no chars are left
    while mask != 0 {
        let next_codepoint = mask.trailing_zeros();
        chars.push(char::from_u32(next_codepoint).unwrap());
        mask &= !(1 << next_codepoint);
    }
    chars
}

/// Splits a [`RangeInclusive`] into two [`RangeInclusive`]s, where `boundary` is contained in the
/// right-hand [`RangeInclusive`].  At least one of the [`Option`]s returned is [`Some`].
fn split_range(
    range: &RangeInclusive<char>,
    boundary: char,
) -> (Option<RangeInclusive<char>>, Option<RangeInclusive<char>>) {
    let char_before_boundary = match (boundary as u32).checked_sub(1) {
        Some(v) => char::from_u32(v).unwrap(),
        // If `boundary` is '\0', then the left range must be empty
        None => return (None, Some(range.clone())),
    };

    let left = (*range.start() < boundary).then(|| {
        let end = char::min(char_before_boundary, *range.end());
        *range.start()..=end
    });
    let right = (*range.end() >= boundary).then(|| {
        let start = char::max(boundary, *range.start());
        start..=*range.end()
    });
    (left, right)
}

#[cfg(test)]
mod tests {
    use std::ops::RangeInclusive;

    #[test]
    fn split_range() {
        fn convert_range(range: &RangeInclusive<u32>) -> RangeInclusive<char> {
            let start = char::from_u32(*range.start()).unwrap();
            let end = char::from_u32(*range.end()).unwrap();
            start..=end
        }

        fn check(
            range: RangeInclusive<u32>,
            boundary: u32,
            left: Option<RangeInclusive<u32>>,
            right: Option<RangeInclusive<u32>>,
        ) {
            let range = convert_range(&range);
            let boundary = char::from_u32(boundary).unwrap();
            let left = left.as_ref().map(convert_range);
            let right = right.as_ref().map(convert_range);

            assert_eq!(super::split_range(&range, boundary), (left, right));
        }

        check(0..=10, 5, Some(0..=4), Some(5..=10));
        check(0..=10, 10, Some(0..=9), Some(10..=10));
        check(0..=10, 0, None, Some(0..=10));
        check(5..=10, 0, None, Some(5..=10));
        check(5..=10, 6, Some(5..=5), Some(6..=10));
    }
}
