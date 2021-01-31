//! A module to contain Rust representations of ASTs in a format that Sapling can work with.

pub mod display_token;
pub mod json;
pub mod test_json;

use std::error::Error;

use crate::arena::Arena;
use crate::core::Size;
use display_token::{write_tokens, DisplayToken, RecTok};

/// The possible ways an insertion could fail
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum InsertError {
    /// Inserting the node would cause the child count to exceed the limit for that node type
    TooManyChildren {
        /// The name of the node who's child count has been exceeded
        name: String,
        /// The maximum number of children that the node being inserted into could have
        max_children: usize,
    },
}

impl std::fmt::Display for InsertError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InsertError::TooManyChildren { name, max_children } => write!(
                f,
                "Can't exceed child count limit of {} in {}",
                max_children, name
            ),
        }
    }
}

impl Error for InsertError {}

/// The possible ways a deletion could fail
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum DeleteError {
    /// Deleting the requested node(s) would cause the parent to have too few children
    TooFewChildren {
        /// The [`display_name`](Ast::display_name) of the node who's minimum child count
        /// constraint has been violated
        name: String,
        /// The minimum number of children that the node in question could have had
        min_children: usize,
    },
    /// The requsted node doesn't exist.  This shouldn't be able to occur in practice, because it
    /// would require selecting a non-existent node - but nevertheless I don't think Sapling should
    /// panic in this situation.
    IndexOutOfRange {
        /// The length of the current child array
        len: usize,
        /// The index of the child that was attempted to be removed
        index: usize,
    },
}

impl std::fmt::Display for DeleteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeleteError::TooFewChildren { name, min_children } => write!(
                f,
                "Node type {} can't have fewer than {} children.",
                name, min_children
            ),
            DeleteError::IndexOutOfRange { len, index } => write!(
                f,
                "Deleting child index {} is out of range 0..{}",
                index, len
            ),
        }
    }
}

/// A function that recursively writes the tree view of a node and all its children to a given
/// [`String`].  To avoid allocations, this function modifies a [`String`] buffer
/// `indentation_string`, which will be appended to the front of every line, and will cause the
/// indentation levels to increase.
fn write_tree_view_recursive<'arena, Node>(
    node: &'arena Node,
    string: &mut String,
    indentation_string: &mut String,
) where
    Node: Ast<'arena>,
{
    // Push the node's display name with indentation and a newline
    string.push_str(indentation_string);
    string.push_str(&node.display_name());
    string.push('\n');
    // Indent by two spaces
    indentation_string.push_str("  ");
    // Write all the children
    for child in node.children().iter() {
        write_tree_view_recursive(*child, string, indentation_string);
    }
    // Reset indentation
    for _ in 0..2 {
        indentation_string.pop();
    }
}

/// A macro to generate an implementation of [`AstClass`] called `Class` and automatically fill the
/// required information.
#[macro_export]
macro_rules! ast_class {
    ($( $variant_name: ident => $c: expr, $name: literal );+) => {
        #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
        #[allow(missing_docs)]
        pub enum Class {
            $( $variant_name ),+
        }

        impl AstClass for Class {
            fn to_char(self) -> char {
                match self {
                    $( Class::$variant_name => $c ),+
                }
            }

            fn name(self) -> &'static str {
                match self {
                    $( Class::$variant_name => $name ),+
                }
            }

            fn from_char(c: char) -> Option<Self> {
                match c {
                    $( $c => Some(Class::$variant_name), )+
                    _ => None,
                }
            }
        }
    };
}

/// All the possible types which [`Ast`] nodes can take.
pub trait AstClass: Copy + std::fmt::Debug + Eq + std::hash::Hash {
    /// Gets the [`char`] that would have been used to create this value
    fn to_char(self) -> char;

    /// Returns the name of this value
    fn name(self) -> &'static str;

    /// Creates a `AstClass` from a [`char`], returning [`None`] if invalid.
    fn from_char(c: char) -> Option<Self>;
}

/// The specification of an AST that sapling can edit
pub trait Ast<'arena>: std::fmt::Debug + Clone + Eq + Default + std::hash::Hash {
    /// A type parameter that will represent the different ways this AST can be rendered
    type FormatStyle;
    /// A type parameter that will represent the different node types this AST can use
    type Class: AstClass;
    /// The error type for ways that parsing can fail
    type ParseErr: std::error::Error;

    /* FORMATTING FUNCTIONS */

    /// Returns an iterator of all the items that need to be rendered to the screen to make up this
    /// node, along with their on-screen locations.
    fn display_tokens_rec(
        &'arena self,
        format_style: &Self::FormatStyle,
    ) -> Vec<RecTok<'arena, Self>>;

    /// Parses from text and adds to an arena, return a pointer to the allocated root node.
    fn parse_to_arena(
        text: impl std::io::Read,
        arena: &'arena mut Arena<Self>,
    ) -> Result<&'arena Self, Self::ParseErr>;

    /// Uses [`display_tokens_rec`](Self::display_tokens_rec) to build a stream of
    /// [`DisplayToken`]s representing this node, but where each [`DisplayToken`] is paired with a
    /// reference to the node that owns it.  This extra data is used by the rendering code to
    /// determine which pieces of text correspond to nodes that are selected.
    fn display_tokens(
        &'arena self,
        format_style: &Self::FormatStyle,
    ) -> Vec<(&'arena Self, DisplayToken)> {
        let mut tok_pairs: Vec<(&'arena Self, DisplayToken)> = Vec::new();
        for i in self.display_tokens_rec(format_style) {
            match i {
                RecTok::Tok(t) => {
                    tok_pairs.push((self, t));
                }
                RecTok::Child(c) => {
                    tok_pairs.extend(c.display_tokens(format_style));
                }
            }
        }
        tok_pairs
    }

    /// Determine the space on the screen occupied by this node in an AST
    fn size(&self, format_style: &Self::FormatStyle) -> Size;

    /// Write the textual representation of this AST to a string
    fn write_text(&'arena self, string: &mut String, format_style: &Self::FormatStyle) {
        write_tokens(self, string, format_style);
    }

    /// Make a [`String`] representing this AST.
    /// Same as [`write_text`](Ast::write_text) but creates a new [`String`].
    fn to_text(&'arena self, format_style: &Self::FormatStyle) -> String {
        let mut s = String::new();
        self.write_text(&mut s, format_style);
        s
    }

    /* DEBUG VIEW FUNCTIONS */

    /// Get a slice over the direct children of this node.  This operation is expected to be cheap
    /// - it will be used a lot of times without caching the results.
    fn children<'s>(&'s self) -> &'s [&'arena Self];

    /// Get a mutable slice over the direct children of this node.  Like
    /// [`children`](Ast::children), this operation is expected to be cheap - it will be used a lot
    /// of times without caching the results.
    fn children_mut<'s>(&'s mut self) -> &'s mut [&'arena Self];

    /// Replaces the `index`th child of this node with a reference to a `new_node`
    fn replace_child(&mut self, index: usize, new_node: &'arena Self) {
        self.children_mut()[index] = new_node;
    }

    /// Removes the child at a given index from the children of this node, if possible.  If the
    /// removal was not possible, then we return a custom error type.
    fn delete_child(&mut self, index: usize) -> Result<(), DeleteError>;

    /// Insert a given pre-allocated node as a new child of this node.  This can involve allocating
    /// extra nodes (usually as ancestors of `new_node` but descendants of `self`).  This is
    /// required for cases like inserting into JSON objects (e.g. inserting true into the empty
    /// object will correspond to two extra nodes being allocated (an empty string and a field):
    /// `{}` -> `{"": true}`).
    fn insert_child(
        &mut self,
        new_node: &'arena Self,
        arena: &'arena Arena<Self>,
        index: usize,
    ) -> Result<(), InsertError>;

    /// Get the display name of this node
    fn display_name(&self) -> String;

    /// Append a debug-style tree view of this node to a [`String`], similar to the output of the
    /// Unix command 'tree'
    fn write_tree_view(&'arena self, string: &mut String) {
        let mut indentation_string = String::new();
        write_tree_view_recursive(self, string, &mut indentation_string);
        // Pop the unnecessary newline at the end
        let popped_char = string.pop();
        debug_assert_eq!(Some('\n'), popped_char);
    }

    /// Build a string of the a tree view of this node, similar to the output of the Unix command
    /// 'tree'.  This is the same as [`write_tree_view`](Self::write_tree_view), except that it
    /// returns a new [`String`] rather than appending to an existing [`String`].
    fn tree_view(&'arena self) -> String {
        let mut s = String::new();
        self.write_tree_view(&mut s);
        s
    }

    /* AST EDITING FUNCTIONS */

    /// Generate a new node from a AstClass.
    fn from_class(node_type: Self::Class) -> Self;

    /// Returns whether or not a given index and [`char`] is a valid child
    fn is_valid_child(&self, index: usize, node_type: Self::Class) -> bool;

    /// Returns whether or not a give index and ['char'] is a valid root
    fn is_valid_root(&self, node_type: Self::Class) -> bool;
}
