//! A module containing a way of representing the **location** of a node within a tree.

use crate::ast::Ast;

/// A tree-independent struct for representing the locations of nodes within trees.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Path {
    child_indices: Vec<usize>,
}

impl Path {
    /// Creates a cursor path from a given [`Vec`]
    #[inline]
    pub fn from_vec(vec: Vec<usize>) -> Path {
        Path { child_indices: vec }
    }

    /// Creates a cursor path that refers to the root of any tree.
    #[inline]
    pub fn root() -> Path {
        Self::from_vec(vec![])
    }

    /// Return the depth of the cursor in the tree.  The root has depth `0`.
    pub fn depth(&self) -> usize {
        self.child_indices.len()
    }

    /// Walks this path down from the given root, and returns the node that lies underneath the
    /// cursor.
    #[inline]
    pub fn cursor<'arena, Node: Ast<'arena>>(&self, root: &'arena Node) -> &'arena Node {
        // We can unwrap here because `NodeIter` is guarunteed to return at least one value
        // (the first value it returns is always the root that we gave it).
        self.node_iter(root).last().unwrap()
    }

    /// Walks this path down from the given root, and returns the node that lies underneath the
    /// cursor, along with the direct parent of that node (if it exists).
    pub fn cursor_and_parent<'arena, Node: Ast<'arena>>(
        &self,
        root: &'arena Node,
    ) -> (&'arena Node, Option<&'arena Node>) {
        // Track the current node, and the parent of that node
        let mut node = root;
        let mut parent: Option<&Node> = None;
        // Step down the iterator.
        // Invariant: At the end of every iteration, `parent` holds the parent of `node` (if that
        //            parent exists).
        for next_node in self.node_iter(root).skip(1) {
            parent = Some(node);
            node = next_node;
        }
        // Since `node` == `self.cursor()` and the invariant holds, this returns the cursor and its
        // parent
        (node, parent)
    }

    /// Pushes a new child onto the path.  This has the effect of moving the cursor one level down
    /// the tree, to the `new_child_index`th child of the node the `Path` is currently
    /// pointing at.
    #[inline]
    pub fn push(&mut self, new_child_index: usize) {
        self.child_indices.push(new_child_index);
    }

    /// Removes and returns the last child from the path (if it exists).  This has the effect of
    /// moving the cursor to its parent, and returning the old cursor's sibling index.  Returns
    /// `None` if the path referred to the root of the tree (and therefore the pop had no effect).
    #[inline]
    pub fn pop(&mut self) -> Option<usize> {
        self.child_indices.pop()
    }

    /// Returns `true` if this path refers to the root of any tree (i.e. the path has no segments).
    #[inline]
    pub fn is_root(&self) -> bool {
        self.child_indices.is_empty()
    }

    /// Returns the last child index in the path (if it exists).
    #[inline]
    pub fn last(&self) -> Option<usize> {
        self.child_indices.last().copied()
    }

    /// Returns a mutable reference to the last child index in the path (if it exists).
    #[inline]
    pub fn last_mut(&mut self) -> Option<&mut usize> {
        self.child_indices.last_mut()
    }

    /// Returns an iterator over the child indices contained in this path.
    #[inline]
    pub fn iter(&self) -> std::slice::Iter<'_, usize> {
        self.child_indices.iter()
    }

    /// Returns an iterator over the AST `Node`s generated when this path is traversed starting
    /// with a given root.
    #[inline]
    pub fn node_iter<'arena, 'p, Node>(&'p self, root: &'arena Node) -> NodeIter<'arena, 'p, Node>
    where
        Node: Ast<'arena>,
    {
        NodeIter::new(root, &self)
    }
}

/// An iterator that walks down a tree following a [`Path`].  The first item returned from
/// this iterator is always the root of the tree.  As a consequence, this yields one more AST node
/// than the original tree had.
pub struct NodeIter<'arena, 'p, Node>
where
    Node: Ast<'arena>,
{
    node: Option<&'arena Node>,
    iter: std::slice::Iter<'p, usize>,
}

impl<'arena, 'p, Node> NodeIter<'arena, 'p, Node>
where
    Node: Ast<'arena>,
{
    /// Creates a new `NodeIter` that follows a given [`Path`] down from a root `Node`.  This
    /// will only be called from [`Path::node_iter`].
    #[inline]
    fn new(root: &'arena Node, path: &'p Path) -> Self {
        NodeIter {
            node: Some(root),
            iter: path.iter(),
        }
    }

    /// Helper function that picks the correct descendant of `self.node`, returning `None` if any
    /// of the following conditions happen:
    /// - the path has finished (there are no more child indices to read)
    /// - the child index has a value too big to represent a valid child of the current node
    /// - self.node = None; i.e. the iterator has already finished.  This condition means that this
    ///   iterator is **fused** (see [`std::iter::Fuse`])
    ///
    /// # Panics
    /// This will panic if the cursor path reaches a point where a node does not have enough
    /// children for the path to continue.  This follows the garbage-in, immediately-panic strategy.
    #[inline]
    fn next_descendant(&mut self) -> Option<&'arena Node> {
        Some(
            *self
                .node?
                .children()
                .get(*self.iter.next()?)
                .expect("cursor path points to a non-existent node."),
        )
    }
}

impl<'arena, 'iter, Node> Iterator for NodeIter<'arena, 'iter, Node>
where
    Node: Ast<'arena>,
{
    type Item = &'arena Node;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let last_node = self.node;
        self.node = self.next_descendant();
        last_node
    }
}

// Because of how [`NodeIter::next_descendant`] works, NodeIter is a fused iterator - it will
// never return Some(x) after the first None.
impl<'arena, Node: Ast<'arena>> std::iter::FusedIterator for NodeIter<'arena, '_, Node> {}

#[cfg(test)]
mod tests {
    use super::Path;
    use crate::arena::Arena;
    use crate::ast::json::{add_value_to_arena, Json};
    use crate::ast::Ast;

    use serde_json::json;

    #[test]
    fn is_root() {
        // A path that's created as a root is ... a root!
        let mut path = Path::root();
        assert!(path.is_root());
        // Moving to any child stops it being a root
        path.push(0);
        assert!(!path.is_root());
        // Pushing more children means it still isn't a root
        path.push(4);
        assert!(!path.is_root());
        // If we pop *one* child, we're still not back at the root
        assert_eq!(path.pop(), Some(4));
        assert!(!path.is_root());
        // But if we pop the last child, we get back to the root
        assert_eq!(path.pop(), Some(0));
        assert!(path.is_root());
    }

    #[test]
    fn last_and_last_mut() {
        // A path that's pointing to the root has no last
        let mut path = Path::root();
        assert_eq!(path.last(), None);
        assert_eq!(path.last_mut(), None);
        // If we push an index, then that should be that 'last'
        path.push(0);
        assert_eq!(path.last(), Some(0));
        assert_eq!(path.last_mut().copied(), Some(0));
        // If we change the last bit of the path, then `path.last()` should also change
        *path.last_mut().unwrap() = 5;
        assert_eq!(path.last(), Some(5));
        assert_eq!(path.last_mut().copied(), Some(5));
        // Push some more
        path.push(3);
        assert_eq!(path.last(), Some(3));
        assert_eq!(path.last_mut().copied(), Some(3));
        // Pop the last thing
        assert_eq!(path.pop(), Some(3));
        assert_eq!(path.last(), Some(5));
        assert_eq!(path.last_mut().copied(), Some(5));
        // Pop to the root
        assert_eq!(path.pop(), Some(5));
        assert_eq!(path.last(), None);
        assert_eq!(path.last_mut(), None);
    }

    #[test]
    fn depth() {
        // A path that's created as a root is ... a root!
        let mut path = Path::root();
        assert_eq!(path.depth(), 0);
        // Moving to any child stops it being a root
        path.push(0);
        assert_eq!(path.depth(), 1);
        // Pushing more children means it still isn't a root
        path.push(4);
        assert_eq!(path.depth(), 2);
        // If we pop *one* child, we're still not back at the root
        assert_eq!(path.pop(), Some(4));
        assert_eq!(path.depth(), 1);
        // But if we pop the last child, we get back to the root
        assert_eq!(path.pop(), Some(0));
        assert_eq!(path.depth(), 0);
    }

    #[test]
    fn node_iter() {
        // Create some test Json and add it to an arena
        let arena = Arena::new();
        let root = add_value_to_arena(json!([true, false, { "value": true }]), &arena);
        // Create a path to the root, and test properties of it
        let mut path = Path::root();
        assert_eq!(path.node_iter(root).collect::<Vec<&Json<'_>>>(), [root]);
        // Move the path to the 2nd ('1th') child (which is 'false')
        path.push(1);
        assert_eq!(
            path.node_iter(root)
                .map(|x| x.display_name())
                .collect::<Vec<_>>(),
            vec!["array".to_string(), "false".to_string()]
        );
        // Move to the ':' field object
        *path.last_mut().unwrap() = 2;
        path.push(0);
        assert_eq!(
            path.node_iter(root)
                .map(|x| x.display_name())
                .collect::<Vec<_>>(),
            vec![
                "array".to_string(),
                "object".to_string(),
                "field".to_string()
            ]
        );
        // Move down to the 'true' in the object
        path.push(1);
        assert_eq!(
            path.node_iter(root)
                .map(|x| x.display_name())
                .collect::<Vec<_>>(),
            vec![
                "array".to_string(),
                "object".to_string(),
                "field".to_string(),
                "true".to_string()
            ]
        );
        // Pop back up two levels to the object
        assert_eq!(path.pop(), Some(1));
        assert_eq!(path.pop(), Some(0));
        assert_eq!(
            path.node_iter(root)
                .map(|x| x.display_name())
                .collect::<Vec<_>>(),
            vec!["array".to_string(), "object".to_string(),]
        );
    }

    #[test]
    fn cursor() {
        // Create some test Json and add it to an arena
        let arena = Arena::new();
        let root = add_value_to_arena(json!([true, false, { "value": true }]), &arena);
        // Create a path to the root, and check that the cursor is the root
        let mut path = Path::root();
        assert!(std::ptr::eq(path.cursor(root), root));
        // Move down to the 'false' object
        path.push(1);
        assert_eq!(path.cursor(root).display_name(), "false");
        // Move to the ':' field object
        *path.last_mut().unwrap() = 2;
        path.push(0);
        assert_eq!(path.cursor(root).display_name(), "field");
        // Move down to the 'true' in the object
        path.push(1);
        assert_eq!(path.cursor(root).display_name(), "true");
    }

    #[test]
    fn cursor_and_parent() {
        // Create some test Json and add it to an arena
        let arena = Arena::new();
        let root = add_value_to_arena(json!([true, false, { "value": true }]), &arena);
        // Create a path to the root.  The root has no parent
        let mut path = Path::root();
        assert_eq!(path.cursor_and_parent(root), (root, None));
        // Move down to the 'false' object.  Now the parent is the root
        path.push(1);
        let (c, p) = path.cursor_and_parent(root);
        assert_eq!(c.display_name(), "false");
        assert!(std::ptr::eq(p.unwrap(), root));
        // Move to the ':' field object, whos parent is an 'object'
        *path.last_mut().unwrap() = 2;
        path.push(0);
        let (c, p) = path.cursor_and_parent(root);
        assert_eq!(c.display_name(), "field");
        assert_eq!(p.unwrap().display_name(), "object");
        // Move down to the 'true' in the object
        path.push(1);
        let (c, p) = path.cursor_and_parent(root);
        assert_eq!(c.display_name(), "true");
        assert_eq!(p.unwrap().display_name(), "field");
    }
}
