//! Automated random testing and benchmarking for various components of Sapling (tokenizers,
//! parsers, etc.).
//!
//! TODO: Replace this with `quickcheck` or `proptest`?

mod parser;
mod runner;
mod tokenizer;
mod utils;

use std::{borrow::Cow, ops::Deref};

use rand::Rng;
use sapling::Lang;

fn main() {
    let lang = Lang::load_toml_file("json.toml").unwrap();

    let fuzz_tokenizer = false;

    // Average length should be around 10k tokens, and run 10k iterations
    if fuzz_tokenizer {
        tokenizer::fuzz(&lang, Some(10_000), 10_000.0);
    } else {
        parser::fuzz(&lang, Some(10_000));
    }
}

pub trait Arbitrary<'lang>: Sized + Eq {
    /// Configuration parameters passed into the [`fuzz`] function
    type Config: Default;
    /// Static data generated once before entering the fuzzing loop
    type StaticData;
    /// Sample tables generated every couple of thousand fuzzing iterations.  This allows the
    /// program to cache commonly computed values (e.g. whitespace) to speed up sample generation.
    type SampleTable;
    /// A shrunk instance of `Self`.  Extra state can be added to this to implement more complex
    /// shrinking strategies.
    type Shrink: Shrink + From<Self> + Into<Self> + Deref<Target = Self>;

    /* STATIC TABLE GENERATION */
    fn gen_static_data(lang: &'lang Lang, config: &Self::Config) -> Self::StaticData;
    fn gen_table(
        data: &Self::StaticData,
        rng: &mut impl Rng,
        config: &Self::Config,
    ) -> Self::SampleTable;

    /* TESTING */
    /// Create a new sample to test
    fn gen(
        data: &Self::StaticData,
        table: &Self::SampleTable,
        config: &Self::Config,
        rng: &mut impl Rng,
    ) -> Self;
    /// Write this sample to a string
    fn unparse(&self, data: &Self::StaticData, s: &mut String);
    /// Parse a sample from a given string.  This is expected to be an inverse of `unparse`
    fn parse(data: &Self::StaticData, s: &str) -> Option<Self>;
}

pub trait Shrink: Clone {
    fn smaller_cases<'s>(&'s self) -> Box<dyn Iterator<Item = Cow<'s, Self>> + 's>;
}
