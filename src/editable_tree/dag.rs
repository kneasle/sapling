use super::EditableTree;
use crate::ast_spec::{ASTSpec, NodeMap, ReadableNodeMap};
use crate::vec_node_map::{Index, VecNodeMap};

/// An [`EditableTree`] that stores the history as a DAG (Directed Acyclic Graph).  This means that
/// every node that has ever been created exists somewhere in the DAG, and when changes are made,
/// every node above that is cloned until the root is reached and that root becomes the new root.
/// Therefore, moving back through the history is as simple as reading a different root node from
/// the `roots` vector, and following its descendants through the DAG of nodes.
#[derive(Debug, Clone)]
pub struct DAG<Node: ASTSpec<Index>> {
    node_map: VecNodeMap<Node>,
    roots: Vec<Index>,
}

impl<Node: ASTSpec<Index>> ReadableNodeMap<Index, Node> for DAG<Node> {
    fn get_node(&self, id: Index) -> Option<&Node> {
        self.node_map.get_node(id)
    }

    fn root(&self) -> Index {
        self.node_map.root()
    }
}

impl<Node: ASTSpec<Index>> EditableTree<Index, Node> for DAG<Node> {
    fn new() -> Self {
        let node_map = VecNodeMap::with_default_root();
        DAG {
            roots: vec![node_map.root()],
            node_map,
        }
    }

    fn cursor(&self) -> Index {
        // TODO: Allow the user to move the cursor
        self.root()
    }

    fn replace_cursor(&mut self, new_node: Node) {
        self.node_map.add_as_root(new_node);
    }

    fn insert_child(&mut self, new_node: Node) {
        
    }

    fn write_text(&self, string: &mut String, format: &Node::FormatStyle) {
        self.node_map.write_text(string, format);
    }
}
