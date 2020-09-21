use crate::ast_spec::{ASTSpec, NodeMap, Reference};

// An import solely used by doc-comments
#[allow(unused_imports)]
use crate::editable_tree::EditableTree;

/// A small type used as a reference into Vec-powered [EditableTree]s.  `Index` acts as a type-safe
/// alternative to just using [usize], and can only be created and used by [VecNodeMap]s - to the
/// rest of the code `Indices` are essentially black boxes.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Index(usize);

impl Reference for Index {}

impl Index {
    fn new(val: usize) -> Index {
        Index(val)
    }
}

impl Index {
    #[inline]
    fn as_usize(self) -> usize {
        self.0
    }
}

/// A [NodeMap] that stores all the AST nodes in a [Vec] and uses indices into this [Vec] as IDs
/// for the nodes.
#[derive(Debug, Clone)]
pub struct VecNodeMap<Node> {
    nodes: Vec<Node>,
    root: Index,
}

impl<Node: ASTSpec<Index>> NodeMap<Index, Node> for VecNodeMap<Node> {
    /// Create a new `NodeMap` with a given `Node` as root
    fn with_root(node: Node) -> Self {
        VecNodeMap {
            nodes: vec![node],
            root: Index::new(0),
        }
    }

    /// Get the reference of the root node of the tree
    #[inline]
    fn root(&self) -> Index {
        self.root
    }

    /// Set the root of the tree to be the node at a given reference, returning `true` if the
    /// reference was valid.  If the reference was invalid, the root will not be replaced and
    /// `false` will be returned.
    fn set_root(&mut self, new_root: Index) -> bool {
        let is_ref_valid = self.get_node(new_root).is_some();
        if is_ref_valid {
            self.root = new_root;
        }
        is_ref_valid
    }

    /// Gets node from a reference, returning [None] if the reference is invalid.
    #[inline]
    fn get_node<'a>(&'a self, id: Index) -> Option<&'a Node> {
        self.nodes.get(id.as_usize())
    }

    /// Gets mutable node from a reference, returning [None] if the reference is invalid.
    #[inline]
    fn get_node_mut<'a>(&'a mut self, id: Index) -> Option<&'a mut Node> {
        self.nodes.get_mut(id.as_usize())
    }

    /// Add a new `Node` to the tree, and return its reference
    #[inline]
    fn add_node(&mut self, node: Node) -> Index {
        self.nodes.push(node);
        Index::new(self.nodes.len() - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::{Index, VecNodeMap};
    use crate::ast_spec::{ASTSpec, NodeMap, Reference};

    /// An extremely basic node type, used for testing [VecNodeMap].
    #[derive(Debug, Eq, PartialEq, Hash, Clone)]
    enum ExampleNode<Ref> {
        DefaultValue,
        Value1,
        Value2,
        WithPayload(usize),
        Recursive(Ref),
    }

    impl<Ref: Reference> Default for ExampleNode<Ref> {
        #[inline]
        fn default() -> ExampleNode<Ref> {
            ExampleNode::DefaultValue
        }
    }

    impl<Ref: Reference> ASTSpec<Ref> for ExampleNode<Ref> {
        type FormatStyle = ();

        /// Write the textual representation of this AST to a string
        fn write_text(
            &self,
            node_map: &impl NodeMap<Ref, Self>,
            string: &mut String,
            format_style: &Self::FormatStyle,
        ) {
            match self {
                ExampleNode::DefaultValue => {
                    string.push_str("default");
                }
                ExampleNode::Value1 => {
                    string.push_str("value1");
                }
                ExampleNode::Value2 => {
                    string.push_str("value2");
                }
                ExampleNode::WithPayload(payload) => {
                    string.push_str(&format!("with_payload({})", payload));
                }
                ExampleNode::Recursive(child_ref) => {
                    string.push_str("recursive(");
                    if let Some(node) = node_map.get_node(*child_ref) {
                        node.write_text(node_map, string, format_style);
                    } else {
                        string.push_str(&format!("<INVALID NODE REF {:?}>", child_ref));
                    }
                    string.push(')');
                }
            }
        }

        /// Get an iterator over the direct children of this node
        fn get_children<'a>(&'a self) -> Box<dyn Iterator<Item = Ref> + 'a> {
            match self {
                ExampleNode::DefaultValue
                | ExampleNode::Value1
                | ExampleNode::Value2
                | ExampleNode::WithPayload(_) => Box::new(std::iter::empty()),
                ExampleNode::Recursive(child_ref) => Box::new(std::iter::once(*child_ref)),
            }
        }

        /// Get the display name of this node
        fn get_display_name(&self) -> String {
            match self {
                ExampleNode::DefaultValue => "default",
                ExampleNode::Value1 => "value1",
                ExampleNode::Value2 => "value2",
                ExampleNode::WithPayload(_) => "with_payload",
                ExampleNode::Recursive(_) => "recursive",
            }
            .to_string()
        }

        /// Generate an iterator over the possible shorthand [char]s that a user could type to replace
        /// this node with something else.
        fn get_replace_chars(&self) -> Box<dyn Iterator<Item = char>> {
            Box::new(std::iter::empty())
        }

        /// Generate a new node from a [char] that a user typed as part of the `r` command.  If `c` is
        /// an element of [get_replace_chars](AST::get_replace_chars), this must return `Some` value,
        /// if it isn't, then this can return `None`.
        fn from_replace_char(&self, _c: char) -> Option<Self> {
            None
        }
    }

    /// A useful type alias to make the unit tests terser
    type TestNodeMap = VecNodeMap<ExampleNode<Index>>;

    #[test]
    fn with_root() {
        let node_map: TestNodeMap = VecNodeMap::with_root(ExampleNode::WithPayload(42));

        assert_eq!(
            node_map.get_node(node_map.root()),
            Some(&ExampleNode::WithPayload(42))
        );
    }

    #[test]
    fn with_default_root() {
        let node_map: TestNodeMap = VecNodeMap::with_default_root();

        assert_eq!(
            node_map.get_node(node_map.root()),
            Some(&ExampleNode::default())
        );
    }

    #[test]
    fn get_node_mut() {
        let mut node_map: TestNodeMap = VecNodeMap::with_root(ExampleNode::WithPayload(42));

        node_map
            .get_node_mut(node_map.root())
            .unwrap()
            .clone_from(&ExampleNode::WithPayload(0));

        assert_eq!(
            node_map.get_node(node_map.root()),
            Some(&ExampleNode::WithPayload(0))
        );
    }

    #[test]
    fn add_node() {
        let mut node_map: TestNodeMap = VecNodeMap::with_default_root();

        let r1 = node_map.add_node(ExampleNode::Value1);
        let r2 = node_map.add_node(ExampleNode::Value2);

        assert_eq!(node_map.get_node(r1), Some(&ExampleNode::Value1));
        assert_eq!(node_map.get_node(r2), Some(&ExampleNode::Value2));
    }

    #[test]
    fn manual_set_root() {
        let mut node_map: TestNodeMap = VecNodeMap::with_root(ExampleNode::WithPayload(42));

        let r = node_map.add_node(ExampleNode::Recursive(node_map.root()));
        assert!(node_map.set_root(r));

        if let Some(node) = node_map.get_node(node_map.root()) {
            match node {
                ExampleNode::Recursive(child_ref) => {
                    assert_eq!(
                        node_map.get_node(*child_ref),
                        Some(&ExampleNode::WithPayload(42))
                    );
                }
                _ => {
                    panic!("New root node has the wrong value.");
                }
            }
        } else {
            panic!("New root node not valid.");
        }
    }

    #[test]
    fn set_root() {
        let mut node_map: TestNodeMap = VecNodeMap::with_root(ExampleNode::WithPayload(42));
        node_map.add_as_root(ExampleNode::Recursive(node_map.root()));

        if let Some(node) = node_map.get_node(node_map.root()) {
            match node {
                ExampleNode::Recursive(child_ref) => {
                    assert_eq!(
                        node_map.get_node(*child_ref),
                        Some(&ExampleNode::WithPayload(42))
                    );
                }
                _ => {
                    panic!("New root node has the wrong value.");
                }
            }
        } else {
            panic!("New root node not valid.");
        }
    }
}
