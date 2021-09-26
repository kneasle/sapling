//! Automated random testing and benchmarking for various components of Sapling (tokenizers,
//! parsers, etc.)

mod parser;
mod tokenizer;
mod utils;

use sapling::Lang;

fn main() {
    let lang = Lang::load_toml_file("json.toml").unwrap();

    // Average length should be around 10k tokens, and run 10k iterations
    tokenizer::fuzz_tokenizer(&lang, Some(10_000), 10_000.0);
}
