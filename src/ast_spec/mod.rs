//! A module to contain Rust representations of ASTs in a format that sapling can work with.

pub mod json;

#[allow(unused_imports)]
use crate::editable_tree::EditableTree;

/// A trait bound that specifies what types can be used as a reference to Node in an AST
pub trait Reference: Copy + Eq + std::fmt::Debug + std::hash::Hash {}

/// A trait bound for a type that can be used to access nodes (used to give [NodeMap]-like
/// attributes to [EditableTree]s).
pub trait ReadableNodeMap<Ref: Reference, Node: ASTSpec<Ref>> {
    /// Gets node from a reference, returning [None] if the reference is invalid.
    fn get_node<'a>(&'a self, id: Ref) -> Option<&'a Node>;

    /// Gets mutable node from a reference, returning [None] if the reference is invalid.
    fn get_node_mut<'a>(&'a mut self, id: Ref) -> Option<&'a mut Node>;

    /// Get the reference of the root node of the tree.  This is required to be a valid reference,
    /// i.e. `self.get_node(self.root())` should never return [None].
    fn root(&self) -> Ref;

    /// Get the node that is the root of the current tree
    fn root_node<'a>(&'a self) -> &'a Node {
        // We can unwrap here, because self.root() is required to be a valid reference.
        self.get_node(self.root()).unwrap()
    }

    /// Get the node that is the root of the current tree
    fn root_node_mut<'a>(&'a mut self) -> &'a mut Node {
        // We can unwrap here, because self.root() is required to be a valid reference.
        self.get_node_mut(self.root()).unwrap()
    }
}

/// A trait bound for a type that can store `Node`s, accessible by references.
pub trait NodeMap<Ref: Reference, Node: ASTSpec<Ref>>: ReadableNodeMap<Ref, Node> {
    /// Create a new `NodeMap` with a given `Node` as root
    fn with_root(root: Node) -> Self;

    /// Create a new `NodeMap` containing only the default node as root
    fn with_default_root() -> Self
    where
        Self: Sized,
    {
        Self::with_root(Node::default())
    }

    /// Set the root of the tree to be the node at a given reference, returning `true` if the
    /// reference was valid.  If the reference was invalid, the root will not be replaced and
    /// `false` will be returned.
    fn set_root(&mut self, new_root: Ref) -> bool;

    /// Adds a new node and set it to the tree's root
    fn add_as_root(&mut self, new_root_node: Node) -> Ref {
        let r = self.add_node(new_root_node);
        let is_valid = self.set_root(r);
        debug_assert!(is_valid);
        r
    }

    /// Writes the text rendering of the root node to a string (same as calling
    /// [to_text](ASTSpec::to_text) on the [root](ReadableNodeMap::root)).
    fn write_text(&self, string: &mut String, format_style: &Node::FormatStyle)
    where
        Self: Sized,
    {
        match self.get_node(self.root()) {
            Some(root) => {
                root.write_text(self, string, format_style);
            }
            None => {
                string.push_str("<INVALID ROOT NODE>");
            }
        }
    }

    /// Generates the text rendering of the root node (same as calling [to_text](ASTSpec::to_text)
    /// on the [root](ReadableNodeMap::root)).
    fn to_text(&self, format_style: &Node::FormatStyle) -> String
    where
        Self: Sized,
    {
        match self.get_node(self.root()) {
            Some(root) => root.to_text(self, format_style),
            None => "<INVALID ROOT NODE>".to_string(),
        }
    }

    /// Add a new `Node` to the tree, and return its reference
    fn add_node(&mut self, node: Node) -> Ref;
}

/// The specification of an AST that sapling can edit
pub trait ASTSpec<Ref: Reference>: Eq + Default {
    /// A type parameter that will represent the different ways this AST can be rendered
    type FormatStyle;

    /* FORMATTING FUNCTIONS */

    /// Write the textual representation of this AST to a string
    fn write_text(
        &self,
        node_map: &impl NodeMap<Ref, Self>,
        string: &mut String,
        format_style: &Self::FormatStyle,
    );

    /// Make a [String] representing this AST.
    /// Same as [write_text](ASTSpec::write_text) but creates a new [String].
    fn to_text(
        &self,
        node_map: &impl NodeMap<Ref, Self>,
        format_style: &Self::FormatStyle,
    ) -> String {
        let mut s = String::new();
        self.write_text(node_map, &mut s, format_style);
        s
    }

    /* DEBUG VIEW FUNCTIONS */

    /// Get an iterator over the direct children of this node
    fn get_children<'a>(&'a self) -> Box<dyn Iterator<Item = Ref> + 'a>;

    /// Get the display name of this node
    fn get_display_name(&self) -> String;

    fn write_tree_view_recursive(
        &self,
        node_map: &impl NodeMap<Ref, Self>,
        string: &mut String,
        indentation_string: &mut String,
    ) {
        unimplemented!();
    }

    /// Render a tree view of this node, similar to the output of the Unix command 'tree'
    fn write_tree_view(&self, node_map: &impl NodeMap<Ref, Self>, string: &mut String) {
        let mut indentation_string = String::new();
        self.write_tree_view_recursive(node_map, string, &mut indentation_string);
    }

    /// Build a string of the a tree view of this node, similar to the output of the Unix command
    /// 'tree'.  This is the same as [write_tree_view](ASTSpec::write_tree_view), except that it
    /// returns a [String] rather than appending to an existing [String].
    fn tree_view(&self, node_map: &impl NodeMap<Ref, Self>) -> String {
        let mut s = String::new();
        self.write_tree_view(node_map, &mut s);
        s
    }

    /* AST EDITING FUNCTIONS */

    /// Generate an iterator over the possible shorthand [char]s that a user could type to replace
    /// this node with something else.
    fn get_replace_chars(&self) -> Box<dyn Iterator<Item = char>>;

    /// Generate a new node from a [char] that a user typed as part of the `r` command.  If `c` is
    /// an element of [get_replace_chars](ASTSpec::get_replace_chars), this must return `Some` value,
    /// if it isn't, then this should return `None`.
    fn from_replace_char(&self, c: char) -> Option<Self>;
}
