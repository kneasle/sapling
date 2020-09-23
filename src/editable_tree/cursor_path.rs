use crate::ast_spec::Reference;

// Imports solely for doc comments
#[allow(unused_imports)]
use crate::editable_tree::EditableTree;
#[allow(unused_imports)]
use crate::editable_tree::dag::DAG;

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
