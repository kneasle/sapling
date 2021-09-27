use itertools::Itertools;
use number_prefix::NumberPrefix;
use rand::{prelude::SliceRandom, Rng};
use rand_distr::Distribution;
use sapling_grammar::{char_set, Stringy};

pub fn gen_ws_samples(
    n: usize,
    ws_sampler: &char_set::Sampler,
    rng: &mut impl Rng,
    length_distr: impl Distribution<u64> + Copy,
) -> Vec<String> {
    (0..n)
        .map(|_| {
            let len = rng.sample(length_distr) as usize;
            std::iter::repeat_with(|| ws_sampler.sample(rng).unwrap())
                .take(len)
                .collect()
        })
        .collect_vec()
}

pub fn sample_ws<'s>(samples: &'s [String], rng: &mut impl Rng) -> &'s str {
    samples.choose(rng).unwrap()
}

/// Format a large number of bytes, using prefixes like Giga-, Mega-, kilo-, etc.
pub fn format_big_bytes(num: f32) -> String {
    match NumberPrefix::decimal(num) {
        NumberPrefix::Standalone(n) => format!("{} bytes", n),
        NumberPrefix::Prefixed(prefix, n) => format!("{:.1} {}B", n, prefix),
    }
}

/// Generates a `(contents, display_str)` pair for a random instance of a given [`Stringy`] type
pub fn gen_stringy(
    stringy: &Stringy,
    regex: &rand_regex::Regex,
    rng: &mut impl Rng,
) -> (String, String) {
    let contents: String = regex.sample(rng);
    let display_str = stringy.create_display_string(&contents);
    (contents, display_str)
}
