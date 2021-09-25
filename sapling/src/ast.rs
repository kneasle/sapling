use std::{fmt, rc::Rc};

use sapling_grammar::{parser, Grammar, TokenId, TypeId};

/// A syntax tree which fully stores the state of a text buffer (including formatting)
#[derive(Debug, Clone)]
pub struct Tree {
    /// The whitespace which appears before the first token in a file.  All tokens own the
    /// whitespace which comes directly after them, but there is no token to own the start of the
    /// file so we have to add that whitespace separately (although we could possibly add a dummy
    /// token to one end of the file to prevent this).
    pub(crate) leading_ws: String,
    /// The root node of the tree
    pub(crate) root: Node,
}

#[derive(Debug, Clone)]
pub enum Node {
    Tree(TreeNode),
    Stringy(StringyNode, String),
}

impl parser::Ast for Node {
    type Builder = NodeBuilder;

    fn new_stringy(type_id: TypeId, contents: String, display_str: String, ws: &str) -> Self {
        Node::Stringy(
            StringyNode {
                type_: type_id,
                contents,
                display_str,
            },
            ws.to_owned(),
        )
    }
}

#[derive(Debug, Clone)]
pub struct NodeBuilder {
    type_id: TypeId,
    pattern: Vec<Elem>,
}

impl parser::Builder for NodeBuilder {
    type Node = Node;

    fn new(type_id: TypeId) -> Self {
        NodeBuilder {
            type_id,
            pattern: Vec::new(),
        }
    }

    fn add_node(&mut self, type_bound: TypeId, node: Rc<Self::Node>) {
        self.pattern.push(Elem::Node { type_bound, node });
    }

    fn add_token(&mut self, token: TokenId, ws: &str) {
        self.pattern.push(Elem::Token {
            token,
            ws: ws.to_owned(),
        });
    }

    fn seq_start(&mut self) {
        self.pattern.push(Elem::SeqStart);
    }

    fn seq_delim(&mut self, token: TokenId, ws: &str) {
        self.pattern.push(Elem::SeqDelim(token, ws.to_owned()));
    }

    fn seq_end(&mut self) {
        self.pattern.push(Elem::SeqEnd);
    }

    fn into_node(self) -> Self::Node {
        let NodeBuilder { type_id, pattern } = self;
        Node::Tree(TreeNode { type_id, pattern })
    }
}

/// An syntax tree node which contains a sequence of tokens and sub-nodes
#[derive(Debug, Clone)]
pub struct TreeNode {
    type_id: TypeId,
    pattern: Vec<Elem>,
}

#[derive(Debug, Clone)]
pub enum Elem {
    /// The token and any whitespace which directly follows it
    Token {
        token: TokenId,
        ws: String,
    },
    /// This element contains a [`Node`] which stores a sub-tree.  This can be replaced with any
    /// [`Node`] who's [`Type`] is a descendent of `type_bound`.
    Node {
        type_bound: TypeId,
        node: Rc<Node>,
    },
    SeqStart,
    SeqDelim(TokenId, String),
    SeqEnd,
}

/// An syntax tree node representing a 'stringy' node.  This node cannot contain sub-nodes, but
/// instead contains an arbitrary string value that can be edited by the user.  This is very useful
/// for e.g. identifiers or string/numeric literals.
#[derive(Debug, Clone)]
pub struct StringyNode {
    type_: TypeId,
    /// The un-escaped contents of this node
    contents: String,
    /// The escaped contents of this node which should be added to the file
    display_str: String,
}

///////////////////////
// STRING CONVERSION //
///////////////////////

impl Tree {
    pub fn to_text(&self, grammar: &Grammar) -> Result<String, fmt::Error> {
        let mut s = String::new();
        self.write_text(&mut s, grammar)?;
        Ok(s)
    }

    pub fn write_text(&self, w: &mut impl fmt::Write, grammar: &Grammar) -> fmt::Result {
        w.write_str(&self.leading_ws)?;
        self.root.write_text(w, grammar)
    }
}

impl Node {
    pub fn write_text(&self, w: &mut impl fmt::Write, grammar: &Grammar) -> fmt::Result {
        match self {
            Node::Tree(TreeNode {
                pattern: contents, ..
            }) => {
                for elem in contents {
                    elem.write_text(w, grammar)?;
                }
                Ok(())
            }
            Node::Stringy(StringyNode { display_str, .. }, ws) => {
                w.write_str(display_str)?;
                w.write_str(ws)
            }
        }
    }
}

impl Elem {
    pub fn write_text(&self, w: &mut impl fmt::Write, grammar: &Grammar) -> fmt::Result {
        match self {
            Elem::Token { token, ws } => {
                w.write_str(grammar.token_text(*token))?;
                w.write_str(ws)
            }
            Elem::Node { node, .. } => node.write_text(w, grammar),
            Elem::SeqStart | Self::SeqEnd => Ok(()),
            Elem::SeqDelim(token, ws) => {
                w.write_str(grammar.token_text(*token))?;
                w.write_str(ws)
            }
        }
    }
}
