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

const CHAR_TRUE: char = 't';
const CHAR_FALSE: char = 'f';
const CHAR_ARRAY: char = 'a';
const CHAR_OBJECT: char = 'o';
const CHAR_FIELD: char = 'i';
const CHAR_STRING: char = 's';

/// The sapling representation of the AST for a subset of JSON (where all values are either 'true'
/// or 'false', and keys only contain ASCII).
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum JSON<Ref: Reference> {
    /// The JSON value for 'true'.  Corresponds to the string `true`.
    True,
    /// The JSON value 'false'.  Corresponds to the string `false`.
    False,
    /// A JSON array of multiple values.
    /// Corresponds to a string `[<v1>, <v2>, ...]` where `v1`, `v2`, ... are JSON values.
    Array(Vec<Ref>),
    /// A JSON object, represented as a map of [`String`]s to more JSON values.
    /// Corresponds to a string `{"<key1>": <v1>, "<key2>": <v2>, ...}` where `<key1>`, `<key2>`,
    /// ... are the keys, and `<v1>`, `<v2>`, ... are the corresponding JSON values.  The `Ref`s
    /// contained inside this must be [`Field`](JSON::Field)s.
    Object(Vec<Ref>),
    /// A JSON object field.  The first `Ref` must be a [`Str`](JSON::Str), and the second is any
    /// JSON object
    Field([Ref; 2]),
    /// A JSON string
    Str(String),
}

impl<Ref: Reference> JSON<Ref> {
    /// Return an iterator over all the possible chars that could represent JSON nodes
    fn all_object_chars() -> Box<dyn Iterator<Item = char>> {
        Box::new(
            [CHAR_TRUE, CHAR_FALSE, CHAR_ARRAY, CHAR_OBJECT, CHAR_STRING]
                .iter()
                .copied(),
        )
    }

    fn write_text_compact(&self, node_map: &impl NodeMap<Ref, Self>, string: &mut String) {
        macro_rules! draw_recursive {
            ($ref_expr: expr) => {{
                let r = $ref_expr;
                match node_map.get_node(r) {
                    Some(node) => {
                        node.write_text_compact(node_map, string);
                    }
                    None => {
                        string.push_str(&format!("<INVALID REF {:?}>", r));
                    }
                }
            }};
        };
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
                    draw_recursive!(child);
                }
                // Finish the array with a ']'
                string.push(']');
            }
            JSON::Object(fields) => {
                // All objects start with a '{'
                string.push('{');
                // Append all the children, separated by commas
                let mut is_first_child = true;
                for field in fields.iter().copied() {
                    // Add the comma if this isn't the first element
                    if !is_first_child {
                        string.push_str(", ");
                    }
                    is_first_child = false;
                    // Push the field's name then a colon then the child value
                    draw_recursive!(field);
                }
                // Finish the array with a '}'
                string.push('}');
            }
            JSON::Field([key, value]) => {
                /* We want to generate the string `<key>: <value>` */
                // Push the 'key' node
                draw_recursive!(*key);
                // Push the ': '
                string.push_str(": ");
                // Push the 'value' node
                draw_recursive!(*value);
            }
            JSON::Str(content) => {
                // Just push `"<content>"` - we won't worry about proper escaping yet
                string.push('"');
                string.push_str(&content);
                string.push('"');
            }
        }
    }

    fn write_text_pretty(
        &self,
        node_map: &impl NodeMap<Ref, Self>,
        string: &mut String,
        indentation_buffer: &mut String,
    ) {
        macro_rules! draw_recursive {
            ($ref_expr: expr) => {{
                let r = $ref_expr;
                match node_map.get_node(r) {
                    Some(node) => {
                        node.write_text_pretty(node_map, string, indentation_buffer);
                    }
                    None => {
                        string.push_str(&format!("<INVALID REF {:?}>", r));
                    }
                }
            }};
        };
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
                        draw_recursive!(child);
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
                    for field in fields.iter().copied() {
                        // Add the comma if this isn't the first element
                        if !is_first_child {
                            string.push_str(",\n");
                        }
                        is_first_child = false;
                        // Indent and then render the child
                        string.push_str(indentation_buffer);
                        draw_recursive!(field);
                    }
                    // Return to the current indentation level
                    for _ in 0..4 {
                        indentation_buffer.pop();
                    }
                    // Put the finishing ']' on its own line
                    string.push('\n');
                    string.push_str(indentation_buffer);
                }
                // Push a closing '}'
                string.push('}');
            }
            JSON::Field([key, value]) => {
                /* We want to generate the string `<key>: <value>` */

                // Push the 'key' node
                draw_recursive!(*key);
                // Push the ': '
                string.push_str(": ");
                // Push the 'value' node
                draw_recursive!(*value);
            }
            JSON::Str(content) => {
                // Just push `"<content>"` - we won't worry about proper escaping yet
                string.push('"');
                string.push_str(&content);
                string.push('"');
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

    fn children(&self) -> &[Ref] {
        match self {
            JSON::True | JSON::False | JSON::Str(_) => &[],
            JSON::Array(children) => &children,
            JSON::Object(fields) => &fields,
            JSON::Field(key_value) => &key_value[..],
        }
    }

    fn display_name(&self) -> String {
        match self {
            JSON::True => "true".to_string(),
            JSON::False => "false".to_string(),
            JSON::Array(_) => "array".to_string(),
            JSON::Object(_) => "object".to_string(),
            JSON::Field(_) => "field".to_string(),
            JSON::Str(content) => format!(r#""{}""#, content),
        }
    }

    /* AST EDITING FUNCTIONS */

    fn replace_chars(&self) -> Box<dyn Iterator<Item = char>> {
        Self::all_object_chars()
    }

    fn from_char(&self, c: char) -> Option<Self> {
        match c {
            CHAR_TRUE => Some(JSON::True),
            CHAR_FALSE => Some(JSON::False),
            CHAR_ARRAY => Some(JSON::Array(vec![])),
            CHAR_OBJECT => Some(JSON::Object(vec![])),
            CHAR_STRING => Some(JSON::Str("".to_string())),
            _ => None,
        }
    }

    fn insert_chars(&self) -> Box<dyn Iterator<Item = char>> {
        match self {
            JSON::True | JSON::False | JSON::Field(_) | JSON::Str(_) => {
                Box::new(std::iter::empty())
            }
            JSON::Object(_) => Box::new(std::iter::once(CHAR_FIELD)),
            JSON::Array(_) => Self::all_object_chars(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{JSONFormat, JSON};
    use crate::ast_spec::test_json::TestJSON;
    use crate::ast_spec::NodeMap;
    use crate::vec_node_map::{Index, VecNodeMap};

    /// None-generic version of [`TestJSON::build_node_map`] that always returns a [`VecNodeMap`].
    fn build_vec_node_map(tree: &TestJSON) -> VecNodeMap<JSON<Index>> {
        tree.build_node_map::<Index, VecNodeMap<JSON<Index>>>()
    }

    #[test]
    fn to_text_compact() {
        for (tree, expected_string) in &[
            (TestJSON::True, "true"),
            (TestJSON::False, "false"),
            (TestJSON::Array(vec![]), "[]"),
            (TestJSON::Object(vec![]), "{}"),
            (
                TestJSON::Array(vec![TestJSON::True, TestJSON::False]),
                "[true, false]",
            ),
            (
                TestJSON::Object(vec![
                    ("foo".to_string(), TestJSON::True),
                    ("bar".to_string(), TestJSON::False),
                ]),
                r#"{"foo": true, "bar": false}"#,
            ),
            (
                TestJSON::Array(vec![
                    TestJSON::Object(vec![
                        (
                            "foos".to_string(),
                            TestJSON::Array(vec![TestJSON::False, TestJSON::True, TestJSON::False]),
                        ),
                        ("bar".to_string(), TestJSON::False),
                    ]),
                    TestJSON::True,
                ]),
                r#"[{"foos": [false, true, false], "bar": false}, true]"#,
            ),
        ] {
            assert_eq!(
                build_vec_node_map(tree).to_text(&JSONFormat::Compact),
                *expected_string
            );
        }
    }

    #[test]
    fn to_text_pretty() {
        for (tree, expected_string) in &[
            (TestJSON::True, "true"),
            (TestJSON::False, "false"),
            (TestJSON::Array(vec![]), "[]"),
            (TestJSON::Object(vec![]), "{}"),
            (
                TestJSON::Array(vec![TestJSON::True, TestJSON::False]),
                "[
    true,
    false
]",
            ),
            (
                TestJSON::Object(vec![
                    ("foo".to_string(), TestJSON::True),
                    ("bar".to_string(), TestJSON::False),
                ]),
                r#"{
    "foo": true,
    "bar": false
}"#,
            ),
            (
                TestJSON::Array(vec![
                    TestJSON::Object(vec![
                        (
                            "foos".to_string(),
                            TestJSON::Array(vec![TestJSON::False, TestJSON::True, TestJSON::False]),
                        ),
                        ("bar".to_string(), TestJSON::False),
                    ]),
                    TestJSON::True,
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
            assert_eq!(
                build_vec_node_map(tree).to_text(&JSONFormat::Pretty),
                *expected_string
            );
        }
    }

    /*
        // This function actually tests `write_tree_view` from 'ast/mod.rs', but since that is a trait
        // method, it can only be tested on a concrete implementation of AST
        #[test]
        fn tree_view() {
            for (tree, expected_string) in &[
                (TestJSON::True, "true"),
                (TestJSON::False, "false"),
                (TestJSON::Object(vec![]), "object"),
                (TestJSON::Array(vec![]), "array"),
                (
                    TestJSON::Array(vec![TestJSON::True, TestJSON::False]),
                    "array
    ├── true
    └── false",
                ),
            ] {
                let node_map = tree.build_node_map();
                assert_eq!(node_map.root_node().tree_view(&node_map), *expected_string);
            }
        }
    */
}
