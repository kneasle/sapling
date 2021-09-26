//! Code to fuzz Sapling's tokenizer

use itertools::Itertools;
use rand::Rng;
use rand_distr::Geometric;
use sapling::Lang;
use sapling_grammar::{tokenizer, TokenId, TypeId};

use crate::{
    fuzzer::{self, Arbitrary},
    utils,
};

/// A string of tokens, interspersed with whitespace
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TokenString {
    leading_ws: String,
    tokens: Vec<(ParsedToken, String)>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum ParsedToken {
    Static(TokenId),
    Stringy {
        ty_id: TypeId,
        contents: String,
        display_str: String,
    },
}

impl From<tokenizer::ParsedToken<'_>> for ParsedToken {
    fn from(tok: tokenizer::ParsedToken) -> Self {
        match tok {
            tokenizer::ParsedToken::Static(id) => ParsedToken::Static(id),
            tokenizer::ParsedToken::Stringy(ty_id, contents, display_str) => ParsedToken::Stringy {
                ty_id,
                contents,
                display_str: display_str.to_owned(),
            },
        }
    }
}

impl<'lang> Arbitrary<'lang> for TokenString {
    type Config = Config;
    type StaticData = StaticData<'lang>;
    type SampleTable = SampleTable;

    fn gen_static_data(lang: &'lang Lang, config: &Self::Config) -> Self::StaticData {
        StaticData {
            ws_len_distr: Geometric::new(1.0 / config.average_ws_length).unwrap(),
            stream_len_distr: Geometric::new(1.0 / config.average_length_tokens).unwrap(),
            num_static_token_types: lang.grammar().num_tokens(),
            lang,
        }
    }

    fn gen_table(
        data: &Self::StaticData,
        rng: &mut impl Rng,
        _config: &Self::Config,
    ) -> Self::SampleTable {
        SampleTable {
            ws_samples: utils::gen_ws_samples(
                3000,
                &data.lang.grammar().whitespace().all_chars(),
                rng,
                data.ws_len_distr,
            ),
        }
    }

    fn gen(
        data: &Self::StaticData,
        table: &Self::SampleTable,
        rng: &mut impl Rng,
        _config: &Self::Config,
    ) -> Self {
        let leading_ws = utils::sample_ws(&table.ws_samples, rng).to_owned();
        let stream_length = rng.sample(data.stream_len_distr);
        let tokens = (0..stream_length)
            .map(|_| {
                // For now, only generate static tokens
                let tok_id = TokenId::new(rng.gen_range(0..data.num_static_token_types));
                let ws = utils::sample_ws(&table.ws_samples, rng).to_owned();
                (ParsedToken::Static(tok_id), ws)
            })
            .collect_vec();
        Self { leading_ws, tokens }
    }

    fn unparse(&self, data: &Self::StaticData, s: &mut String) {
        s.clear();
        s.push_str(&self.leading_ws);
        for (token, ws) in &self.tokens {
            match token {
                ParsedToken::Static(tok_id) => s.push_str(data.lang.grammar().token_text(*tok_id)),
                ParsedToken::Stringy { display_str, .. } => s.push_str(&display_str),
            }
            s.push_str(ws);
        }
    }

    fn parse(data: &Self::StaticData, s: &str) -> Option<Self> {
        let (leading_ws, token_iter) = data.lang.tokenize(s);
        let mut tokens = Vec::<(ParsedToken, String)>::new();
        for token_result in token_iter {
            let (token, ws) = token_result.ok()?;
            tokens.push((token.into(), ws.to_owned()));
        }
        Some(Self {
            leading_ws: leading_ws.to_owned(),
            tokens,
        })
    }
}

/// Configuration parameters for generating token strings
#[derive(Debug, Clone)]
pub struct Config {
    /// The average number of tokens in each generated string
    average_length_tokens: f64,
    /// The average length of the whitespace string
    average_ws_length: f64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            average_length_tokens: 10_000.0,
            average_ws_length: 5.0,
        }
    }
}

/// Static data for generating token strings of a given language
#[derive(Debug, Clone)]
pub struct StaticData<'lang> {
    ws_len_distr: Geometric,
    stream_len_distr: Geometric,
    lang: &'lang Lang,
    /// How many different types of static tokens the language has
    num_static_token_types: usize,
}

/// Table in which random samples can be cached to speed up the parsing table
#[derive(Debug, Clone)]
pub struct SampleTable {
    ws_samples: Vec<String>,
}

pub fn fuzz_tokenizer(lang: &Lang, iteration_limit: Option<usize>, average_length_tokens: f64) {
    let config = Config {
        average_length_tokens,
        ..Config::default()
    };
    fuzzer::fuzz::<TokenString>(lang, iteration_limit, &config);
}
