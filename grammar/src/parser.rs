use std::sync::Arc;

use crate::{tokenizer::Tokenizer, Grammar};

/// A persistent `struct` which can parse [`str`]ings into syntax trees, following a [`Grammar`].
#[derive(Debug, Clone)]
pub struct Parser {
    grammar: Arc<Grammar>,
    // TODO: Once treeifier is implemented, make this not pub
    pub tokenizer: Tokenizer,
}

impl Parser {
    /// Creates a new [`Parser`] which parses the language specified by a given [`Grammar`].
    pub fn new(grammar: Arc<Grammar>) -> Self {
        Self {
            tokenizer: Tokenizer::new(grammar.clone()),
            grammar,
        }
    }
}
