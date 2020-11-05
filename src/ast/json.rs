use super::size::Size;
use super::{Ast, DisplayToken, Reference};
use crate::node_map::NodeMap;

/// An enum to hold the different ways that a JSON AST can be formatted
#[derive(Eq, PartialEq, Copy, Clone)]
pub enum JSONFormat {
    /// The most compact representation, has minimal whitespace.
    /// E.g. `[{"foo": true, "bar": false}, true]`
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
}

impl<Ref: Reference> Default for JSON<Ref> {
    fn default() -> JSON<Ref> {
        JSON::Object(vec![])
    }
}

impl<Ref: Reference> Ast<Ref> for JSON<Ref> {
    type FormatStyle = JSONFormat;

    /* FORMATTING FUNCTIONS */

    fn display_tokens(&self, format_style: &Self::FormatStyle) -> Vec<DisplayToken<Ref>> {
        let is_pretty = format_style == &JSONFormat::Pretty;
        match self {
            JSON::True => vec![DisplayToken::Text("true".to_string())],
            JSON::False => vec![DisplayToken::Text("false".to_string())],
            JSON::Str(string) => vec![DisplayToken::Text(format!(r#""{}""#, string))],
            JSON::Field([key, value]) => vec![
                DisplayToken::Child(*key),
                DisplayToken::Text(": ".to_string()),
                DisplayToken::Child(*value),
            ],
            JSON::Array(children) => {
                // Special case: if this array is empty, render it as '[]'
                if children.is_empty() {
                    return vec![DisplayToken::Text("[]".to_string())];
                }

                let mut tokens = Vec::with_capacity(6 + 3 * children.len());
                // Push some initial tokens
                tokens.push(DisplayToken::Text("[".to_string()));
                if is_pretty {
                    tokens.push(DisplayToken::Newline);
                    tokens.push(DisplayToken::Indent);
                }
                // Push the children, delimited by commas
                let mut is_first_child = true;
                for c in children {
                    // Push the delimiting
                    if !is_first_child {
                        tokens.push(DisplayToken::Text(",".to_string()));
                        if is_pretty {
                            tokens.push(DisplayToken::Newline);
                        } else {
                            tokens.push(DisplayToken::Whitespace(1));
                        }
                    }
                    is_first_child = false;
                    // Push the single child
                    tokens.push(DisplayToken::Child(*c));
                }
                // Push the closing bracket
                if is_pretty {
                    tokens.push(DisplayToken::Newline);
                    tokens.push(DisplayToken::Dedent);
                }
                tokens.push(DisplayToken::Text("]".to_string()));
                // Return the token stream
                tokens
            }
            JSON::Object(fields) => {
                // Special case: if this object is empty, render it as '{}'
                if fields.is_empty() {
                    return vec![DisplayToken::Text("{}".to_string())];
                }

                let mut tokens = Vec::with_capacity(6 + 3 * fields.len());
                // Push some initial tokens
                tokens.push(DisplayToken::Text("{".to_string()));
                if is_pretty {
                    tokens.push(DisplayToken::Newline);
                    tokens.push(DisplayToken::Indent);
                }
                // Push the children, delimited by commas
                let mut is_first_child = true;
                for f in fields {
                    // Push the delimiting
                    if !is_first_child {
                        tokens.push(DisplayToken::Text(",".to_string()));
                        if is_pretty {
                            tokens.push(DisplayToken::Newline);
                        } else {
                            tokens.push(DisplayToken::Whitespace(1));
                        }
                    }
                    is_first_child = false;
                    // Push the single child
                    tokens.push(DisplayToken::Child(*f));
                }
                // Push the closing bracket
                if is_pretty {
                    tokens.push(DisplayToken::Newline);
                    tokens.push(DisplayToken::Dedent);
                }
                tokens.push(DisplayToken::Text("}".to_string()));
                // Return the token stream
                tokens
            }
        }
    }

    fn size(&self, node_map: &impl NodeMap<Ref, Self>, format_style: &Self::FormatStyle) -> Size {
        /// A cheeky macro that generates a recursive call to get the size of a child node
        macro_rules! get_size {
            ($ref: expr) => {
                node_map
                    .get_node($ref)
                    .unwrap()
                    .size(node_map, format_style)
            };
        };

        match format_style {
            JSONFormat::Pretty => {
                match self {
                    JSON::True => Size::new(0, 4),  // same as Size::from("true")
                    JSON::False => Size::new(0, 5), // same as Size::from("false")
                    JSON::Str(string) => {
                        Size::new(0, 1) + Size::from(string.as_str()) + Size::new(0, 1)
                    }
                    JSON::Field([key, value]) => {
                        get_size!(*key) + Size::new(0, 2) + get_size!(*value)
                    }
                    JSON::Object(fields) => {
                        // Special case: if the object is empty, then it will be rendered as "{}",
                        // which only takes up one line
                        if fields.is_empty() {
                            return Size::new(0, 2); // same as Size::from("{}")
                        }
                        /* For an object, we are only interested in how many lines are occupied -
                         * the last line will always just be "}" */
                        // We initialise this to 1 because the opening '{' occupies its own line.
                        let mut number_of_lines = 1;
                        for f in fields {
                            // The `+ 1` accounts for the extra newline char generated between
                            // every field.
                            number_of_lines += get_size!(*f).lines() + 1;
                        }
                        Size::new(number_of_lines, 1)
                    }
                    JSON::Array(children) => {
                        // Special case: if the array is empty, then it will be rendered as "[]",
                        // which only takes up one line
                        if children.is_empty() {
                            return Size::new(0, 2); // same as Size::from("[]");
                        }
                        /* For an array, we are only interested in how many lines are occupied -
                         * the last line will always just be "]" */
                        // We initialise this to 1 because the opening '[' occupies its own line.
                        let mut number_of_lines = 1;
                        for c in children {
                            // The `+ 1` accounts for the extra newline char generated between
                            // every child.
                            number_of_lines += get_size!(*c).lines() + 1;
                        }
                        Size::new(number_of_lines, 1)
                    }
                }
            }
            JSONFormat::Compact => {
                match self {
                    JSON::True => Size::new(0, 4),  // same as Size::from("true")
                    JSON::False => Size::new(0, 5), // same as Size::from("false")
                    JSON::Str(string) => {
                        Size::new(0, 1) + Size::from(string.as_str()) + Size::new(0, 1)
                    }
                    JSON::Field([key, value]) => {
                        get_size!(*key) + Size::new(0, 2) + get_size!(*value)
                    }
                    JSON::Object(fields) => {
                        // Size accumulator - starts with just the size of "{"
                        let mut size = Size::new(0, 1);
                        // Append all the children, and put ", " between all of them
                        let mut is_first_child = true;
                        for f in fields {
                            // If we're not on the first child, add a ", "
                            if !is_first_child {
                                size += Size::new(0, 2);
                            }
                            is_first_child = false;
                            size += get_size!(*f);
                        }
                        // Append one more char for "}" to the end, and return
                        size + Size::new(0, 1)
                    }
                    JSON::Array(children) => {
                        // Size accumulator - starts with just the size of "["
                        let mut size = Size::new(0, 1);
                        // Append all the children, and put ", " between all of them
                        let mut is_first_child = true;
                        for c in children {
                            // If we're not on the first child, add a ", "
                            if !is_first_child {
                                size += Size::new(0, 2);
                            }
                            is_first_child = false;
                            size += get_size!(*c);
                        }
                        // Append one more char for "]" to the end, and return
                        size + Size::new(0, 1)
                    }
                }
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

    fn children_mut(&mut self) -> &mut [Ref] {
        match self {
            JSON::True | JSON::False | JSON::Str(_) => &mut [],
            JSON::Array(children) => children,
            JSON::Object(fields) => fields,
            JSON::Field(key_value) => &mut key_value[..],
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
    use crate::ast_spec::size::Size;
    use crate::ast_spec::test_json::TestJSON;
    use crate::ast_spec::ASTSpec;
    use crate::node_map::vec::{Index, VecNodeMap};
    use crate::node_map::{NodeMap, NodeMapMut};

    /// Non-generic version of [`TestJSON::build_node_map`] that always returns a [`VecNodeMap`].
    fn build_vec_node_map(tree: &TestJSON) -> VecNodeMap<JSON<Index>> {
        tree.build_node_map::<Index, VecNodeMap<JSON<Index>>>()
    }

    #[test]
    fn to_text() {
        for (tree, expected_compact_string, expected_pretty_string, tree_string) in &[
            (TestJSON::True, "true", "true", "true"),
            (TestJSON::False, "false", "false", "false"),
            (TestJSON::Array(vec![]), "[]", "[]", "array"),
            (TestJSON::Object(vec![]), "{}", "{}", "object"),
            (
                TestJSON::Array(vec![TestJSON::True, TestJSON::False]),
                "[true, false]",
                "[
    true,
    false
]",
                "array
  true
  false",
            ),
            (
                TestJSON::Object(vec![
                    ("foo".to_string(), TestJSON::True),
                    ("bar".to_string(), TestJSON::False),
                ]),
                r#"{"foo": true, "bar": false}"#,
                r#"{
    "foo": true,
    "bar": false
}"#,
                r#"object
  field
    "foo"
    true
  field
    "bar"
    false"#,
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
                r#"array
  object
    field
      "foos"
      array
        false
        true
        false
    field
      "bar"
      false
  true"#,
            ),
        ] {
            println!("Testing {}", expected_compact_string);

            let node_map = build_vec_node_map(tree);
            // Test compact string
            let compact_string = node_map.to_text(&JSONFormat::Compact);
            assert_eq!(compact_string, *expected_compact_string);
            assert_eq!(
                node_map.root_node().size(&node_map, &JSONFormat::Compact),
                Size::from(*expected_compact_string)
            );
            // Test pretty string
            let pretty_string = node_map.to_text(&JSONFormat::Pretty);
            assert_eq!(pretty_string, *expected_pretty_string);
            assert_eq!(
                node_map.root_node().size(&node_map, &JSONFormat::Pretty),
                Size::from(*expected_pretty_string)
            );
            // Test debug tree view
            let mut s = String::new();
            let node_map = build_vec_node_map(tree);
            node_map.root_node().write_tree_view(&node_map, &mut s);
            assert_eq!(s, *tree_string);
        }
    }
}
