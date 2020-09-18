use super::{ASTSpec, NodeMap};
use crate::editable_tree::reference::Ref;

pub struct VecNodeMap<Node> {
    nodes: Vec<Node>,
    root: Ref,
}

impl<Node: ASTSpec<Ref>> NodeMap<Ref, Node> for VecNodeMap<Node> {
    /// Create a new `NodeMap` with a given `Node` as root
    fn with_root(node: Node) -> Self {
        VecNodeMap {
            nodes: vec![node],
            root: Ref::from(0),
        }
    }

    /// Get the reference of the root node of the tree
    #[inline]
    fn root(&self) -> Ref {
        self.root
    }

    /// Gets node from a reference, returning [None] if the reference is invalid.
    #[inline]
    fn get_node<'a>(&'a self, id: Ref) -> Option<&'a Node> {
        self.nodes.get(id.as_usize())
    }

    /// Gets mutable node from a reference, returning [None] if the reference is invalid.
    #[inline]
    fn get_node_mut<'a>(&'a mut self, id: Ref) -> Option<&'a mut Node> {
        self.nodes.get_mut(id.as_usize())
    }

    /// Add a new `Node` to the tree, and return its reference
    #[inline]
    fn add_node(&mut self, node: Node) -> Ref {
        self.nodes.push(node);
        Ref::from(self.nodes.len() - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::VecNodeMap;
    use crate::ast_spec::{ASTSpec, NodeMap, Reference};
    use crate::editable_tree::reference::Ref;

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
    type TestNodeMap = VecNodeMap<ExampleNode<Ref>>;

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

        assert_eq!(node_map.get_node(node_map.root()), Some(&ExampleNode::WithPayload(0)));
    }

    #[test]
    fn add_node() {
        let mut node_map: TestNodeMap = VecNodeMap::with_default_root();

        let r1 = node_map.add_node(ExampleNode::Value1);
        let r2 = node_map.add_node(ExampleNode::Value2);

        assert_eq!(node_map.get_node(r1), Some(&ExampleNode::Value1));
        assert_eq!(node_map.get_node(r2), Some(&ExampleNode::Value2));
    }
}
