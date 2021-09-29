/// A struct representing the location of Sapling's cursors.  Each cursor in Sapling selects a node
/// and, implicitly, the sub-tree rooted at that node.  As such, it doesn't make any sense for a
/// cursor to be a descendant of another cursor (because then part of the tree is selected twice).
/// In other words, each path from the root to a leaf must contain at most one cursor.
/// Additionally, there must always be at least one cursor in existence - even if this cursor is
/// selecting the whole tree.
///
/// Note how these properties are inductive - if the cursor properties hold for all children of a
/// node (or the node itself is a cursor), then the properties also apply to a node.
///
/// All of these constraints are enforced by this datatype, and non-conforming `Cursors` can't be
/// represented.  A `Cursors` struct exists for every node which has any cursors in its descendants
/// (including itself); any node which doesn't contain a cursor will not have a corresponding
/// `Cursors` struct.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Cursors {
    /// The paths descending from this node which contain cursors (paired with their indices).
    ///
    /// _There is a cursor in this location if and only if `children` is empty._
    ///
    /// **Invariant**:
    /// - These are stored in ascending order by index
    children: Vec<(usize, Cursors)>,
}

impl Cursors {
    /// Creates a set of `Cursors`, containing one cursor at the tree root.
    pub fn cursor() -> Self {
        Self { children: vec![] }
    }

    /// `true` if the set of `Cursors` at this node is just a single cursor
    pub fn is_cursor(&self) -> bool {
        self.children.is_empty()
    }

    /// Move all the cursors up `n` level in the tree, capping out at current node.  If two cursors
    /// would end up in the same path from the root, then the lower one is merged into the higher
    /// one:
    /// ```text
    ///        +---------+                 +---------+      +---------+
    ///        |      /  |                 |      /  |      |      /  |
    ///        |     .   |                 |     #   |      |     #   |
    ///        |    / \  |                 |         |      |    /    |
    /// Moving |   .   # | up by one makes |         |, not |   #     |
    ///        |  /      |                 |         |      |         |
    ///        | #       |                 |         |      |         |
    ///        +---------+                 +---------+      +---------+
    /// ```
    ///
    /// The time complexity of this operation is linear in the size of `self`, and constant in `n`.
    pub fn move_up(&mut self, n: usize) {
        self.move_up_returning_min_depth(n);
    }

    /// Same as [`Self::move_up`], but returns the depth of the shallowest cursor below this node
    /// (where `self` is at depth 0).  This extra information is used by parent nodes to determine
    /// where their cursors will end up.
    fn move_up_returning_min_depth(&mut self, n: usize) -> usize {
        match self
            .children
            .iter_mut()
            .map(|(_, child)| child.move_up_returning_min_depth(n))
            .min()
            .map(|depth| depth + 1) // This node is 1 level higher than its children
        {
            Some(min_depth) => {
                // If `min_depth < n`, then the shallowest cursor will get moved above `self` (and
                // any nodes that might get moved to `self` will get merged into that higher node).
                // If `min_depth > n`, then no cursors can be merged into `self` because they are
                // all too far away.  In either case, no further changes to `self` are required.
                if min_depth == n {
                    // If the nearest cursor is `n` steps away, then that cursor will be moved into
                    // onto `self` and any deeper cursors will get merged into `self`.
                    self.children.clear();
                }
                min_depth
            }
            // `self.children` must be empty, so `self` is a cursor and therefore can't be
            // moved up
            None => 0,
        }
    }
}

#[cfg(test)]
mod test {
    use super::Cursors;

    /// Macro to generate a set of cursors.  This doesn't enforce the child ordering invariant, so
    /// isn't safe for general use.
    macro_rules! cur {
        ( $( $idx: literal => $child: expr ),* ) => {
            Cursors {
                children: vec![$( ($idx, $child) ),*],
            }
        };
    }

    #[test]
    fn is_cursor() {
        assert!(cur!().is_cursor());
        assert!(!cur!(0 => cur!()).is_cursor());
    }

    #[test]
    fn move_up() {
        #[track_caller]
        fn check(mut cs_before: Cursors, levels: usize, cs_after: Cursors) {
            cs_before.move_up(levels);
            assert_eq!(cs_before, cs_after);
        }

        check(cur!(), 1, cur!()); // The root can't be moved up
        check(cur!(), 3, cur!()); // The root can't be moved up
        check(cur!(0 => cur!()), 1, cur!());
        // Merging two nodes together
        check(cur!(0 => cur!(), 1 => cur!()), 1, cur!());
        // Check node merging at different heights
        check(
            cur!(
                0 => cur!(),
                1 => cur!(4 => cur!(1 => cur!()))
            ),
            1,
            cur!(),
        );
        check(
            cur!(
                0 => cur!(1 => cur!(1 => cur!())),
                1 => cur!(4 => cur!(1 => cur!()))
            ),
            2,
            cur!(0 => cur!(), 1 => cur!()),
        );
        check(
            cur!(
                0 => cur!(1 => cur!(1 => cur!())),
                1 => cur!(4 => cur!(1 => cur!()))
            ),
            3,
            cur!(),
        );
    }
}
