use super::json::JSON;
use crate::ast_spec::{NodeMap, Reference};

/// A copy of [`JSON`] that does not rely on a [`NodeMap`] for recursive types
pub enum TestJSON {
    True,
    False,
    Array(Vec<TestJSON>),
    Object(Vec<(String, TestJSON)>),
}

impl TestJSON {
    fn recursive_add_node_to_map<Ref: Reference, M: NodeMap<Ref, JSON<Ref>>>(
        &self,
        map: &mut M,
    ) -> Ref {
        match self {
            TestJSON::True => map.add_node(JSON::True),
            TestJSON::False => map.add_node(JSON::False),
            TestJSON::Array(child_nodes) => {
                let child_refs = child_nodes
                    .iter()
                    .map(|x| x.recursive_add_node_to_map(map))
                    .collect::<Vec<Ref>>();
                map.add_node(JSON::Array(child_refs))
            }
            TestJSON::Object(fields) => {
                let mut children = Vec::with_capacity(fields.len());
                for (key, value) in fields.iter() {
                    // Add both child nodes
                    let s = map.add_node(JSON::Str(key.clone()));
                    let v = value.recursive_add_node_to_map(map);
                    // Combine the two nodes into a fields
                    children.push(map.add_node(JSON::Field(s, v)));
                }
                map.add_node(JSON::Object(children))
            }
        }
    }

    /// Turn this node into a [`VecNodeMap`] which contains the corresponding [`JSON`] node as
    /// root. This also adds all the children to that VecNodeMap.
    pub fn build_node_map<Ref: Reference, M: NodeMap<Ref, JSON<Ref>>>(&self) -> M {
        let mut node_map = M::with_default_root();
        let root = self.recursive_add_node_to_map(&mut node_map);
        node_map.set_root(root);
        node_map
    }
}
