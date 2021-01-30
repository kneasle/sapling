//! # Sapling
//!
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
use crate::ast::json::{add_value_to_arena, JsonFormat};
use crate::config::Config;
use crate::core::Path;
use crate::editor::{dag::Dag, Editor};

use std::path::PathBuf;

use serde_json::Value;

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

    let file_path = PathBuf::from("thing.json");
    let initial_json =
        serde_json::from_reader::<_, Value>(std::fs::File::open(&file_path).unwrap()).unwrap();

    // Create an empty arena for Sapling to use
    log::trace!("Creating arena");
    let arena = Arena::new();
    // For the time being, start the editor with some pre-made Json
    let root = add_value_to_arena(initial_json, &arena);

    let mut tree = Dag::new(&arena, root, Path::root());
    let editor = Editor::new(&mut tree, JsonFormat::Pretty, Config::default(), file_path);
    editor.run();
}
