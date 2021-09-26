use itertools::Itertools;
use number_prefix::NumberPrefix;
use rand::{prelude::SliceRandom, Rng};
use rand_distr::Distribution;

pub fn gen_ws_samples(
    n: usize,
    ws_chars: &[char],
    rng: &mut impl Rng,
    length_distr: impl Distribution<u64> + Copy,
) -> Vec<String> {
    (0..n)
        .map(|_| {
            let len = rng.sample(length_distr) as usize;
            std::iter::repeat_with(|| *ws_chars.choose(rng).unwrap())
                .take(len)
                .collect()
        })
        .collect_vec()
}

pub fn sample_ws<'s>(samples: &'s [String], rng: &mut impl Rng) -> &'s str {
    samples.choose(rng).unwrap()
}

pub fn format_big_bytes(num: f32) -> String {
    match NumberPrefix::decimal(num) {
        NumberPrefix::Standalone(n) => format!("{} bytes", n),
        NumberPrefix::Prefixed(prefix, n) => format!("{:.1} {}B", n, prefix),
    }
}
