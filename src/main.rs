use sapling::ast_spec::json::{JSONFormat, JSON};
use sapling::editable_tree::persistent::{Persistent, Ref};
use sapling::editable_tree::EditableTree;
use sapling::editor::Editor;

fn main() {
    let tree: Persistent<JSON<Ref>> = Persistent::new();
    let editor = Editor::new(tree, JSONFormat::Pretty);
    editor.mainloop();
}
