//! Code for an editable, undoable forest of syntax trees.

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

/// An enum to represent the two sides of a node
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Side {
    Prev,
    Next,
}

impl Side {
    /// Converts this `Side` into either `"before"` or `"after"`
    pub fn relational_word(&self) -> &'static str {
        match self {
            Side::Prev => "before",
            Side::Next => "after",
        }
    }
}

/// An enum that's returned when any of the 'edit' methods in [`DAG`] are successful.
pub enum EditSuccess {
    Undo,
    Redo,
    Replace { c: char, name: String },
    InsertChild { c: char, name: String },
    InsertNextToCursor { side: Side, c: char, name: String },
}

impl EditSuccess {
    /// Writes an info message of a successful action using `info!`
    fn log_message(self) {
        match self {
            EditSuccess::Undo => log::info!("Undoing one change"),
            EditSuccess::Redo => log::info!("Redoing one change"),
            EditSuccess::Replace { c, name } => log::info!("Replacing with '{}'/{}", c, name),
            EditSuccess::InsertChild { c, name } => {
                log::info!("Inserting '{}'/{} as new child", c, name)
            }
            EditSuccess::InsertNextToCursor { side, c, name } => log::info!(
                "Inserting '{}'/{} {} the cursor",
                c,
                name,
                side.relational_word()
            ),
        }
    }
}

/// An error that represents an error in any of the 'edit' methods in [`DAG`].
pub enum EditErr {
    /// Trying to undo the earliest change
    NoChangesToUndo,
    /// Trying to redo the latest change
    NoChangesToRedo,
    /// The user typed a char that doesn't correspond to any node
    CharNotANode(char),
    /// Trying to replace a node with one that cannot be a child of its parent
    CannotReplace(char),
    /// Trying to insert a node that cannot be a child of the cursor
    CannotInsertInto { c: char, parent_name: String },
    /// Trying to add a sibling to the root
    AddSiblingToRoot,
}

impl EditErr {
    /// Writes an warning message of the encountered error using either `warn!` or `error!`,
    /// depending on the severity of the error
    fn log_message(self) {
        match self {
            EditErr::NoChangesToUndo => log::warn!("No changes to undo."),
            EditErr::NoChangesToRedo => log::warn!("No changes to redo."),
            EditErr::CharNotANode(c) => log::warn!("'{}' doesn't correspond to any node type.", c),
            EditErr::CannotReplace(c) => log::warn!("Can't replace cursor with '{}'", c),
            EditErr::CannotInsertInto { c, parent_name } => {
                log::warn!("Can't insert '{}' into {}", c, parent_name)
            }
            EditErr::AddSiblingToRoot => log::warn!("Can't add siblings to the root."),
        }
    }
}

/// An alias for [`Result`] that is the return type of all of [`DAG`]'s edit methods.
pub type EditResult = Result<EditSuccess, EditErr>;

/// A trait-extension that provides a convenient way convert [`EditResult`]s into log messages.
pub trait LogMessage {
    /// Log the current result's message to the appropriate log channel.
    fn log_message(self);
}

impl LogMessage for EditResult {
    /// Consumes this `EditResult` and logs an appropriate summary report (using `info!` for
    /// [`EditOk`]s and `warn!` or `error!` for [`EditErr`]s).
    fn log_message(self) {
        match self {
            Ok(ok) => ok.log_message(),
            Err(err) => err.log_message(),
        }
    }
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

    /* HISTORY METHODS */

    /// Move one step back in the tree history
    pub fn undo(&mut self) -> EditResult {
        // Early return if there are no changes to undo
        if self.history_index == 0 {
            return Err(EditErr::NoChangesToUndo);
        }
        // Move the history index back by one to perform the undo
        self.history_index -= 1;
        // Follow the behaviour of other text editors and update the location of the cursor
        // with its location in the snapshot we are going back to
        self.current_cursor_path
            .clone_from(&self.root_history[self.history_index].1);
        Ok(EditSuccess::Undo)
    }

    /// Move one step forward in the tree history
    pub fn redo(&mut self) -> EditResult {
        // Early return if there are no changes to redo
        if self.history_index >= self.root_history.len() - 1 {
            return Err(EditErr::NoChangesToRedo);
        }
        // Move the history index forward by one to perform the redo
        self.history_index += 1;
        // Follow the behaviour of other text editors and update the location of the cursor
        // with its location in the snapshot we are going back to
        self.current_cursor_path
            .clone_from(&self.root_history[self.history_index].1);
        Ok(EditSuccess::Redo)
    }

    /* EDITING METHODS */

    /// Utility function to finish an edit.  This handles removing any redo history, and cloning
    /// the nodes that are parents of the node that changed.
    fn finish_edit(
        &mut self,
        nodes_to_clone: &[&'arena Node],
        steps_above_cursor: usize,
        new_node: Node,
    ) {
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
        // SANITY CHECK: Assert that items_to_clone and self.cursor_path have the same length once
        // steps_above_cursor have been taken off - i.e. that we aren't losing any information by
        // zipping the two things together
        if nodes_to_clone.len() != self.current_cursor_path.depth() - steps_above_cursor {
            panic!(
                "`nodes_to_clone` ({:?}) has a different length to `self.cursor_path` ({:?}) with \
{} items popped.",
                nodes_to_clone, self.current_cursor_path, steps_above_cursor
            );
        }
        // Iterate backwards over the child indices and the nodes, whilst cloning the tree and
        // replacing the correct child reference to point to the newly created node.
        for (n, child_index) in nodes_to_clone.iter().rev().zip(
            self.current_cursor_path
                .iter()
                .rev()
                .skip(steps_above_cursor),
        ) {
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

    /// Replaces the current cursor with a node represented by `c`
    pub fn replace_cursor(&mut self, c: char) -> EditResult {
        /* CHECK THE VALIDITY OF ARGUMENTS */

        // Cache the node under the cursor, since finding the cursor involves non-trivial amounts
        // of work
        let cursor = self.cursor();
        // Short circuit if the char to insert couldn't correspond to a valid child
        if !cursor.is_replace_char(c) {
            return Err(EditErr::CannotReplace(c));
        }

        /* PERFORM THE ACTION */

        let new_node = cursor.from_char(c).ok_or(EditErr::CharNotANode(c))?;
        // Generate a vec of pointers to the nodes that we will have to clone.  We have to store
        // this as a vec because the iterator that produces them (cursor_path::NodeIter) can only
        // yield values from the root downwards, whereas we need the nodes in the opposite order.
        let mut nodes_to_clone: Vec<_> = self.current_cursor_path.node_iter(self.root()).collect();
        // The last value of nodes_to_clone is the node under the cursor, which we do not need to
        // clone, so we pop that reference.
        assert!(nodes_to_clone.pop().is_some());

        /* FINISH THE EDIT AND RETURN SUCCESS */

        // Store the new_node's display name before it's consumed by `finish_edit`
        let new_node_name = new_node.display_name();
        self.finish_edit(&nodes_to_clone, 0, new_node);
        Ok(EditSuccess::Replace {
            c,
            name: new_node_name,
        })
    }

    /// Updates the internal state so that the tree now contains `new_node` inserted as the last
    /// child of the selected node.  Also moves the cursor so that the new node is selected.
    pub fn insert_child(&mut self, c: char) -> EditResult {
        /* CHECK THE VALIDITY OF ARGUMENTS */

        // Cache the node under the cursor, since finding the cursor involves non-trivial amounts
        // of work
        let cursor = self.cursor();
        // Short circuit if `c` couldn't be a valid child of the cursor
        if !cursor.is_insert_char(c) {
            return Err(EditErr::CannotInsertInto {
                c,
                parent_name: cursor.display_name(),
            });
        }

        /* PERFORM THE ACTION */

        let new_node = self
            .arena
            .alloc(cursor.from_char(c).ok_or(EditErr::CharNotANode(c))?);
        // Generate a vec of pointers to the nodes that we will have to clone.  We have to store
        // this as a vec because the iterator that produces them (cursor_path::NodeIter) can only
        // yield values from the root downwards, whereas we need the nodes in the opposite order.
        let mut nodes_to_clone: Vec<_> = self.current_cursor_path.node_iter(self.root()).collect();
        // Clone the node that currently is the cursor, and add the new child to the end of its
        // children.  Unwrapping here is fine, because `cursor_path::NodeIter` will always return
        // one value.
        let mut cloned_cursor = nodes_to_clone.pop().unwrap().clone();
        // Store the new_node's display name before it's consumed by `finish_edit`
        let new_node_name = new_node.display_name();
        // Add the new child to the children of the cloned cursor
        cloned_cursor.insert_child(new_node, self.arena, cloned_cursor.children().len());

        /* FINISH THE EDIT AND RETURN SUCCESS */

        self.finish_edit(&nodes_to_clone, 0, cloned_cursor);
        Ok(EditSuccess::InsertChild {
            c,
            name: new_node_name,
        })
    }

    /// Updates the internal state so that the tree now contains `new_node` inserted as the first
    /// child of the selected node.  Also moves the cursor so that the new node is selected.
    pub fn insert_next_to_cursor(&mut self, c: char, side: Side) -> EditResult {
        /* CHECK VALIDITY OF ARGUMENTS */

        // Find (and cache) the parent of the cursor.  If the parent of the cursor doesn't exist,
        // the cursor must be the root and we can't insert a node next to the root.
        let parent = match self.cursor_and_parent().1 {
            None => return Err(EditErr::AddSiblingToRoot),
            Some(p) => p,
        };
        // Short circuit if not an insertable char
        if !parent.is_insert_char(c) {
            return Err(EditErr::CannotInsertInto {
                c,
                parent_name: parent.display_name(),
            });
        }

        /* PERFORM THE INSERTION */

        // Generate a vec of pointers to the nodes that we will have to clone.  We have to store
        // this as a vec because the iterator that produces them (cursor_path::NodeIter) can only
        // yield values from the root downwards, whereas we need the nodes in the opposite order.
        let mut nodes_to_clone: Vec<_> = self.current_cursor_path.node_iter(self.root()).collect();
        // Pop the cursor, because it will be unchanged.  The only part of this that we need is
        // the cursor's index.
        assert!(nodes_to_clone.pop().is_some());
        // Find the index of the cursor, so that we know where to insert.  We can unwrap, because
        // if we were at the root, then we'd early return from the if statement above
        let cursor_sibling_index = *self.current_cursor_path.last_mut().unwrap();
        let insert_index = cursor_sibling_index
            + match side {
                Side::Prev => 0,
                Side::Next => 1,
            };
        // Clone the node that currently is the cursor, and add the new child to the end of its
        // children.  Unwrapping here is fine, because `cursor_path::NodeIter` will always
        // return one value.
        let mut cloned_parent = nodes_to_clone.pop().unwrap().clone();
        // Create the new child node according to the given char.
        let new_child_node = self
            .arena
            .alloc(cloned_parent.from_char(c).ok_or(EditErr::CharNotANode(c))?);
        // Store the new_node's display name before it's consumed by `insert_child`
        let new_node_name = new_child_node.display_name();
        // Add the new child to the children of the cloned cursor
        cloned_parent.insert_child(new_child_node, self.arena, insert_index);

        /* FINISH THE EDIT AND RETURN SUCCESS */

        // Finish the edit and update the history
        self.finish_edit(&nodes_to_clone, 1, cloned_parent);
        // Return the success
        Ok(EditSuccess::InsertNextToCursor {
            side,
            c,
            name: new_node_name,
        })
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
        self.finish_edit(&nodes_to_clone, 0, cloned_parent);
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
