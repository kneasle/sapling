//! A module to contain Rust representations of ASTs in a format that sapling can work with.

pub mod json;

/// The specification of an AST that sapling can edit
pub trait AST: Eq + Default {
    type FormatStyle;

    /// Write the textual representation of an AST to a string
    fn write_text(&self, string: &mut String, format_style: Self::FormatStyle);

    /// Make a [String] representing this AST.  Same as [write_text](AST::write_text) but creates a
    /// new [String].
    fn to_text(&self, format_style: Self::FormatStyle) -> String {
        let mut s = String::new();
        self.write_text(&mut s, format_style);
        s
    }
}
