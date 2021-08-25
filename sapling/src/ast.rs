use std::{
    fmt::{Display, Formatter},
    rc::Rc,
};

use crate::grammar::{
    self,
    compiled::{Grammar, StringyType, Token, Type},
};

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
    type_: Rc<Type>,
    contents: Vec<Elem>,
}

/// An syntax tree node representing a 'stringy' node.  This node cannot contain sub-nodes, but
/// instead contains an arbitrary string value that can be edited by the user.  This is very useful
/// for e.g. identifiers or string/numeric literals.
#[derive(Debug, Clone)]
pub struct StringyNode {
    type_: Rc<StringyType>,
    /// The un-escaped contents of this node
    internal_str: String,
    /// The escaped contents of this node which should be added to the file
    display_str: String,
}

#[derive(Debug, Clone)]
pub enum Elem {
    /// The token and any whitespace which directly follows it
    Token {
        token: Rc<Token>,
        whitespace: String,
    },
    /// This element contains a [`Node`] which stores a sub-tree.  This can be replaced with any
    /// [`Node`] who's [`Type`] is a descendent of `type_bound`.
    Node {
        type_bound: Rc<Type>,
        node: Rc<Node>,
    },
}

///////////////////////
// STRING CONVERSION //
///////////////////////

impl Display for Tree {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.leading_whitespace, self.root)
    }
}

impl Display for Node {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Node::Tree(TreeNode { contents, .. }) => {
                for elem in contents {
                    write!(f, "{}", elem)?;
                }
            }
            Node::Stringy(StringyNode { display_str, .. }) => write!(f, "{}", display_str)?,
        }
        Ok(())
    }
}

impl Display for Elem {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Elem::Token { token, whitespace } => write!(f, "{}{}", token, whitespace),
            Elem::Node { node, .. } => write!(f, "{}", node),
        }
    }
}

/////////////
// EXAMPLE //
/////////////

pub fn example() -> (Rc<Grammar>, Tree) {
    let json_grammar: Grammar = grammar::spec::expr().compile();
    let json_grammar = Rc::new(json_grammar);

    /*
    let root = Node::Tree(TreeNode {
        type_: json_grammar.get_type("value"),
        contents: vec![
            Elem::Token {
                token: json_grammar.get_token("["),
                whitespace: String::new(),
            },
            Elem::Token {
                token: json_grammar.get_token("]"),
                whitespace: String::new(),
            },
        ],
    });
    */

    todo!()
}
