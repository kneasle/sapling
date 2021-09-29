use std::collections::VecDeque;

use crate::ast::Tree;

/// A [`History`] of syntax [`Tree`]
#[derive(Debug, Clone)]
pub struct History {
    /// The sequence of [`Tree`]s, each being a snapshot of a buffer after each edit.  These are
    /// stored in the order that they were created (i.e. oldest at the 'front', newest at the
    /// 'back').
    ///
    /// **Invariant**: This must always be non-empty.
    trees: VecDeque<Tree>,
    /// The index (within `self.trees`) of the currently visible snapshot.  Undo decrements this
    /// counter, whilst redo increments it.
    ///
    /// **Invariant**: `index < trees.len()`
    index: usize,
    /// The maximum number of undo steps which can be recorded in this history.  Once this limit is
    /// exceeded, the oldest snapshots will be dropped.
    max_undo_depth: usize,
}

impl History {
    /// Creates a new `History` containing only one snapshot
    pub fn new(tree: Tree, max_undo_depth: usize) -> Self {
        let mut trees = VecDeque::new();
        trees.push_back(tree);
        Self {
            trees,
            index: 0,
            max_undo_depth,
        }
    }
}
