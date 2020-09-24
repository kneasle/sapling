use super::{cursor_path, EditableTree};
use crate::ast_spec::ASTSpec;
use crate::node_map::vec::{Index, VecNodeMap};
use crate::node_map::{NodeMap, NodeMapMut};

/// A snapshot of the undo history of a specification [`EditableTree`].  This is cloned every time
/// a changes is made to the [`Spec`] struct.
#[derive(Debug, Clone)]
struct Snapshot<Node: ASTSpec<Index>> {
    /// The [`NodeMap`] containing the tree at this point in the undo history
    pub node_map: VecNodeMap<Node>,
    pub cursor_path: Vec<cursor_path::Segment<Index>>,
}

impl<Node: ASTSpec<Index>> Snapshot<Node> {
    /// Makes a `Snapshot` from a given [`VecNodeMap`] with the cursor selecting the root of that
    /// tree
    fn from_node_map(node_map: VecNodeMap<Node>) -> Self {
        let cursor = node_map.root();
        Snapshot {
            node_map,
            cursor_path: vec![cursor_path::Segment::new(cursor, 0)],
        }
    }

    /// Gets the [`Index`] of the current path
    fn cursor(&self) -> Index {
        self.cursor_path.last().unwrap().node
    }
}

/// An [`EditableTree`] that is used as a specification to test other [`EditableTree`]
/// implementations against.  No effort is made to make `Spec` performant in any way - the
/// important thing is that it should be difficult to introduce unintended behaviour.
///
/// This works by storing the history as a [`Snapshot`] of [`VecNodeMap`]s for the trees along with
/// any other metadata about that save state.  Every edit of the tree causes the last [`Snapshot`]
/// to be cloned and the edits made on the new [`Snapshot`].  Therefore, undoing is as simple as
/// just subtracting `1` from `history_index` to make it point to a previous [`Snapshot`].
#[derive(Debug, Clone)]
pub struct Spec<Node: ASTSpec<Index>> {
    /// The sequence of [`Snapshot`]s that represents the undo history.  We require that this
    /// history always contains at least one item.
    history: Vec<Snapshot<Node>>,
    /// The index of the current [`Snapshot`].  We require that this always points to a valid index
    /// in `history`
    current_snapshot_index: usize,
}

impl<Node: ASTSpec<Index>> Spec<Node> {
    /// Makes a `DAG` that contains the tree stored inside `node_map`
    pub fn from_tree(node_map: VecNodeMap<Node>) -> Self {
        Spec {
            history: vec![Snapshot::from_node_map(node_map)],
            current_snapshot_index: 0,
        }
    }

    /// Returns the currently viewed [`Snapshot`]
    fn snapshot(&self) -> &Snapshot<Node> {
        // We don't have to worry about bounds checks because we require that
        // `self.current_snapshot_index` is a valid index in `self.history`
        &self.history[self.current_snapshot_index]
    }

    /// Adds a new snapshot to the tree history (deleting the current redo history if needed).
    fn make_change(&mut self, snapshot: Snapshot<Node>) {
        // Delete the history that happened in front of the current snapshot
        while self.history.len() > self.current_snapshot_index + 1 {
            self.history.pop();
        }
        debug_assert_eq!(self.history.len(), self.current_snapshot_index + 1);
        // Add the new snapshot
        self.history.push(snapshot);
        self.current_snapshot_index += 1;
    }
}

impl<Node: ASTSpec<Index>> NodeMap<Index, Node> for Spec<Node> {
    fn get_node(&self, id: Index) -> Option<&Node> {
        self.snapshot().node_map.get_node(id)
    }

    fn root(&self) -> Index {
        // We require that current_path.len() >= 1, so we don't have to worry about panics
        self.snapshot().node_map.root()
    }
}

impl<Node: ASTSpec<Index>> EditableTree<Index, Node> for Spec<Node> {
    fn new() -> Self {
        Self::from_tree(VecNodeMap::with_default_root())
    }

    fn cursor(&self) -> Index {
        self.snapshot().cursor()
    }

    fn replace_cursor(&mut self, new_node: Node) {
        let mut new_snapshot = self.snapshot().clone();
        // Overwrite the node under the cursor
        new_snapshot
            .node_map
            .overwrite_node(new_snapshot.cursor(), new_node);
        self.make_change(new_snapshot);
    }

    fn insert_child(&mut self, _new_node: Node) {
        unimplemented!();
    }

    fn write_text(&self, string: &mut String, format: &Node::FormatStyle) {
        self.snapshot().node_map.write_text(string, format);
    }
}
