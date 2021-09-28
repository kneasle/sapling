//! Fuzzer for Sapling's full parsing pipeline (i.e. converting strings into syntax trees)

use std::{collections::HashMap, rc::Rc};

use rand::{prelude::SliceRandom, Rng};
use rand_distr::Geometric;
use sapling::{ast, Lang};
use sapling_grammar::{char_set, Grammar, PatternElement, Stringy, TypeId, TypeInner};

use crate::utils;

pub fn fuzz(lang: &Lang, iteration_limit: Option<usize>) {
    crate::fuzz::<ast::Tree>(lang, iteration_limit, &Config::default());
}

impl<'lang> crate::Arbitrary<'lang> for ast::Tree {
    type Config = Config;
    type StaticData = StaticData<'lang>;
    type SampleTable = SampleTable;

    fn gen_static_data(lang: &'lang Lang, config: &Config) -> Self::StaticData {
        let stringy_regex_generators = lang
            .grammar()
            .types()
            .iter_enumerated()
            .filter_map(|(id, ty)| {
                let regex_str = match ty.inner() {
                    TypeInner::Stringy(s) => match s.unanchored_content_regex() {
                        Some(regex) => regex.as_str(),
                        None => ".*",
                    },
                    _ => return None,
                };
                let regex_gen =
                    rand_regex::Regex::compile(regex_str, config.max_stringy_regex_repeats)
                        .unwrap();
                Some((id, regex_gen))
            })
            .collect();

        StaticData {
            lang,

            ws_len_distr: Geometric::new(1.0 / config.average_ws_length).unwrap(),
            one_minus_new_segment_prob: 1.0 / config.average_tree_size,
            tree_depth_limit: config.tree_depth_limit,
            tree_node_limit: config.tree_node_limit,

            ws_sampler: lang.grammar().whitespace().sampler(),
            stringy_regex_generators,
        }
    }

    fn gen_table(
        data: &Self::StaticData,
        rng: &mut impl Rng,
        _config: &Self::Config,
    ) -> Self::SampleTable {
        SampleTable {
            ws_samples: utils::gen_ws_samples(3000, &data.ws_sampler, rng, data.ws_len_distr),
        }
    }

    /// Create a random syntax tree
    fn gen(
        data: &Self::StaticData,
        table: &Self::SampleTable,
        _config: &Self::Config,
        rng: &mut impl Rng,
    ) -> Self {
        let leading_ws = table.gen_ws(rng);
        let mut state = TreeGenState {
            data,
            table,
            rng,
            nodes_generated: 0,
        };
        let root = gen_node(data.grmr().root_type(), &mut state, 0);
        Self::new(leading_ws, root)
    }

    fn unparse(&self, data: &Self::StaticData, s: &mut String) {
        self.write_text(data.grmr(), s).unwrap();
    }

    fn parse(data: &Self::StaticData, s: &str) -> Option<Self> {
        data.lang.parse_root(s).ok()
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    average_ws_length: f64,
    average_tree_size: f64,
    /// The upper bound placed on open-bounded regex patterns in stringy tokens
    max_stringy_regex_repeats: u32,
    tree_depth_limit: usize,
    tree_node_limit: usize,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            average_ws_length: 5.0,
            average_tree_size: 2.0,
            max_stringy_regex_repeats: 15,
            tree_depth_limit: 15,
            tree_node_limit: 1_000,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StaticData<'lang> {
    lang: &'lang Lang,

    ws_len_distr: Geometric,
    /// The probability with which each new sequence segment is added.  This is the reciprocal of
    /// the expected number of seq segments in the tree.
    one_minus_new_segment_prob: f64,
    tree_depth_limit: usize,
    tree_node_limit: usize,

    ws_sampler: char_set::Sampler<'lang>,
    stringy_regex_generators: HashMap<TypeId, rand_regex::Regex>,
}

impl<'lang> StaticData<'lang> {
    fn grmr(&self) -> &Grammar {
        self.lang.grammar()
    }
}

/// Table in which random samples can be cached to speed up generation of trees
#[derive(Debug, Clone)]
pub struct SampleTable {
    ws_samples: Vec<String>,
}

impl SampleTable {
    fn gen_ws(&self, rng: &mut impl Rng) -> String {
        utils::sample_ws(&self.ws_samples, rng).to_owned()
    }
}

/////////////////////
// TREE GENERATION //
/////////////////////

/// Static state passed to all the `gen_*` functions
#[derive(Debug)]
struct TreeGenState<'a, 'lang, R: Rng> {
    data: &'a StaticData<'lang>,
    table: &'a SampleTable,
    rng: &'a mut R,
    nodes_generated: usize,
}

fn gen_node(type_id: TypeId, state: &mut TreeGenState<impl Rng>, depth: usize) -> Rc<ast::Node> {
    let data = state.data;
    state.nodes_generated += 1;

    let ty = data.grmr().get_type(type_id);
    // Pick a random descendant type to generate
    let concrete_id = *ty
        .parseable_descendants()
        .choose(state.rng)
        .expect("Can't generate a type with no parse-able descendants");

    let concrete_ty = data.grmr().get_type(concrete_id);
    let node = match concrete_ty.inner() {
        TypeInner::Pattern(src_pattern) => {
            let mut pattern = ast::Pattern::new();
            gen_pattern(&src_pattern, &mut pattern, state, depth);
            ast::Node::Tree(ast::TreeNode::new(concrete_id, pattern))
        }
        TypeInner::Stringy(stringy) => ast::Node::Stringy {
            inner: gen_stringy(concrete_id, stringy, data, state.rng),
            ws: state.table.gen_ws(state.rng),
        },
        TypeInner::Container => {
            unreachable!("`ty.parseable_descendants()` shouldn't return a container")
        }
    };
    Rc::new(node)
}

fn gen_pattern(
    pattern: &[PatternElement],
    out: &mut ast::Pattern,
    state: &mut TreeGenState<impl Rng>,
    depth: usize,
) {
    for elem in pattern {
        gen_elem(elem, out, state, depth);
    }
}

fn gen_elem(
    elem: &PatternElement,
    out: &mut ast::Pattern,
    state: &mut TreeGenState<impl Rng>,
    depth: usize,
) {
    let table = state.table;
    let data = state.data;

    match elem {
        &PatternElement::Token(token_id) => {
            let new_elem = ast::Elem::Token {
                token_id,
                ws: table.gen_ws(state.rng),
            };
            out.push(new_elem);
        }
        &PatternElement::Type(type_bound) => {
            let new_elem = ast::Elem::Node {
                type_bound,
                node: gen_node(type_bound, state, depth + 1),
            };
            out.push(new_elem);
        }
        PatternElement::Seq { pattern, delimiter } => {
            out.push(ast::Elem::SeqStart);
            let mut is_first_node = true;
            while state.rng.gen_range(0.0..1.0) > data.one_minus_new_segment_prob {
                // TODO: Handle depth limiting better than this
                if depth > data.tree_depth_limit || state.nodes_generated > data.tree_node_limit {
                    break;
                }

                // Add delimiter between nodes
                if !is_first_node {
                    out.push(ast::Elem::SeqDelim(*delimiter, table.gen_ws(state.rng)));
                }
                // Add segment
                gen_pattern(pattern, out, state, depth);
                is_first_node = false;
            }
            out.push(ast::Elem::SeqEnd);
        }
    }
}

fn gen_stringy(
    type_id: TypeId,
    stringy: &Stringy,
    data: &StaticData,
    rng: &mut impl Rng,
) -> ast::StringyNode {
    let regex_generator = data.stringy_regex_generators.get(&type_id).unwrap();
    let (contents, display_str) = utils::gen_stringy(stringy, regex_generator, rng);
    ast::StringyNode::new(type_id, contents, display_str)
}
