use std::{fmt, rc::Rc};

use sapling_grammar::{Grammar, TokenId, TypeId};

/// A syntax tree which fully stores the state of a text buffer (including formatting)
#[derive(Debug, Clone)]
pub struct Tree {
    /// The whitespace which appears before the first token in a file.  All tokens own the
    /// whitespace which comes directly after them, but there is no token to own the start of the
    /// file so we have to add that whitespace separately (although we could possibly add a dummy
    /// token to one end of the file to prevent this).
    leading_whitespace: String,
    /// The root node of the tree
    root: Node,
}

#[derive(Debug, Clone)]
pub enum Node {
    Tree(TreeNode),
    Stringy(StringyNode),
}

/// An syntax tree node which contains a sequence of tokens and sub-nodes
#[derive(Debug, Clone)]
pub struct TreeNode {
    type_: TypeId,
    contents: Vec<Elem>,
}

/// An syntax tree node representing a 'stringy' node.  This node cannot contain sub-nodes, but
/// instead contains an arbitrary string value that can be edited by the user.  This is very useful
/// for e.g. identifiers or string/numeric literals.
#[derive(Debug, Clone)]
pub struct StringyNode {
    type_: TypeId,
    /// The un-escaped contents of this node
    internal_str: String,
    /// The escaped contents of this node which should be added to the file
    display_str: String,
}

#[derive(Debug, Clone)]
pub enum Elem {
    /// The token and any whitespace which directly follows it
    Token { token: TokenId, whitespace: String },
    /// This element contains a [`Node`] which stores a sub-tree.  This can be replaced with any
    /// [`Node`] who's [`Type`] is a descendent of `type_bound`.
    Node { type_bound: TypeId, node: Rc<Node> },
}

///////////////////////
// STRING CONVERSION //
///////////////////////

impl Tree {
    pub fn write_text(&self, w: &mut impl fmt::Write, grammar: &Grammar) -> fmt::Result {
        w.write_str(&self.leading_whitespace)?;
        self.root.write_text(w, grammar)
    }
}

impl Node {
    pub fn write_text(&self, w: &mut impl fmt::Write, grammar: &Grammar) -> fmt::Result {
        match self {
            Node::Tree(TreeNode { contents, .. }) => {
                for elem in contents {
                    elem.write_text(w, grammar)?;
                }
                Ok(())
            }
            Node::Stringy(StringyNode { display_str, .. }) => w.write_str(display_str),
        }
    }
}

impl Elem {
    pub fn write_text(&self, w: &mut impl fmt::Write, grammar: &Grammar) -> fmt::Result {
        match self {
            Elem::Token { token, whitespace } => {
                w.write_str(grammar.token_text(*token))?;
                w.write_str(whitespace)
            }
            Elem::Node { node, .. } => node.write_text(w, grammar),
        }
    }
}
