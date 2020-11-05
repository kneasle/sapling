use crate::ast_spec::ASTSpec;
use crate::node_map::{NodeMap, Reference};

// Imports solely for doc comments
#[allow(unused_imports)]
use crate::editable_tree::dag::DAG;
#[allow(unused_imports)]
use crate::editable_tree::{Direction, EditableTree};

/// One part of a path from the root of a tree to the cursor.
///
/// A [`Vec`] of these allows the [`DAG`] [`EditableTree`] to climb back up the trees to the root
/// without having to keep backpointers updated in the DAG - keeping backpointers inside a **DAG**
/// is particularly probablematic since each node can (and often will) have multiple parents and
/// therefore it's very badly defined which one to use.
///
/// Each `Segment` contains a reference to the node at that point in the path, and an index that
/// determines which of the parent's children this path segment refers to.  For the first element
/// (which is the tree's root) this index doesn't matter.  So for example,
/// suppose we have the following tree, where we are calculating `NODE8`'s path:
///
/// ```text
/// ROOT
///   NODE1
///     NODE2
///       NODE3
///       NODE4
///     NODE5
///       NODE6
///       NODE7
///       [NODE8]
///       NODE9
///   NODE10
/// ```
///
/// The corresponding list of `Segment` would look like this:
///
/// ```text
/// [
///     Segment { node: <ROOT>, sibling_index: 0 },
///     Segment { node: <NODE1>, sibling_index: 0 },
///     Segment { node: <NODE3>, sibling_index: 1 },
///     Segment { node: <NODE8>, sibling_index: 2 },
/// ]
/// ```
#[derive(Debug, Clone)]
pub(super) struct Segment<Ref: Reference> {
    pub node: Ref,
    pub sibling_index: usize,
}

impl<Ref: Reference> Segment<Ref> {
    /// Constructs a new `CursorLocationSegment` from its component parts
    pub fn new(node_index: Ref, sibling_index: usize) -> Self {
        Segment {
            node: node_index,
            sibling_index,
        }
    }

    /// Constructs a `CursorLocationSegment` that is correct for representing the root of a tree
    pub fn root(node_index: Ref) -> Self {
        Self::new(node_index, 0)
    }
}

/// An encapsulation of a full path of [`Segment`]s, which asserts the following invariants:
///
/// - A path always has at least one [`Segment`].
#[derive(Debug, Clone)]
pub(super) struct SegPath<Ref: Reference> {
    segments: Vec<Segment<Ref>>,
}

impl<Ref: Reference> SegPath<Ref> {
    pub fn with_root(root: Ref) -> SegPath<Ref> {
        SegPath {
            segments: vec![Segment::new(root, 0)],
        }
    }

    pub fn node(&self) -> Ref {
        self.segments.last().unwrap().node
    }

    pub fn _move<Node: ASTSpec<Ref>>(
        &mut self,
        direction: Direction,
        node_map: &impl NodeMap<Ref, Node>,
    ) -> Option<String> {
        match direction {
            Direction::Up => {
                if self.segments.len() == 1 {
                    return Some("Already at the tree root.".to_string());
                }
                self.segments.pop();
            }
            Direction::Down => match node_map.get_node(self.node()).unwrap().children().get(0) {
                Some(node) => {
                    self.segments.push(Segment::new(*node, 0));
                }
                None => {
                    return Some("Current node has no children.".to_string());
                }
            },
            _ => {
                return Some(format!("Direction {:?} not implemented yet.", direction));
            }
        }

        None
    }
}
