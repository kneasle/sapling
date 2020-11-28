//! Specification of an editable, undoable buffer of trees and some implementations thereof.

pub mod cursor_path;

use crate::arena::Arena;
use crate::ast::Ast;
use cursor_path::CursorPath;

/// The possible ways you can move the cursor
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Direction {
    Up,
    Down,
    Prev,
    Next,
}

/// An enum to represent the two sides of a cursor
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Side {
    Prev,
    Next,
}

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
    /// Builds a new `DAG`, given the tree it should contain
    pub fn new(arena: &'arena Arena<Node>, root: &'arena Node) -> Self {
        DAG {
            arena,
            root_history: vec![(root, CursorPath::root())],
            history_index: 0,
            current_cursor_path: CursorPath::root(),
        }
    }

    /* HISTORY METHODS */

    /// Move one step back in the tree history, returning `false` if there are no more changes
    pub fn undo(&mut self) -> bool {
        if self.history_index > 0 {
            self.history_index -= 1;
            // Follow the behaviour of other text editors and update the location of the cursor
            // with its location in the snapshot we are going back to
            self.current_cursor_path
                .clone_from(&self.root_history[self.history_index].1);
            true
        } else {
            false
        }
    }

    /// Move one step forward in the tree history, return `false` if there was no change to be
    /// redone
    pub fn redo(&mut self) -> bool {
        if self.history_index < self.root_history.len() - 1 {
            self.history_index += 1;
            // Follow the behaviour of other text editors and update the location of the cursor
            // with its location in the snapshot we are going back to
            self.current_cursor_path
                .clone_from(&self.root_history[self.history_index].1);
            true
        } else {
            false
        }
    }

    /* NAVIGATION METHODS */

    /// Returns a reference to the node that is currently the root of the AST.
    pub fn root(&self) -> &'arena Node {
        // This indexing shouldn't panic because we require that `self.history_index` is a valid index
        // into `self.root_history`, and `self.root_history` has at least one element
        self.root_history[self.history_index].0
    }

    /// Returns the cursor node and its direct parent (if such a parent exists)
    pub fn cursor_and_parent(&self) -> (&'arena Node, Option<&'arena Node>) {
        self.current_cursor_path.cursor_and_parent(self.root())
    }

    /// Returns a reference to the node that is currently under the cursor.
    pub fn cursor(&self) -> &'arena Node {
        self.current_cursor_path.cursor(self.root())
    }

    /// Move the cursor in a given direction across the tree.  Returns [`Some`] error string if an
    /// error is found, or [`None`] if the movement was possible.
    pub fn move_cursor(&mut self, direction: Direction) -> Option<String> {
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
                    Some("Cannot move to a sibling of the root.".to_string())
                }
            }
            Direction::Next => {
                if let Some(last_index) = self.current_cursor_path.last_mut() {
                    // We can unwrap here, because the only way for a node to not have a parent is
                    // if it's the root.  And if the cursor is at the root, then the `if let` would
                    // have failed and this code would not be run.
                    if *last_index + 1 < cursor_parent.unwrap().children().len() {
                        *last_index += 1;
                        None
                    } else {
                        Some("Cannot move past the last sibling of a node.".to_string())
                    }
                } else {
                    Some("Cannot move to a sibling of the root.".to_string())
                }
            }
        }
    }

    /* EDITING FUNCTIONS */

    /// Utility function to finish an edit.  This handles removing any redo history, and cloning
    /// the nodes that are parents of the node that changed.
    fn finish_edit(&mut self, nodes_to_clone: &[&'arena Node], new_node: Node) {
        // Remove future trees from the history vector so that the currently 'checked-out' tree is
        // the most recent tree in the history.
        while self.history_index < self.root_history.len() - 1 {
            // TODO: Deallocate the tree so that we don't get a 'memory leak'
            self.root_history.pop();
        }
        // Because AST nodes are immutable, we make changes to nodes by entirely cloning the path
        // down to the node under the cursor.  We do this starting at the node under the cursor and
        // work our way up parent by parent until we reach the root of the tree.  At that point,
        // this node becomes the root of the new tree.
        let mut node = self.arena.alloc(new_node);
        // Iterate backwards over the child indices and the nodes, whilst cloning the tree and
        // replacing the correct child reference to point to the newly created node.
        for (n, child_index) in nodes_to_clone
            .iter()
            .rev()
            .zip(self.current_cursor_path.iter().rev())
        {
            let mut cloned_node = (*n).clone();
            cloned_node.children_mut()[*child_index] = node;
            node = self.arena.alloc(cloned_node);
        }
        // At this point, `node` contains a reference to the root of the new tree, so we just add
        // this to the history, along with the cursor path.
        self.root_history
            .push((node, self.current_cursor_path.clone()));
        // Move the history index on by one so that we are pointing at the latest change
        self.history_index = self.root_history.len() - 1;
    }

    /// Updates the internal state so that the tree now contains `new_node` in the position of the
    /// `cursor`.
    pub fn replace_cursor(&mut self, new_node: Node) {
        // Generate a vec of pointers to the nodes that we will have to clone.  We have to store
        // this as a vec because the iterator that produces them (cursor_path::NodeIter) can only
        // yield values from the root downwards, whereas we need the nodes in the opposite order.
        let mut nodes_to_clone: Vec<_> = self.current_cursor_path.node_iter(self.root()).collect();
        // The last value of nodes_to_clone is the node under the cursor, which we do not need to
        // clone, so we pop that reference.
        assert!(nodes_to_clone.pop().is_some());
        self.finish_edit(&nodes_to_clone, new_node);
    }

    /// Updates the internal state so that the tree now contains `new_node` inserted as the first
    /// child of the selected node.  Also moves the cursor so that the new node is selected.
    pub fn insert_child(&mut self, new_node: Node) -> Result<(), Node::InsertError> {
        // Generate a vec of pointers to the nodes that we will have to clone.  We have to store
        // this as a vec because the iterator that produces them (cursor_path::NodeIter) can only
        // yield values from the root downwards, whereas we need the nodes in the opposite order.
        let mut nodes_to_clone: Vec<_> = self.current_cursor_path.node_iter(self.root()).collect();
        let new_child_node = self.arena.alloc(new_node);
        // Clone the node that currently is the cursor, and add the new child to the end of its
        // children.  Unwrapping here is fine, because `cursor_path::NodeIter` will always return
        // one value.
        let mut cloned_cursor = nodes_to_clone.pop().unwrap().clone();
        // Add the new child to the children of the cloned cursor
        cloned_cursor.insert_child(new_child_node, cloned_cursor.children().len())?;
        self.finish_edit(&nodes_to_clone, cloned_cursor);
        Ok(())
    }

    /// Updates the internal state so that the tree now contains `new_node` inserted as the first
    /// child of the selected node.  Also moves the cursor so that the new node is selected.
    pub fn insert_next_to_cursor(
        &mut self,
        new_node: Node,
        side: Side,
    ) -> Result<(), Node::InsertError> {
        // Generate a vec of pointers to the nodes that we will have to clone.  We have to store
        // this as a vec because the iterator that produces them (cursor_path::NodeIter) can only
        // yield values from the root downwards, whereas we need the nodes in the opposite order.
        let mut nodes_to_clone: Vec<_> = self.current_cursor_path.node_iter(self.root()).collect();
        // Pop the cursor, because it will be unchanged.  The only part of this that we need is
        // the cursor's index.
        assert!(nodes_to_clone.pop().is_some());
        if nodes_to_clone.is_empty() {
            // TODO: Return an error
            log::warn!("Trying to add a sibling to the root!");
            panic!();
        }
        // Find the index of the cursor, so that we know where to insert.  We can unwrap, because
        // if we were at the root, then we'd early return from the if statement above
        let cursor_sibling_index = *self.current_cursor_path.last_mut().unwrap();
        let insert_index = cursor_sibling_index
            + match side {
                Side::Prev => 0,
                Side::Next => 1,
            };
        let new_child_node = self.arena.alloc(new_node);
        // Clone the node that currently is the cursor, and add the new child to the end of its
        // children.  Unwrapping here is fine, because `cursor_path::NodeIter` will always
        // return one value.
        let mut cloned_parent = nodes_to_clone.pop().unwrap().clone();
        // Add the new child to the children of the cloned cursor
        cloned_parent.insert_child(new_child_node, insert_index)?;
        self.finish_edit(&nodes_to_clone, cloned_parent);
        Ok(())
    }

    pub fn delete_cursor(&mut self) -> Result<(), Node::DeleteError> {
        // Generate a vec of pointers to the nodes that we will have to clone.  We have to store
        // this as a vec because the iterator that produces them (cursor_path::NodeIter) can only
        // yield values from the root downwards, whereas we need the nodes in the opposite order.
        let mut nodes_to_clone: Vec<_> = self.current_cursor_path.node_iter(self.root()).collect();
        // Pop the cursor, because it will be unchanged.  The only part of this that we need is
        // the cursor's index.
        assert!(nodes_to_clone.pop().is_some());
        if nodes_to_clone.is_empty() {
            // TODO: Return an error
            log::warn!("Trying to remove the root!");
            panic!();
        }
        // Find the index of the cursor, so that we know where to insert.  We can unwrap, because
        // if we were at the root, then we'd early return from the if statement above
        let cursor_sibling_index = *self.current_cursor_path.last_mut().unwrap();
        let mut cloned_parent = nodes_to_clone.pop().unwrap().clone();
        cloned_parent.delete_child(cursor_sibling_index)?;
        // If we remove the only child of a node then we move the cursor up
        if cloned_parent.children().len() == 0 {
            self.current_cursor_path.pop();
        } else {
            // If we deleted the last child of a node (and this isn't the last child), we move
            // the cursor back by one
            if cursor_sibling_index == cloned_parent.children().len() {
                // We can unwrap here because we know we aren't removing the root
                *self.current_cursor_path.last_mut().unwrap() -= 1;
            }
        }
        // Finish the edit and commit the new tree to the arena
        self.finish_edit(&nodes_to_clone, cloned_parent);
        Ok(())
    }

    /* DISPLAY METHODS */

    /// Build the text representation of the current tree into the given [`String`]
    pub fn write_text(&self, string: &mut String, format: &Node::FormatStyle) {
        self.root().write_text(string, format);
    }

    /// Build and return a [`String`] of the current tree
    pub fn to_text(&self, format: &Node::FormatStyle) -> String {
        let mut s = String::new();
        self.write_text(&mut s, format);
        s
    }
}
