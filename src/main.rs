use sapling::arena::Arena;
use sapling::ast::json::JSONFormat;
use sapling::ast::test_json::TestJSON;
use sapling::editable_tree::{dag::DAG, EditableTree};
use sapling::editor::Editor;

fn main() {
    // Create an empty arena for Sapling to use
    let arena = Arena::new();
    // For the time being, start the editor with some pre-made JSON
    let root = TestJSON::Array(vec![
        TestJSON::True,
        TestJSON::False,
        TestJSON::Object(vec![("value".to_string(), TestJSON::True)]),
    ])
    .add_to_arena(&arena);

    let mut tree = DAG::new(&arena, root);
    let editor = Editor::new(&mut tree, JSONFormat::Pretty);
    editor.run();
}
