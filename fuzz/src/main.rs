//! Automated random testing and benchmarking for various components of Sapling (tokenizers,
//! parsers, etc.)

use std::time::{Duration, Instant};

use itertools::Itertools;
use number_prefix::NumberPrefix;
use rand::{prelude::SliceRandom, Rng};
use rand_distr::{Distribution, Geometric};
use sapling::Lang;
use sapling_grammar::TokenId;

fn main() {
    let lang = Lang::load_toml_file("json.toml").unwrap();
    // Average length should be around 10k tokens, no iteration limit
    fuzz_tokenizer(lang, 10_000, Some(10_000));
}

pub fn fuzz_tokenizer(lang: Lang, average_length_tokens: usize, iteration_limit: Option<usize>) {
    // TODO: Before fuzzing, check for token sequences which generate a new token (for example `&`
    // and `&` would be re-tokenized to simply `&&`).  This doesn't happen for JSON so I'm gonna
    // ignore it for now.

    let mut rng = rand::thread_rng();
    let ws_len_distr = Geometric::new(0.2).unwrap();
    let stream_len_distr = Geometric::new(1.0 / average_length_tokens as f64).unwrap();
    let num_unique_tokens = lang.grammar().num_tokens();

    // Stat printing state
    let fuzz_start_time = Instant::now();
    let mut elapsed_secs_for_last_print = 0;

    // Fuzzing loop
    let mut unparsed_string = String::new();
    let mut iterations = 0usize;
    let mut total_bytes_tokenized = 0;
    let mut total_time_tokenizing = Duration::ZERO;
    loop {
        // Generate a load of whitespace samples every couple of thousand loop iterations, to save
        // compute time during the fuzzing loop
        let ws_samples = gen_ws_samples(3000, &lang, &mut rng, ws_len_distr);

        for _ in 0..1_000 {
            // Generate test case (i.e. a parsed token stream)
            let leading_ws = sample_ws(&ws_samples, &mut rng);
            let stream_length = rng.sample(stream_len_distr);
            let expected_tokens = (0..stream_length)
                .map(|_| {
                    let token = TokenId::new(rng.gen_range(0..num_unique_tokens));
                    let ws = sample_ws(&ws_samples, &mut rng);
                    (token, ws)
                })
                .collect_vec();

            // 'Unparse' the token stream into a string
            unparsed_string.clear();
            unparsed_string.push_str(leading_ws);
            for (token_id, ws) in &expected_tokens {
                unparsed_string.push_str(lang.grammar().token_text(*token_id));
                unparsed_string.push_str(ws);
            }

            // Tokenise the string.  Also time the speed of the tokenizer
            let start = Instant::now();
            let (tokenized_leading_ws, token_iter) = lang.tokenize(&unparsed_string);
            let tokenize_result = token_iter.collect::<Result<Vec<_>, _>>();
            let is_passed = tokenize_result.map_or(false, |tokens| {
                leading_ws == tokenized_leading_ws && tokens == expected_tokens
            });
            total_bytes_tokenized += unparsed_string.len();
            total_time_tokenizing += start.elapsed();

            // Check that the test passed.  TODO: Shrink the input in this case
            assert!(is_passed);

            iterations += 1;
            let reached_iteration_limit =
                iteration_limit.map_or(false, |limit| iterations >= limit);

            // Print stats roughly every second, or when the test ends.  We check the times every
            // few loop iterations to speed up the fuzzing loop.
            let elapsed_secs = fuzz_start_time.elapsed().as_secs();
            if elapsed_secs > elapsed_secs_for_last_print || reached_iteration_limit {
                elapsed_secs_for_last_print = elapsed_secs;
                println!(
                    "{} iters.  {} in {:?} = {}/s",
                    iterations,
                    format_big_bytes(total_bytes_tokenized as f32),
                    total_time_tokenizing,
                    format_big_bytes(
                        total_bytes_tokenized as f32 / total_time_tokenizing.as_secs_f32()
                    )
                );
            }
            // Exit loop if iteration limit is reached
            if reached_iteration_limit {
                return;
            }
        }
    }
}

fn gen_ws_samples(
    n: usize,
    lang: &Lang,
    rng: &mut impl Rng,
    length_distr: impl Distribution<u64> + Copy,
) -> Vec<String> {
    let whitespace_chars: Vec<char> = lang.grammar().whitespace().all_chars();

    (0..n)
        .map(|_| {
            let len = rng.sample(length_distr) as usize;
            std::iter::repeat_with(|| *whitespace_chars.choose(rng).unwrap())
                .take(len)
                .collect()
        })
        .collect_vec()
}

fn sample_ws<'s>(samples: &'s [String], rng: &mut impl Rng) -> &'s str {
    samples.choose(rng).unwrap()
}

fn format_big_bytes(num: f32) -> String {
    match NumberPrefix::decimal(num) {
        NumberPrefix::Standalone(n) => format!("{} bytes", n),
        NumberPrefix::Prefixed(prefix, n) => format!("{:.1} {}B", n, prefix),
    }
}
