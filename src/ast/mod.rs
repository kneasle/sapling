//! A module to contain Rust representations of ASTs in a format that sapling can work with.

pub mod json;

/// The specification of an AST that sapling can edit
pub trait AST: Eq + Default {
    /// Write the textual representation of an AST to a string
    fn write_text(&self, string: &mut String);

    /// Make a [String] representing this AST.  Same as [write_text](AST::write_text) but creates a
    /// new [String].
    fn to_text(&self) -> String {
        let mut s = String::new();
        self.write_text(&mut s);
        s
    }
}
