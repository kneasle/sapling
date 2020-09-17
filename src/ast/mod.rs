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

    /* AST EDITING FUNCTIONS */

    /// Generate an iterator over the possible shorthand [char]s that a user could type to replace
    /// this node with something else.
    fn get_replace_chars(&self) -> Box<dyn Iterator<Item = char>>;
    /// Generate a new node from a [char] that a user typed as part of the `r` command.  If `c` is
    /// an element of [get_replace_chars](AST::get_replace_chars), this must return `Some` value,
    /// if it isn't, then this can return `None`.
    fn from_replace_char(&self, c: char) -> Option<Self>;
}
