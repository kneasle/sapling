use sapling::ast_spec::json::{JSONFormat, JSON};
use sapling::editable_tree::dag::DAG;
use sapling::editable_tree::reference::Ref;
use sapling::editable_tree::EditableTree;
use sapling::editor::Editor;

fn main() {
    let tree: DAG<JSON<Ref>> = DAG::new();
    let editor = Editor::new(tree, JSONFormat::Pretty);
    editor.mainloop();
}
