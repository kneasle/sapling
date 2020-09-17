use crate::ast::AST;

/// The sapling representation of the AST for a subset of JSON (where all values are either 'true'
/// or 'false', and keys only contain ASCII).
#[derive(Eq, PartialEq, Clone)]
pub enum JSON {
    /// The JSON value for 'true'.  Corresponds to the string `true`.
    True,
    /// The JSON value 'false'.  Corresponds to the string `false`.
    False,
    /// A JSON array of multiple values.
    /// Corresponds to a string `[<v1>, <v2>, ...]` where `v1`, `v2`, ... are JSON values.
    Array(Vec<JSON>),
    /// A JSON object, represented as a map of [String]s to more JSON values.
    /// Corresponds to a string `{"<key1>": <v1>, "<key2>": <v2>, ...}` where `<key1>`, `<key2>`,
    /// ... are the keys, and `<v1>`, `<v2>`, ... are the corresponding JSON values.
    Object(Vec<(String, JSON)>),
}

impl Default for JSON {
    fn default() -> JSON {
        JSON::Object(vec![])
    }
}

impl AST for JSON {
    fn write_text(&self, string: &mut String) {
        match self {
            JSON::True => {
                string.push_str("true");
            }
            JSON::False => {
                string.push_str("false");
            }
            JSON::Array(children) => {
                // All arrays start with a '['
                string.push('[');
                // Push the first child (if it exists) without starting with a ','.
                // We don't need to fuse 'child_iter' because std::slice::Iter is already a
                // FusedIterator
                let mut child_iter = children.iter();
                if let Some(first_child) = child_iter.next() {
                    first_child.write_text(string);
                }
                // Iterate over all the other children, and push a ', ' in front of them to make
                // sure that the values are comma-delimited
                for child in child_iter {
                    string.push_str(", ");
                    child.write_text(string);
                }
                // Finish the array with a ']'
                string.push(']');
            }
            JSON::Object(fields) => {
                // All objects start with a '{'
                string.push('{');
                // Push the first child (if it exists) without starting with a ','.
                // We don't need to fuse 'child_iter' because std::slice::Iter is already a
                // FusedIterator
                let mut child_iter = fields.iter();
                if let Some((name, child)) = child_iter.next() {
                    // Push the field's name, and a colon
                    string.push('"');
                    string.push_str(name);
                    string.push_str("\": ");
                    child.write_text(string);
                }
                // Iterate over all the other fields, and push a ', ' in front of them to make
                // sure that the values are comma-delimited
                for (name, child) in child_iter {
                    string.push_str(", ");
                    string.push('"');
                    string.push_str(name);
                    string.push_str("\": ");
                    child.write_text(string);
                }
                // Finish the array with a '}'
                string.push('}');
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AST, JSON};

    #[test]
    fn text_conversion() {
        for (tree, expected_string) in &[
            (JSON::True, "true"),
            (JSON::False, "false"),
            (JSON::Array(vec![JSON::True, JSON::False]), "[true, false]"),
            (
                JSON::Object(vec![
                    ("foo".to_string(), JSON::True),
                    ("bar".to_string(), JSON::False),
                ]),
                r#"{"foo": true, "bar": false}"#,
            ),
            (
                JSON::Array(vec![
                    JSON::Object(vec![
                        (
                            "foos".to_string(),
                            JSON::Array(vec![JSON::False, JSON::True, JSON::False]),
                        ),
                        ("bar".to_string(), JSON::False),
                    ]),
                    JSON::True,
                ]),
                r#"[{"foos": [false, true, false], "bar": false}, true]"#,
            ),
        ] {
            assert_eq!(tree.to_text(), *expected_string);
        }
    }
}
