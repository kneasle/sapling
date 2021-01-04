//! Another representation of JSON trees (where nodes own their children), useful for unit tests
//! and debugging.

use super::json::Json;
use crate::arena::Arena;

/// A copy of [`Json`] where nodes own their children
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum TestJson {
    /// The constant literal `true`.  Converts to [`Json::True`]
    True,
    /// The constant literal `false`.  Converts to [`Json::False`]
    False,
    /// The constant literal `null`.  Converts to [`Json::Null`]
    Null,
    /// A JSON array `[child1, child2, ..., childN]`.  Converts to [`Json::Array`]
    Array(Vec<TestJson>),
    /// A JSON object `{key1: child1, key2: child2, ..., keyN: childN}`.  Converts to a
    /// [`Json::Object`] with `N` [`Json::Field`]s containing the key/value pairs.
    Object(Vec<(String, TestJson)>),
    /// A JSON 'String' literal
    Str(String),
}

impl TestJson {
    /// Convert this node into a standard [`Json`] tree, where all the nodes are allocated in a
    /// given [`Arena`]
    pub fn add_to_arena<'arena>(&self, arena: &'arena Arena<Json<'arena>>) -> &'arena Json<'arena> {
        match self {
            TestJson::True => arena.alloc(Json::True),
            TestJson::False => arena.alloc(Json::False),
            TestJson::Null => arena.alloc(Json::Null),
            TestJson::Str(s) => arena.alloc(Json::Str(s.clone())),
            TestJson::Array(children) => {
                let mut child_vec: Vec<&'arena Json<'arena>> = Vec::with_capacity(children.len());
                for c in children {
                    child_vec.push(c.add_to_arena(arena));
                }
                arena.alloc(Json::Array(child_vec))
            }
            TestJson::Object(fields) => {
                let mut children = Vec::with_capacity(fields.len());
                for (key, value) in fields.iter() {
                    // Add both child nodes
                    let s = arena.alloc(Json::Str(key.clone()));
                    let v = value.add_to_arena(arena);
                    // Combine the two nodes into a fields
                    children.push(arena.alloc(Json::Field([s, v])));
                }
                arena.alloc(Json::Object(children))
            }
        }
    }
}

impl PartialEq<&Json<'_>> for TestJson {
    fn eq(&self, other: &&Json) -> bool {
        match (self, other) {
            (TestJson::True, Json::True) => true,
            (TestJson::False, Json::False) => true,
            (TestJson::Null, Json::Null) => true,
            (TestJson::Array(test_children), Json::Array(children)) => test_children == children,
            (TestJson::Object(test_fields), Json::Object(fields)) => {
                test_fields.len() == fields.len()
                    && test_fields.iter().zip(fields.iter()).all(|((k1, v1), f)| {
                        if let Json::Field([Json::Str(k2), v2]) = f {
                            k1 == k2 && v1 == v2
                        } else {
                            false
                        }
                    })
            }
            (TestJson::Str(s1), Json::Str(s2)) => s1 == s2,
            _ => false,
        }
    }
}
