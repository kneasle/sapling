//! A module to contain Rust representations of ASTs in a format that sapling can work with.

pub mod json;

/// The specification of an AST that sapling can edit
pub trait AST: Eq + Default {
    type FormatStyle;

    /* FORMATTING FUNCTIONS */

    /// Write the textual representation of this AST to a string
    fn write_text(&self, string: &mut String, format_style: Self::FormatStyle);

    /// Make a [String] representing this AST.
    /// Same as [write_text](AST::write_text) but creates a new [String].
    fn to_text(&self, format_style: Self::FormatStyle) -> String {
        let mut s = String::new();
        self.write_text(&mut s, format_style);
        s
    }

    /* DEBUG VIEW FUNCTIONS */

    /// Get an iterator over the direct children of this node
    fn get_children<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Self> + 'a>;
    /// Get the display name of this node
    fn get_display_name(&self) -> String;

    fn write_tree_view_recursive(&self, string: &mut String, indentation_string: &mut String) {
        unimplemented!();
    }

    /// Render a tree view of this node, similar to the output of the Unix command 'tree'
    fn write_tree_view(&self, string: &mut String) {
        let mut indentation_string = String::new();
        self.write_tree_view_recursive(string, &mut indentation_string);
    }
    
    /// Build a string of the a tree view of this node, similar to the output of the Unix command
    /// 'tree'.  This is the same as [write_tree_view](AST::write_tree_view), except that it
    /// returns a [String] rather than appending to an existing [String].
    fn tree_view(&self) -> String {
        let mut s = String::new();
        self.write_tree_view(&mut s);
        s
    }

    /* AST EDITING FUNCTIONS */

    /// Generate an iterator over the possible shorthand [char]s that a user could type to replace
    /// this node with something else.
    fn get_replace_chars(&self) -> Box<dyn Iterator<Item = char>>;
    /// Generate a new node from a [char] that a user typed as part of the `r` command.  If `c` is
    /// an element of [get_replace_chars](AST::get_replace_chars), this must return `Some` value,
    /// if it isn't, then this can return `None`.
    fn from_replace_char(&self, c: char) -> Option<Self>;
}
