use super::{ASTSpec, NodeMap, Reference};

/// An enum to hold the different ways that a JSON AST can be formatted
#[derive(Eq, PartialEq, Copy, Clone)]
pub enum JSONFormat {
    /// The most compact representation, has minimal whitespace.
    /// E.g. `[{"foo": true, "bar: false}, true]`
    Compact,
    /// A prettified representation, with pretty indenting and every element on a newline.
    Pretty,
}

/// The sapling representation of the AST for a subset of JSON (where all values are either 'true'
/// or 'false', and keys only contain ASCII).
#[derive(Eq, PartialEq, Clone)]
pub enum JSON<Ref: Reference> {
    /// The JSON value for 'true'.  Corresponds to the string `true`.
    True,
    /// The JSON value 'false'.  Corresponds to the string `false`.
    False,
    /// A JSON array of multiple values.
    /// Corresponds to a string `[<v1>, <v2>, ...]` where `v1`, `v2`, ... are JSON values.
    Array(Vec<Ref>),
    /// A JSON object, represented as a map of [String]s to more JSON values.
    /// Corresponds to a string `{"<key1>": <v1>, "<key2>": <v2>, ...}` where `<key1>`, `<key2>`,
    /// ... are the keys, and `<v1>`, `<v2>`, ... are the corresponding JSON values.
    Object(Vec<(String, Ref)>),
}

const CHAR_TRUE: char = 't';
const CHAR_FALSE: char = 'f';
const CHAR_ARRAY: char = 'a';
const CHAR_OBJECT: char = 'o';

impl<Ref: Reference> JSON<Ref> {
    fn write_text_compact(&self, node_map: &impl NodeMap<Ref, Self>, string: &mut String) {
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
                // Append all the children, separated by commas
                let mut is_first_child = true;
                for child in children.iter().copied() {
                    // Add the comma if this isn't the first element
                    if !is_first_child {
                        string.push_str(", ");
                    }
                    is_first_child = false;
                    // Push the field's name then a colon then the child value
                    match node_map.get_node(child) {
                        Some(node) => {
                            node.write_text_compact(node_map, string);
                        }
                        None => {
                            string.push_str(&format!("<INVALID REF {:?}>", child));
                        }
                    }
                }
                // Finish the array with a ']'
                string.push(']');
            }
            JSON::Object(fields) => {
                // All objects start with a '{'
                string.push('{');
                // Append all the children, separated by commas
                let mut is_first_child = true;
                for (name, child) in fields.iter() {
                    // Add the comma if this isn't the first element
                    if !is_first_child {
                        string.push_str(", ");
                    }
                    is_first_child = false;
                    // Push the field's name then a colon then the child value
                    string.push('"');
                    string.push_str(name);
                    string.push_str("\": ");
                    match node_map.get_node(*child) {
                        Some(node) => {
                            node.write_text_compact(node_map, string);
                        }
                        None => {
                            string.push_str(&format!("<INVALID REF {:?}>", child));
                        }
                    }
                }
                // Finish the array with a '}'
                string.push('}');
            }
        }
    }

    fn write_text_pretty(
        &self,
        node_map: &impl NodeMap<Ref, Self>,
        string: &mut String,
        indentation_buffer: &mut String,
    ) {
        // Insert the text for this JSON tree
        match self {
            JSON::True => {
                string.push_str("true");
            }
            JSON::False => {
                string.push_str("false");
            }
            JSON::Array(children) => {
                // Push the '[' on its own line
                string.push('[');
                if !children.is_empty() {
                    string.push('\n');
                    // Indent by one extra level
                    indentation_buffer.push_str("    ");
                    // Append all the children, separated by commas
                    let mut is_first_child = true;
                    for child in children.iter().copied() {
                        // Add the comma if this isn't the first element
                        if !is_first_child {
                            string.push_str(",\n");
                        }
                        is_first_child = false;
                        // Indent and then render the child
                        string.push_str(indentation_buffer);
                        match node_map.get_node(child) {
                            Some(node) => {
                                node.write_text_pretty(node_map, string, indentation_buffer);
                            }
                            None => {
                                string.push_str(&format!("<INVALID REF {:?}>", child));
                            }
                        }
                    }
                    // Return to the current indentation level
                    for _ in 0..4 {
                        indentation_buffer.pop();
                    }
                    // Put the finishing ']' on its own line
                    string.push('\n');
                    string.push_str(indentation_buffer);
                }
                string.push(']');
            }
            JSON::Object(fields) => {
                // Push the '{' on its own line
                string.push('{');
                if !fields.is_empty() {
                    string.push('\n');
                    // Indent by one extra level
                    indentation_buffer.push_str("    ");
                    // Append all the children, separated by commas
                    let mut is_first_child = true;
                    for (name, child) in fields.iter() {
                        // Add the comma if this isn't the first element
                        if !is_first_child {
                            string.push_str(",\n");
                        }
                        is_first_child = false;
                        // Indent the right number of times
                        string.push_str(indentation_buffer);
                        // Push the field's name then a colon then the child value
                        string.push('"');
                        string.push_str(name);
                        string.push_str("\": ");
                        match node_map.get_node(*child) {
                            Some(node) => {
                                node.write_text_pretty(node_map, string, indentation_buffer);
                            }
                            None => {
                                string.push_str(&format!("<INVALID REF {:?}>", child));
                            }
                        }
                    }
                    // Return to the current indentation level
                    for _ in 0..4 {
                        indentation_buffer.pop();
                    }
                    // Put the finishing '}' on its own line
                    string.push('\n');
                    string.push_str(indentation_buffer);
                }
                string.push('}');
            }
        }
    }
}

impl<Ref: Reference> Default for JSON<Ref> {
    fn default() -> JSON<Ref> {
        JSON::Object(vec![])
    }
}

impl<Ref: Reference> ASTSpec<Ref> for JSON<Ref> {
    type FormatStyle = JSONFormat;

    /* FORMATTING FUNCTIONS */

    fn write_text(
        &self,
        node_map: &impl NodeMap<Ref, Self>,
        string: &mut String,
        format_style: &JSONFormat,
    ) {
        match format_style {
            JSONFormat::Compact => {
                self.write_text_compact(node_map, string);
            }
            JSONFormat::Pretty => {
                let mut indentation_buffer = String::new();
                self.write_text_pretty(node_map, string, &mut indentation_buffer);
            }
        }
    }

    /* DEBUG VIEW FUNCTIONS */

    fn get_children<'a>(&'a self) -> Box<dyn Iterator<Item = Ref> + 'a> {
        match self {
            JSON::True | JSON::False => Box::new(std::iter::empty()),
            JSON::Array(children) => Box::new(children.iter().copied()),
            JSON::Object(fields) => Box::new(fields.iter().map(|x| x.1)),
        }
    }

    fn get_display_name(&self) -> String {
        match self {
            JSON::True => "true".to_string(),
            JSON::False => "false".to_string(),
            JSON::Array(_) => "array".to_string(),
            JSON::Object(_) => "object".to_string(),
        }
    }

    /* AST EDITING FUNCTIONS */

    fn get_replace_chars(&self) -> Box<dyn Iterator<Item = char>> {
        Box::new(
            [CHAR_TRUE, CHAR_FALSE, CHAR_ARRAY, CHAR_OBJECT]
                .iter()
                .copied(),
        )
    }

    fn from_replace_char(&self, c: char) -> Option<Self> {
        match c {
            CHAR_TRUE => Some(JSON::True),
            CHAR_FALSE => Some(JSON::False),
            CHAR_ARRAY => Some(JSON::Array(vec![])),
            CHAR_OBJECT => Some(JSON::Object(vec![])),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{JSONFormat, AST, JSON};

    #[test]
    fn to_text_compact() {
        for (tree, expected_string) in &[
            (JSON::True, "true"),
            (JSON::False, "false"),
            (JSON::Array(vec![]), "[]"),
            (JSON::Object(vec![]), "{}"),
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
            assert_eq!(tree.to_text(&JSONFormat::Compact), *expected_string);
        }
    }

    #[test]
    fn to_text_pretty() {
        for (tree, expected_string) in &[
            (JSON::True, "true"),
            (JSON::False, "false"),
            (JSON::Array(vec![]), "[]"),
            (JSON::Object(vec![]), "{}"),
            (
                JSON::Array(vec![JSON::True, JSON::False]),
                "[
    true,
    false
]",
            ),
            (
                JSON::Object(vec![
                    ("foo".to_string(), JSON::True),
                    ("bar".to_string(), JSON::False),
                ]),
                r#"{
    "foo": true,
    "bar": false
}"#,
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
                r#"[
    {
        "foos": [
            false,
            true,
            false
        ],
        "bar": false
    },
    true
]"#,
            ),
        ] {
            assert_eq!(tree.to_text(&JSONFormat::Pretty), *expected_string);
        }
    }

    // This function actually tests `write_tree_view` from 'ast/mod.rs', but since that is a trait
    // method, it can only be tested on a concrete implementation of AST
    #[test]
    fn tree_view() {
        for (tree, expected_string) in &[
            (JSON::True, "true"),
            (JSON::False, "false"),
            (JSON::Object(vec![]), "object"),
            (JSON::Array(vec![]), "array"),
            (
                JSON::Array(vec![JSON::True, JSON::False]),
                "array
├── true
└── false",
            ),
        ] {
            assert_eq!(tree.tree_view(), *expected_string);
        }
    }
}
