//! Crate for handling language-independent grammars.
//!
//! This includes:
//! - The central [`Grammar`] data structure (in the [`grammar`] module)
//! - A deserializeable schema for the TOML files (the [`spec`] and [`spec::convert`] modules)
//! - Code for tokenizing & parsing arbitrary strings into ASTs of any language for which a
//!   [`Grammar`] is provided (in the [`tokenizer`] and [`parser`] modules, respectively).
//!
// TODO: Keep the throughput statistic up-to-date with parser changes
//! The current parsing engine is quite bodged - it works correctly for the few languages currently
//! supported by Sapling, but is relatively slow (throughput is roughly 55MB/s for JSON) and has a
//! few weird edge cases, like being unable to handle left-recursive grammars.  However, it has
//! been quite extensively fuzzed (see the `fuzz` crate), so should be fairly stable and accurate.
//!
//! Speed/quality improvements to this crate are very welcome - optimisations are very much
//! encouraged and rewrites of any scale is good, as long as the external behaviour of the parser
//! is still correct.  Well-justified use of `unsafe` is also fine, but generally Sapling cares
//! more about safety and correctness then absolutely top-notch performance.  Also, I suspect that
//! a reasonably well optimised parser is unlikely to be a major bottleneck of Sapling, so it's
//! worth doing some profiling before committing large amounts of time into optimising the parsing
//! engine.

#![allow(rustdoc::private_intra_doc_links)] // This crate is only meant to be used in Sapling

pub mod char_set;
mod grammar;
pub mod parser;
pub mod spec; // AST-like specification of the TOML format consumed by Sapling
pub mod tokenizer;

pub use grammar::*;
pub use spec::SpecGrammar;
