use sapling::ast_spec::json::{JSONFormat, JSON};
use sapling::editable_tree::dag::DAG;
use sapling::editable_tree::EditableTree;
use sapling::editor::Editor;
use sapling::vec_node_map::Index;

fn main() {
    let tree: DAG<JSON<Index>> = DAG::new();
    let editor = Editor::new(tree, JSONFormat::Pretty);
    editor.mainloop();
}
