use super::cursor_path::CursorPath;
use super::{Direction, EditableTree};
use crate::arena::Arena;
use crate::ast::Ast;

/// An [`EditableTree`] that stores the history as a DAG (Directed Acyclic Graph) of **immutable**
/// nodes.
///
/// This means that every node that has ever been created exists somewhere in the DAG, and when
/// changes are made, every ancestor of that node is cloned until the root is reached and that
/// root becomes the new 'current' root.  This is very similar to the way Git stores the commits,
/// and every edit is analogous to a Git rebase.
///
/// Therefore, moving back through the history is as simple as reading a different root node from
/// the `roots` vector, and following its descendants through the DAG of nodes.
pub struct DAG<'arena, Node: Ast<'arena>> {
    /// The arena in which all the [`Node`]s will be stored
    arena: &'arena Arena<Node>,
    /// A [`Vec`] containing a reference to the root node at every edit in the undo history.  This
    /// is required to always have length at least one.
    root_history: Vec<(&'arena Node, CursorPath)>,
    /// An index into [`root_history`](DAG::root_history) of the current edit.  This is required to
    /// be in `0..root_history.len()`.
    history_index: usize,
    current_cursor_path: CursorPath,
}

impl<'arena, Node: Ast<'arena>> DAG<'arena, Node> {
    /// Returns the cursor node and its direct parent (if such a parent exists)
    fn cursor_and_parent(&self) -> (&'arena Node, Option<&'arena Node>) {
        self.current_cursor_path.cursor_and_parent(self.root())
    }
}

impl<'arena, Node: Ast<'arena>> EditableTree<'arena, Node> for DAG<'arena, Node> {
    fn new(arena: &'arena Arena<Node>, root: &'arena Node) -> Self {
        DAG {
            arena,
            root_history: vec![(root, CursorPath::root())],
            history_index: 0,
            current_cursor_path: CursorPath::root(),
        }
    }

    /* HISTORY METHODS */

    fn undo(&mut self) -> bool {
        if self.history_index > 0 {
            self.history_index -= 1;
            true
        } else {
            false
        }
    }

    fn redo(&mut self) -> bool {
        if self.history_index < self.root_history.len() - 1 {
            self.history_index += 1;
            true
        } else {
            false
        }
    }

    /* NAVIGATION METHODS */

    fn root(&self) -> &'arena Node {
        // This indexing cannot panic because we require that `self.history_index` is a valid index
        // into `self.root_history`, and `self.root_history` has at least one element
        self.root_history[self.history_index].0
    }

    fn cursor(&self) -> &'arena Node {
        self.current_cursor_path.cursor(self.root())
    }

    fn move_cursor(&mut self, direction: Direction) -> Option<String> {
        let (current_cursor, cursor_parent) = self.cursor_and_parent();
        match direction {
            Direction::Down => {
                if current_cursor.children().is_empty() {
                    Some("Cannot move down the tree if the cursor has no children.".to_string())
                } else {
                    self.current_cursor_path.push(0);
                    None
                }
            }
            Direction::Up => {
                if self.current_cursor_path.is_root() {
                    return Some("Cannot move to the parent of the root.".to_string());
                }
                self.current_cursor_path.pop();
                None
            }
            Direction::Prev => {
                if let Some(index) = self.current_cursor_path.last_mut() {
                    if *index == 0 {
                        Some("Cannot move before the first child of a node.".to_string())
                    } else {
                        *index -= 1;
                        None
                    }
                } else {
                    return Some("Cannot move to a sibling of the root.".to_string());
                }
            }
            Direction::Next => {
                if let Some(last_index) = self.current_cursor_path.last_mut() {
                    // We can unwrap here, because the only way for a node to not have a parent is
                    // if it's the root.  And if the cursor is at the root, then the `if let` would
                    // fail, so this code would not run.
                    if *last_index + 1 < cursor_parent.unwrap().children().len() {
                        *last_index += 1;
                        None
                    } else {
                        Some("Cannot move past the last sibling of a node.".to_string())
                    }
                } else {
                    return Some("Cannot move to a sibling of the root.".to_string());
                }
            }
        }
    }

    fn replace_cursor(&mut self, new_node: Node) {
        // Removing future tree from the history vector until we're at the latest change.
        while self.history_index < self.root_history.len() - 1 {
            // TODO: Deallocate the tree so that we don't get a memory leak
            self.root_history.pop();
        }
        // TODO: Once cursor movent is fixed, this needs to not replace the root.
        let new_root = self.arena.alloc(new_node);
        // TODO: Once cursor movent is fixed, we need to somehow preserve the cursor location
        self.root_history.push((new_root, vec![]));
        self.history_index = self.root_history.len() - 1;
    }

    fn insert_child(&mut self, _new_node: Node) {
        unimplemented!();
    }

    fn write_text(&self, string: &mut String, format: &Node::FormatStyle) {
        self.root().write_text(string, format);
    }
}
