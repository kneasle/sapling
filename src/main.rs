//! # Sapling
//! A highly experimental editor where you edit code, not text.

#![deny(missing_docs)]
#![deny(broken_intra_doc_links)]
#![allow(private_intra_doc_links)]

pub mod arena;
pub mod ast;
pub mod config;
pub mod core;
pub mod editor;

use crate::arena::Arena;
use crate::ast::json::JSONFormat;
use crate::ast::test_json::TestJSON;
use crate::config::Config;
use crate::core::Path;
use crate::editor::{dag::DAG, Editor};

/// The entry point of Sapling.
///
/// The main function is tasked with initialising everything, then passing control to
/// [`Editor::run`].
fn main() {
    // Initialise the logging and startup
    pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Debug)
        .init();
    log::info!("Starting up...");

    // Create an empty arena for Sapling to use
    log::trace!("Creating arena");
    let arena = Arena::new();
    // For the time being, start the editor with some pre-made JSON
    let root = TestJSON::Array(vec![
        TestJSON::True,
        TestJSON::False,
        TestJSON::Object(vec![("value".to_string(), TestJSON::True)]),
    ])
    .add_to_arena(&arena);

    let mut tree = DAG::new(&arena, root, Path::root());
    let editor = Editor::new(&mut tree, JSONFormat::Pretty, Config::default());
    editor.run();
}
