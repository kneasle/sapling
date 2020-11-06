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
    root_history: Vec<&'arena Node>,
    /// An index into [`root_history`](DAG::root_history) of the current edit.  This is required to
    /// be in `0..root_history.len()`.
    history_index: usize,
}

impl<'arena, Node: Ast<'arena>> EditableTree<'arena, Node> for DAG<'arena, Node> {
    fn new(arena: &'arena Arena<Node>, root: &'arena Node) -> Self {
        DAG {
            arena,
            root_history: vec![root],
            history_index: 0,
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
        self.root_history[self.history_index]
    }

    fn cursor(&self) -> &'arena Node {
        self.root()
    }

    fn move_cursor(&mut self, _direction: Direction) -> Option<String> {
        unimplemented!();
    }

    fn replace_cursor(&mut self, new_node: Node) {
        // Removing future tree from the history vector until we're at the latest change.
        while self.history_index < self.root_history.len() - 1 {
            // TODO: Deallocate the tree so that we don't get a memory leak
            self.root_history.pop();
        }
        // TODO: Once cursor movent is fixed, this needs to not replace the root.
        let new_root = self.arena.alloc(new_node);
        self.root_history.push(new_root);
        self.history_index = self.root_history.len() - 1;
    }

    fn insert_child(&mut self, _new_node: Node) {
        unimplemented!();
    }

    fn write_text(&self, string: &mut String, format: &Node::FormatStyle) {
        self.root().write_text(string, format);
    }
}
