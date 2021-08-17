//! A hard-coded specification of JSON ASTs in a format editable by Sapling

use super::display_token::{syntax_category, DisplayToken, RecTok};
use super::{Ast, AstClass, DeleteError, InsertError};
use crate::arena::Arena;
use crate::ast_class;
use crate::core::Size;

use serde_json::Value;

/// Converts a [`serde_json::Value`] tree into a [`Json`] tree, whilst allocating the nodes into a
/// given arena.
pub fn add_value_to_arena<'arena>(
    json: Value,
    arena: &'arena Arena<Json<'arena>>,
) -> &'arena Json<'arena> {
    match json {
        Value::Null => arena.alloc(Json::Null),
        Value::Bool(true) => arena.alloc(Json::True),
        Value::Bool(false) => arena.alloc(Json::False),
        Value::Number(_) => unimplemented!(),
        Value::String(s) => arena.alloc(Json::Str(s)),
        Value::Array(children) => arena.alloc(Json::Array(
            children
                .into_iter()
                .map(|c| add_value_to_arena(c, arena))
                .collect(),
        )),
        Value::Object(fields) => arena.alloc(Json::Object(
            fields
                .into_iter()
                .map(|(key, value)| {
                    arena.alloc(Json::Field([
                        arena.alloc(Json::Str(key)),
                        add_value_to_arena(value, arena),
                    ]))
                })
                .collect(),
        )),
    }
}

/// An enum to hold the different ways that a JSON AST can be formatted
#[derive(Eq, PartialEq, Copy, Clone)]
pub enum JsonFormat {
    /// The most compact representation, has minimal whitespace.
    /// E.g. `[{"foo": true, "bar": false}, true]`
    Compact,
    /// A prettified representation, with pretty indenting and every element on a newline.
    Pretty,
}

ast_class!(
    True => 't', "true";
    False => 'f', "false";
    Null => 'n', "null";
    Array => 'a', "array";
    Object => 'o', "object";
    Str => 's', "string"
);

/// The sapling representation of the AST for a subset of JSON (where all values are either 'true'
/// or 'false', and keys only contain ASCII).
#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub enum Json<'arena> {
    /// The JSON value for 'true'.  Corresponds to the string `true`.
    True,
    /// The JSON value 'false'.  Corresponds to the string `false`.
    False,
    /// The JSON value 'null'.  Corresponds to the string `null`.
    Null,
    /// A JSON array of multiple values.
    /// Corresponds to a string `[<v1>, <v2>, ...]` where `v1`, `v2`, ... are Json values.
    Array(Vec<&'arena Json<'arena>>),
    /// A JSON object, represented as a map of [`String`]s to more Json values.
    /// Corresponds to a string `{"<key1>": <v1>, "<key2>": <v2>, ...}` where `<key1>`, `<key2>`,
    /// ... are the keys, and `<v1>`, `<v2>`, ... are the corresponding Json values.  The `Ref`s
    /// contained inside this must be [`Field`](Json::Field)s.
    Object(Vec<&'arena Json<'arena>>),
    /// A JSON object field.  The first `Ref` must be a [`Str`](Json::Str), and the second is any
    /// JSON object
    Field([&'arena Json<'arena>; 2]),
    /// A JSON string
    Str(String),
}

impl Default for Json<'_> {
    fn default() -> Json<'static> {
        Json::Object(vec![])
    }
}

impl<'arena> Ast<'arena> for Json<'arena> {
    type FormatStyle = JsonFormat;
    type Class = Class;
    type ParseErr = serde_json::Error;

    /* FORMATTING FUNCTIONS */

    fn display_tokens_rec(
        &'arena self,
        format_style: &Self::FormatStyle,
    ) -> Vec<RecTok<'arena, Self>> {
        let is_pretty = format_style == &JsonFormat::Pretty;
        match self {
            Json::True => vec![RecTok::from_str("true", syntax_category::CONST)],
            Json::False => vec![RecTok::from_str("false", syntax_category::CONST)],
            Json::Null => vec![RecTok::from_str("null", syntax_category::KEYWORD)],
            Json::Str(string) => vec![RecTok::from_string(
                format!(r#""{}""#, string),
                syntax_category::LITERAL,
            )],
            Json::Field([key, value]) => vec![
                RecTok::Child(key),
                RecTok::from_str(": ", syntax_category::DEFAULT),
                RecTok::Child(value),
            ],
            Json::Array(children) => {
                // Special case: if this array is empty, render it as '[]'
                if children.is_empty() {
                    return vec![RecTok::from_str("[]", syntax_category::DEFAULT)];
                }

                let mut tokens: Vec<RecTok<'_, Self>> = Vec::with_capacity(6 + 3 * children.len());
                // Push some initial tokens
                tokens.push(RecTok::from_str("[", syntax_category::DEFAULT));
                if is_pretty {
                    tokens.push(RecTok::Tok(DisplayToken::Indent));
                    tokens.push(RecTok::Tok(DisplayToken::Newline));
                }
                // Push the children, delimited by commas
                let mut is_first_child = true;
                for c in children {
                    // Push the delimiting
                    if !is_first_child {
                        tokens.push(RecTok::from_str(",", syntax_category::DEFAULT));
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
                tokens.push(RecTok::from_str("]", syntax_category::DEFAULT));
                // Return the token stream
                tokens
            }
            Json::Object(fields) => {
                // Special case: if this object is empty, render it as '{}'
                if fields.is_empty() {
                    return vec![RecTok::from_str("{}", syntax_category::DEFAULT)];
                }

                let mut tokens: Vec<RecTok<'_, Self>> = Vec::with_capacity(6 + 3 * fields.len());
                // Push some initial tokens
                tokens.push(RecTok::from_str("{", syntax_category::DEFAULT));
                if is_pretty {
                    tokens.push(RecTok::Tok(DisplayToken::Indent));
                    tokens.push(RecTok::Tok(DisplayToken::Newline));
                }
                // Push the children, delimited by commas
                let mut is_first_child = true;
                for f in fields {
                    // Push the delimiting
                    if !is_first_child {
                        tokens.push(RecTok::from_str(",", syntax_category::DEFAULT));
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
                tokens.push(RecTok::from_str("}", syntax_category::DEFAULT));
                // Return the token stream
                tokens
            }
        }
    }

    fn parse_to_arena(
        text: impl std::io::Read,
        arena: &'arena mut Arena<Self>,
    ) -> Result<&'arena Self, Self::ParseErr> {
        let value = serde_json::from_reader::<_, Value>(text)?;
        Ok(add_value_to_arena(value, arena))
    }

    fn size(&self, format_style: &Self::FormatStyle) -> Size {
        match format_style {
            JsonFormat::Pretty => {
                match self {
                    Json::True => Size::new(0, 4),  // same as Size::from("true")
                    Json::False => Size::new(0, 5), // same as Size::from("false")
                    Json::Null => Size::new(0, 4),  // same as Size::from("null")
                    Json::Str(string) => {
                        Size::new(0, 1) + Size::from(string.as_str()) + Size::new(0, 1)
                    }
                    Json::Field([key, value]) => {
                        key.size(format_style) + Size::new(0, 2) + value.size(format_style)
                    }
                    Json::Object(fields) => {
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
                    Json::Array(children) => {
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
            JsonFormat::Compact => {
                match self {
                    Json::True => Size::new(0, 4),  // same as Size::from("true")
                    Json::False => Size::new(0, 5), // same as Size::from("false")
                    Json::Null => Size::new(0, 4),  // same as Size::from("null")
                    Json::Str(string) => {
                        Size::new(0, 1) + Size::from(string.as_str()) + Size::new(0, 1)
                    }
                    Json::Field([key, value]) => {
                        key.size(format_style) + Size::new(0, 2) + value.size(format_style)
                    }
                    Json::Object(fields) => {
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
                    Json::Array(children) => {
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

    fn children<'s>(&'s self) -> &'s [&'arena Json<'arena>] {
        match self {
            Json::True | Json::False | Json::Null | Json::Str(_) => &[],
            Json::Array(children) => children,
            Json::Object(fields) => fields,
            Json::Field(key_value) => &key_value[..],
        }
    }

    fn children_mut<'s>(&'s mut self) -> &'s mut [&'arena Json<'arena>] {
        match self {
            Json::True | Json::False | Json::Null | Json::Str(_) => &mut [],
            Json::Array(children) => children,
            Json::Object(fields) => fields,
            Json::Field(key_value) => &mut key_value[..],
        }
    }

    fn insert_child(
        &mut self,
        new_node: &'arena Self,
        arena: &'arena Arena<Self>,
        index: usize,
    ) -> Result<(), InsertError> {
        match self {
            Json::True | Json::False | Json::Null | Json::Str(_) => {
                Err(InsertError::TooManyChildren {
                    name: self.display_name(),
                    max_children: 0,
                })
            }
            Json::Field(_) => Err(InsertError::TooManyChildren {
                name: self.display_name(),
                max_children: 2,
            }),
            Json::Object(fields) => {
                /* Inserting into an object is a special case, since we need to allocate more
                 * objects in order to preserve the validity of the tree. */
                // Allocate an empty string to act as the key
                let key = arena.alloc(Json::Str("".to_string()));
                // Allocate a field as the parent of the key and new_node
                let field = arena.alloc(Json::Field([key, new_node]));
                // Add the new field as a child of `self`
                fields.insert(index, field);
                Ok(())
            }
            Json::Array(children) => {
                children.insert(index, new_node);
                Ok(())
            }
        }
    }

    fn delete_child(&mut self, index: usize) -> Result<(), DeleteError> {
        match self {
            Json::True | Json::False | Json::Null | Json::Str(_) => {
                // We shouldn't be able to delete the child of a node with no children - this would
                // require first selecting the non-existent child, which should be caught by the
                // cursor path code.
                unreachable!();
            }
            Json::Field(_) => Err(DeleteError::TooFewChildren {
                name: self.display_name(),
                min_children: 2,
            }),
            Json::Object(fields) => {
                if index < fields.len() {
                    fields.remove(index);
                    Ok(())
                } else {
                    Err(DeleteError::IndexOutOfRange {
                        len: fields.len(),
                        index,
                    })
                }
            }
            Json::Array(children) => {
                if index < children.len() {
                    children.remove(index);
                    Ok(())
                } else {
                    Err(DeleteError::IndexOutOfRange {
                        len: children.len(),
                        index,
                    })
                }
            }
        }
    }

    fn display_name(&self) -> String {
        match self {
            Json::True => "true".to_string(),
            Json::False => "false".to_string(),
            Json::Null => "null".to_string(),
            Json::Array(_) => "array".to_string(),
            Json::Object(_) => "object".to_string(),
            Json::Field(_) => "field".to_string(),
            Json::Str(content) => format!(r#""{}""#, content),
        }
    }

    /* AST EDITING FUNCTIONS */

    fn from_class(node_type: Self::Class) -> Self {
        match node_type {
            Class::True => Json::True,
            Class::False => Json::False,
            Class::Null => Json::Null,
            Class::Array => Json::Array(vec![]),
            Class::Object => Json::Object(vec![]),
            Class::Str => Json::Str("".to_string()),
        }
    }

    fn is_valid_child(&self, index: usize, node_type: Self::Class) -> bool {
        match self {
            // values like 'true' and 'false' can never have children
            Json::True | Json::False | Json::Str(_) | Json::Null => false,
            // arrays and objects can have any children (except `field` inside `array`, which can't
            // be inserted)
            Json::Array(_) | Json::Object(_) => true,
            // fields must have their left hand side be a string
            Json::Field(_) => {
                if index == 0 {
                    node_type == Class::Str
                } else {
                    true
                }
            }
        }
    }

    fn is_valid_root(&self, _node_type: Class) -> bool {
        true
    }

    fn debug_name(&self) -> String {
        match self {
            Self::True => "True".to_owned(),
            Self::False => "False".to_owned(),
            Self::Null => "Null".to_owned(),
            Self::Str(string) => format!("\"{}\"", string),
            Self::Array(_) => "Array".to_owned(),
            Self::Field(_) => "Field".to_owned(),
            Self::Object(_) => "Object".to_owned(),
        }
    }
}

// Allow JSON to be compared to `serde_json::Value`s
impl PartialEq<Value> for Json<'_> {
    fn eq(&self, other: &Value) -> bool {
        match (self, other) {
            (Json::True, Value::Bool(true))
            | (Json::False, Value::Bool(false))
            | (Json::Null, Value::Null) => true,
            (Json::Str(s1), Value::String(s2)) => s1 == s2,
            (Json::Array(cs1), Value::Array(cs2)) => {
                cs1.iter().zip(cs2.iter()).all(|(a, b)| *a == b)
            }
            (Json::Object(fields), Value::Object(map)) => {
                if fields.len() != map.len() {
                    return false;
                }
                for &f in fields {
                    if let Json::Field([Json::Str(key), value]) = f {
                        if let Some(v) = map.get(key) {
                            if value != &v {
                                return false;
                            }
                        }
                    } else {
                        return false;
                    }
                }
                true
            }
            // Fields cannot be made into a `serde_json::Value`
            (Json::Field(_), _) => false,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{add_value_to_arena, JsonFormat};
    use crate::arena::Arena;
    use crate::ast::Ast;
    use crate::core::Size;

    use serde_json::json;

    #[test]
    fn to_text() {
        for (tree, expected_compact_string, expected_pretty_string, tree_string) in &[
            (json!(true), "true", "true", "true"),
            (json!(false), "false", "false", "false"),
            (json!([]), "[]", "[]", "array"),
            (json!({}), "{}", "{}", "object"),
            (
                json!([true, false]),
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
                json!({"foo": true, "bar": false}),
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
                json!([{"foos": [false, true, false], "bar": false}, true]),
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
            let root = add_value_to_arena(tree.clone(), &arena);
            // Test compact string
            let compact_string = root.to_text(&JsonFormat::Compact);
            assert_eq!(compact_string, *expected_compact_string);
            assert_eq!(
                root.size(&JsonFormat::Compact),
                Size::from(*expected_compact_string)
            );
            // Test pretty string
            let pretty_string = root.to_text(&JsonFormat::Pretty);
            assert_eq!(pretty_string, *expected_pretty_string);
            assert_eq!(
                root.size(&JsonFormat::Pretty),
                Size::from(*expected_pretty_string)
            );
            // Test debug tree view
            let mut s = String::new();
            root.write_tree_view(&mut s);
            assert_eq!(s, *tree_string);
        }
    }
}
