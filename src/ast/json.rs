use super::display_token::{DisplayToken, RecTok};
use super::size::Size;
use super::Ast;
use crate::arena::Arena;

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
const CHAR_NULL: char = 'n';
const CHAR_ARRAY: char = 'a';
const CHAR_OBJECT: char = 'o';
const CHAR_STRING: char = 's';

/// Error produced when inserting a child into a JSON node fails
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum InsertError {
    /// A child was attempted to be inserted into a node that can only have a fixed number of
    /// children.  The second argument is the number of children that this node has to have.  This
    /// is used by nodes such as `field`, which is required to have 2 children.
    FixedChildCount(String, usize),
}

impl std::fmt::Display for InsertError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InsertError::FixedChildCount(node, num_children) => {
                if *num_children == 0 {
                    write!(f, "Node {} cannot contain other nodes.", node)
                } else if *num_children == 1 {
                    write!(f, "Node {} can only have 1 child.", node)
                } else {
                    write!(f, "Node {} can only have {} children.", node, num_children)
                }
            }
        }
    }
}

impl std::error::Error for InsertError {}

/// Error produced when trying to delete a child from this node
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum DeleteError {
    /// We attempted to delete a child who's index was outside the bounds of the child array
    IndexOutOfRange(usize, usize),
    /// We attempted to delete a child from a node which must contain a fixed number of children
    FixedChildCount(String, usize),
}

impl std::fmt::Display for DeleteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeleteError::IndexOutOfRange(num_children, index) => write!(
                f,
                "Tried to remove child #{} from a node that only has {} children.",
                index, num_children
            ),
            DeleteError::FixedChildCount(node, num_children) => {
                if *num_children == 0 {
                    write!(
                        f,
                        "Node {} cannot have children, but we tried to delete a child.",
                        node
                    )
                } else if *num_children == 1 {
                    write!(
                        f,
                        "Cannot delete from a node {} that can only have 1 child.",
                        node
                    )
                } else {
                    write!(
                        f,
                        "Cannot delete from a node {} that can only have {} children.",
                        node, num_children
                    )
                }
            }
        }
    }
}

impl std::error::Error for DeleteError {}

/// The sapling representation of the AST for a subset of JSON (where all values are either 'true'
/// or 'false', and keys only contain ASCII).
#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub enum JSON<'arena> {
    /// The JSON value for 'true'.  Corresponds to the string `true`.
    True,
    /// The JSON value 'false'.  Corresponds to the string `false`.
    False,
    /// The JSON value 'null'.  Corresponds to the string `null`.
    Null,
    /// A JSON array of multiple values.
    /// Corresponds to a string `[<v1>, <v2>, ...]` where `v1`, `v2`, ... are JSON values.
    Array(Vec<&'arena JSON<'arena>>),
    /// A JSON object, represented as a map of [`String`]s to more JSON values.
    /// Corresponds to a string `{"<key1>": <v1>, "<key2>": <v2>, ...}` where `<key1>`, `<key2>`,
    /// ... are the keys, and `<v1>`, `<v2>`, ... are the corresponding JSON values.  The `Ref`s
    /// contained inside this must be [`Field`](JSON::Field)s.
    Object(Vec<&'arena JSON<'arena>>),
    /// A JSON object field.  The first `Ref` must be a [`Str`](JSON::Str), and the second is any
    /// JSON object
    Field([&'arena JSON<'arena>; 2]),
    /// A JSON string
    Str(String),
}

impl JSON<'_> {
    /// Return an iterator over all the possible chars that could represent JSON nodes
    fn all_object_chars() -> Box<dyn Iterator<Item = char>> {
        Box::new(
            [
                CHAR_TRUE,
                CHAR_FALSE,
                CHAR_NULL,
                CHAR_ARRAY,
                CHAR_OBJECT,
                CHAR_STRING,
            ]
            .iter()
            .copied(),
        )
    }
}

impl Default for JSON<'_> {
    fn default() -> JSON<'static> {
        JSON::Object(vec![])
    }
}

impl<'arena> Ast<'arena> for JSON<'arena> {
    type FormatStyle = JSONFormat;
    type InsertError = InsertError;
    type DeleteError = DeleteError;

    /* FORMATTING FUNCTIONS */

    fn display_tokens_rec(
        &'arena self,
        format_style: &Self::FormatStyle,
    ) -> Vec<RecTok<'arena, Self>> {
        let is_pretty = format_style == &JSONFormat::Pretty;
        match self {
            JSON::True => vec![RecTok::Tok(DisplayToken::Text("true".to_string()))],
            JSON::False => vec![RecTok::Tok(DisplayToken::Text("false".to_string()))],
            JSON::Null => vec![RecTok::Tok(DisplayToken::Text("null".to_string()))],
            JSON::Str(string) => vec![RecTok::Tok(DisplayToken::Text(format!(r#""{}""#, string)))],
            JSON::Field([key, value]) => vec![
                RecTok::Child(key),
                RecTok::Tok(DisplayToken::Text(": ".to_string())),
                RecTok::Child(value),
            ],
            JSON::Array(children) => {
                // Special case: if this array is empty, render it as '[]'
                if children.is_empty() {
                    return vec![RecTok::Tok(DisplayToken::Text("[]".to_string()))];
                }

                let mut tokens: Vec<RecTok<'_, Self>> = Vec::with_capacity(6 + 3 * children.len());
                // Push some initial tokens
                tokens.push(RecTok::Tok(DisplayToken::Text("[".to_string())));
                if is_pretty {
                    tokens.push(RecTok::Tok(DisplayToken::Indent));
                    tokens.push(RecTok::Tok(DisplayToken::Newline));
                }
                // Push the children, delimited by commas
                let mut is_first_child = true;
                for c in children {
                    // Push the delimiting
                    if !is_first_child {
                        tokens.push(RecTok::Tok(DisplayToken::Text(",".to_string())));
                        if is_pretty {
                            tokens.push(RecTok::Tok(DisplayToken::Newline));
                        } else {
                            tokens.push(RecTok::Tok(DisplayToken::Whitespace(1)));
                        }
                    }
                    is_first_child = false;
                    // Push the single child
                    tokens.push(RecTok::Child(c));
                }
                // Push the closing bracket
                if is_pretty {
                    tokens.push(RecTok::Tok(DisplayToken::Dedent));
                    tokens.push(RecTok::Tok(DisplayToken::Newline));
                }
                tokens.push(RecTok::Tok(DisplayToken::Text("]".to_string())));
                // Return the token stream
                tokens
            }
            JSON::Object(fields) => {
                // Special case: if this object is empty, render it as '{}'
                if fields.is_empty() {
                    return vec![RecTok::Tok(DisplayToken::Text("{}".to_string()))];
                }

                let mut tokens: Vec<RecTok<'_, Self>> = Vec::with_capacity(6 + 3 * fields.len());
                // Push some initial tokens
                tokens.push(RecTok::Tok(DisplayToken::Text("{".to_string())));
                if is_pretty {
                    tokens.push(RecTok::Tok(DisplayToken::Indent));
                    tokens.push(RecTok::Tok(DisplayToken::Newline));
                }
                // Push the children, delimited by commas
                let mut is_first_child = true;
                for f in fields {
                    // Push the delimiting
                    if !is_first_child {
                        tokens.push(RecTok::Tok(DisplayToken::Text(",".to_string())));
                        if is_pretty {
                            tokens.push(RecTok::Tok(DisplayToken::Newline));
                        } else {
                            tokens.push(RecTok::Tok(DisplayToken::Whitespace(1)));
                        }
                    }
                    is_first_child = false;
                    // Push the single child
                    tokens.push(RecTok::Child(f));
                }
                // Push the closing bracket
                if is_pretty {
                    tokens.push(RecTok::Tok(DisplayToken::Dedent));
                    tokens.push(RecTok::Tok(DisplayToken::Newline));
                }
                tokens.push(RecTok::Tok(DisplayToken::Text("}".to_string())));
                // Return the token stream
                tokens
            }
        }
    }

    fn size(&self, format_style: &Self::FormatStyle) -> Size {
        match format_style {
            JSONFormat::Pretty => {
                match self {
                    JSON::True => Size::new(0, 4),  // same as Size::from("true")
                    JSON::False => Size::new(0, 5), // same as Size::from("false")
                    JSON::Null => Size::new(0, 4),  // same as Size::from("null")
                    JSON::Str(string) => {
                        Size::new(0, 1) + Size::from(string.as_str()) + Size::new(0, 1)
                    }
                    JSON::Field([key, value]) => {
                        key.size(format_style) + Size::new(0, 2) + value.size(format_style)
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
                            number_of_lines += f.size(format_style).lines() + 1;
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
                            number_of_lines += c.size(format_style).lines() + 1;
                        }
                        Size::new(number_of_lines, 1)
                    }
                }
            }
            JSONFormat::Compact => {
                match self {
                    JSON::True => Size::new(0, 4),  // same as Size::from("true")
                    JSON::False => Size::new(0, 5), // same as Size::from("false")
                    JSON::Null => Size::new(0, 4),  // same as Size::from("null")
                    JSON::Str(string) => {
                        Size::new(0, 1) + Size::from(string.as_str()) + Size::new(0, 1)
                    }
                    JSON::Field([key, value]) => {
                        key.size(format_style) + Size::new(0, 2) + value.size(format_style)
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
                            size += f.size(format_style);
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
                            size += c.size(format_style);
                        }
                        // Append one more char for "]" to the end, and return
                        size + Size::new(0, 1)
                    }
                }
            }
        }
    }

    /* DEBUG VIEW FUNCTIONS */

    fn children<'s>(&'s self) -> &'s [&'arena JSON<'arena>] {
        match self {
            JSON::True | JSON::False | JSON::Null | JSON::Str(_) => &[],
            JSON::Array(children) => &children,
            JSON::Object(fields) => &fields,
            JSON::Field(key_value) => &key_value[..],
        }
    }

    fn children_mut<'s>(&'s mut self) -> &'s mut [&'arena JSON<'arena>] {
        match self {
            JSON::True | JSON::False | JSON::Null | JSON::Str(_) => &mut [],
            JSON::Array(children) => children,
            JSON::Object(fields) => fields,
            JSON::Field(key_value) => &mut key_value[..],
        }
    }

    fn insert_child(
        &mut self,
        new_node: &'arena Self,
        arena: &'arena Arena<Self>,
        index: usize,
    ) -> Result<(), Self::InsertError> {
        match self {
            JSON::True | JSON::False | JSON::Null | JSON::Str(_) => {
                Err(InsertError::FixedChildCount(self.display_name(), 0))
            }
            JSON::Field(_) => Err(InsertError::FixedChildCount(self.display_name(), 2)),
            JSON::Object(fields) => {
                /* Inserting into an object is a special case, since we need to allocate more
                 * objects in order to preserve the validity of the tree. */
                // Allocate an empty string to act as the key
                let key = arena.alloc(JSON::Str("".to_string()));
                // Allocate a field as the parent of the key and new_node
                let field = arena.alloc(JSON::Field([key, new_node]));
                // Add the new field as a child of `self`
                fields.insert(index, field);
                Ok(())
            }
            JSON::Array(children) => {
                children.insert(index, new_node);
                Ok(())
            }
        }
    }

    fn delete_child(&mut self, index: usize) -> Result<(), Self::DeleteError> {
        match self {
            JSON::True | JSON::False | JSON::Null | JSON::Str(_) => {
                Err(DeleteError::FixedChildCount(self.display_name(), 0))
            }
            JSON::Field(_) => Err(DeleteError::FixedChildCount(self.display_name(), 2)),
            JSON::Object(fields) => {
                if index < fields.len() {
                    fields.remove(index);
                    Ok(())
                } else {
                    Err(DeleteError::IndexOutOfRange(fields.len(), index))
                }
            }
            JSON::Array(children) => {
                if index < children.len() {
                    children.remove(index);
                    Ok(())
                } else {
                    Err(DeleteError::IndexOutOfRange(children.len(), index))
                }
            }
        }
    }

    fn display_name(&self) -> String {
        match self {
            JSON::True => "true".to_string(),
            JSON::False => "false".to_string(),
            JSON::Null => "null".to_string(),
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
            CHAR_NULL => Some(JSON::Null),
            CHAR_ARRAY => Some(JSON::Array(vec![])),
            CHAR_OBJECT => Some(JSON::Object(vec![])),
            CHAR_STRING => Some(JSON::Str("".to_string())),
            _ => None,
        }
    }

    fn insert_chars(&self) -> Box<dyn Iterator<Item = char>> {
        match self {
            JSON::True | JSON::False | JSON::Null | JSON::Field(_) | JSON::Str(_) => {
                Box::new(std::iter::empty())
            }
            JSON::Object(_) | JSON::Array(_) => Self::all_object_chars(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::size::Size;
    use super::super::test_json::TestJSON;
    use super::JSONFormat;
    use crate::arena::Arena;
    use crate::ast::Ast;

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

            let arena = Arena::new();
            let root = tree.add_to_arena(&arena);
            // Test compact string
            let compact_string = root.to_text(&JSONFormat::Compact);
            assert_eq!(compact_string, *expected_compact_string);
            assert_eq!(
                root.size(&JSONFormat::Compact),
                Size::from(*expected_compact_string)
            );
            // Test pretty string
            let pretty_string = root.to_text(&JSONFormat::Pretty);
            assert_eq!(pretty_string, *expected_pretty_string);
            assert_eq!(
                root.size(&JSONFormat::Pretty),
                Size::from(*expected_pretty_string)
            );
            // Test debug tree view
            let mut s = String::new();
            root.write_tree_view(&mut s);
            assert_eq!(s, *tree_string);
        }
    }
}
