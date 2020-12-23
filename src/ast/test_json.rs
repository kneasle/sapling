//! Another representation of JSON trees (where nodes own their children), useful for unit tests
//! and debugging.

use super::json::JSON;
use crate::arena::Arena;

/// A copy of [`JSON`] where nodes own their children
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum TestJSON {
    /// The constant literal `true`.  Converts to [`JSON::True`]
    True,
    /// The constant literal `false`.  Converts to [`JSON::False`]
    False,
    /// The constant literal `null`.  Converts to [`JSON::Null`]
    Null,
    /// A JSON array `[child1, child2, ..., childN]`.  Converts to [`JSON::Array`]
    Array(Vec<TestJSON>),
    /// A JSON object `{key1: child1, key2: child2, ..., keyN: childN}`.  Converts to a
    /// [`JSON::Object`] with `N` [`JSON::Field`]s containing the key/value pairs.
    Object(Vec<(String, TestJSON)>),
    /// A JSON 'String' literal
    Str(String),
}

impl TestJSON {
    /// Convert this node into a standard [`JSON`] tree, where all the nodes are allocated in a
    /// given [`Arena`]
    pub fn add_to_arena<'arena>(&self, arena: &'arena Arena<JSON<'arena>>) -> &'arena JSON<'arena> {
        match self {
            TestJSON::True => arena.alloc(JSON::True),
            TestJSON::False => arena.alloc(JSON::False),
            TestJSON::Null => arena.alloc(JSON::Null),
            TestJSON::Str(s) => arena.alloc(JSON::Str(s.clone())),
            TestJSON::Array(children) => {
                let mut child_vec: Vec<&'arena JSON<'arena>> = Vec::with_capacity(children.len());
                for c in children {
                    child_vec.push(c.add_to_arena(arena));
                }
                arena.alloc(JSON::Array(child_vec))
            }
            TestJSON::Object(fields) => {
                let mut children = Vec::with_capacity(fields.len());
                for (key, value) in fields.iter() {
                    // Add both child nodes
                    let s = arena.alloc(JSON::Str(key.clone()));
                    let v = value.add_to_arena(arena);
                    // Combine the two nodes into a fields
                    children.push(arena.alloc(JSON::Field([s, v])));
                }
                arena.alloc(JSON::Object(children))
            }
        }
    }
}

impl PartialEq<&JSON<'_>> for TestJSON {
    fn eq(&self, other: &&JSON) -> bool {
        match (self, other) {
            (TestJSON::True, JSON::True) => true,
            (TestJSON::False, JSON::False) => true,
            (TestJSON::Null, JSON::Null) => true,
            (TestJSON::Array(test_children), JSON::Array(children)) => test_children == children,
            (TestJSON::Object(test_fields), JSON::Object(fields)) => {
                test_fields.len() == fields.len()
                    && test_fields.iter().zip(fields.iter()).all(|((k1, v1), f)| {
                        if let JSON::Field([JSON::Str(k2), v2]) = f {
                            k1 == k2 && v1 == v2
                        } else {
                            false
                        }
                    })
            }
            (TestJSON::Str(s1), JSON::Str(s2)) => s1 == s2,
            _ => false,
        }
    }
}
