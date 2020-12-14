pub mod arena;
pub mod ast;
pub mod editable_tree;
pub mod editor;

use crate::arena::Arena;
use crate::ast::json::JSONFormat;
use crate::ast::test_json::TestJSON;
use crate::editable_tree::{cursor_path::CursorPath, DAG};
use crate::editor::Editor;

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

    let mut tree = DAG::new(&arena, root, CursorPath::root());
    let editor = Editor::new(
        &mut tree,
        JSONFormat::Pretty,
        editor::normal_mode::default_keymap(),
    );
    editor.run();
}
