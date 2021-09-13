use std::sync::Arc;

use crate::Grammar;

#[derive(Debug, Clone)]
pub struct Parser {
    grammar: Arc<Grammar>,
}

impl Parser {
    /// Creates a new [`Parser`] which parses the language specified by a given [`Grammar`].
    pub fn new(grammar: Arc<Grammar>) -> Self {
        Self { grammar }
    }
}
