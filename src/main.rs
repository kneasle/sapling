use sapling::ast::json::{JSONFormat, JSON};
use sapling::editor::Editor;

fn main() {
    let editor = Editor::new(JSON::True, JSONFormat::Pretty);
    editor.mainloop();
}
