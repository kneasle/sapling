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
    tui_logger::init_logger(log::LevelFilter::Info).unwrap();
    log::info!("Starting up...");

    // Read a file name as the CLI argument
    let (file_path, initial_json) = if let Some(first_arg) = std::env::args().skip(1).next() {
        let path = PathBuf::from(first_arg);
        let file = match std::fs::File::open(&path) {
            Ok(x) => x,
            Err(e) => {
                eprintln!("Error opening {:?}: {}", path, e);
                return;
            }
        };
        (
            Some(path),
            serde_json::from_reader::<_, Value>(file).unwrap(),
        )
    } else {
        log::warn!("Expected a file-name as an argument.  Using default JSON instead.");
        (None, serde_json::json!([true, false, { "value": false }]))
    };

    // Create an empty arena for Sapling to use
    log::trace!("Creating arena");
    let arena = Arena::new();
    // For the time being, start the editor with some pre-made Json
    let root = add_value_to_arena(initial_json, &arena);

    let mut tree = Dag::new(&arena, root, Path::root());
    let editor = Editor::new(&mut tree, JsonFormat::Pretty, Config::default(), file_path);
    editor.run();
}
