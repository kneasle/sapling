mod path;
pub use path::Path;

/// The possible ways you can move the cursor
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Direction {
    Up,
    Down,
    Prev,
    Next,
}

/// An enum to represent the two sides of a node
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Side {
    Prev,
    Next,
}

impl Side {
    /// Converts this `Side` into either `"before"` or `"after"`
    pub fn relational_word(&self) -> &'static str {
        match self {
            Side::Prev => "before",
            Side::Next => "after",
        }
    }
}

pub const ZERO: Size = Size::new(0, 0);
/// A struct used to represent the screen space occupied by a single node of an AST.  This can be
/// thought of as the size of the bounding box of that node.  The important thing about this is
/// that it is independent of the text indentation, meaning that if a node gets reused multiple
/// times in a tree, then it's `Size` will always be the same.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Size {
    lines: usize,
    last_line_length: usize,
}

impl Size {
    /// Constructs a new `Size` from its parts
    pub const fn new(lines: usize, last_line_length: usize) -> Size {
        Size {
            lines,
            last_line_length,
        }
    }

    /// Returns how many `\n` characters this node contains.  For example, the node `true`
    /// occupies `0` lines, whereas the following:
    /// ```text
    /// {
    ///     "foo": true,
    ///     "bar": false
    /// }
    /// ```
    /// occupies `3` lines.
    pub fn lines(&self) -> usize {
        self.lines
    }

    /// Returns how many characters long the last line of this `Size` occupies.  For example, the
    /// last (and only) line of `true` occupies `4` [`char`]s, whereas the last line of
    /// ```text
    /// {
    ///     "foo": true,
    ///     "bar": false
    /// }
    /// ```
    /// occupies `1` [`char`].
    pub fn last_line_length(&self) -> usize {
        self.last_line_length
    }
}

impl From<&str> for Size {
    fn from(string: &str) -> Size {
        let lines = string.chars().filter(|x| *x == '\n').count();
        let last_line_length = string.chars().rev().take_while(|x| *x != '\n').count();
        Size::new(lines, last_line_length)
    }
}

impl std::ops::Add for Size {
    type Output = Size;

    fn add(self, other: Size) -> Size {
        if other.lines == 0 {
            // If `other` only occupies one line, then it should just be stuck onto the last line
            // of this `Size`.  This is much like how `display: inline;` works in CSS.
            Size {
                lines: self.lines,
                last_line_length: self.last_line_length + other.last_line_length,
            }
        } else {
            // If `other` occupies more than one line, then it doesn't matter how long the last
            // line of `self` is, because the last line of the combined `Size` will be as long as
            // the last line of `other`.
            Size {
                lines: self.lines + other.lines,
                last_line_length: other.last_line_length,
            }
        }
    }
}

impl std::ops::AddAssign for Size {
    fn add_assign(&mut self, other: Size) {
        if other.lines == 0 {
            // If `other` only occupies one line, then it should just be stuck onto the last line
            // of this `Size`.  This is much like how `display: inline;` works in CSS.
            self.last_line_length += other.last_line_length;
        } else {
            // If `other` occupies more than one line, then it doesn't matter how long the last
            // line of `self` is, because the last line of the combined `Size` will be as long as
            // the last line of `other`.
            self.lines += other.lines;
            self.last_line_length = other.last_line_length;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Size, ZERO};

    #[test]
    fn from_str() {
        for (string, lines, last_line_length) in &[
            ("", 0, 0),
            ("true", 0, 4),
            ("false", 0, 5),
            ("\n", 1, 0),
            ("\n,", 1, 1),
            ("Some text\n", 1, 0),
            ("{\n   true\n},", 2, 2),
        ] {
            assert_eq!(Size::from(*string), Size::new(*lines, *last_line_length));
        }
    }

    #[test]
    fn add() {
        let tests: &[&[&str]] = &[
            &["[", "]"],
            &["[", "true", "]"],
            &["[\n    ", "true", ",\n    ", "false", "\n]"],
            &["\n\t\r", "bang", "\n\n\n\n last line here!\r"],
        ];
        for strings in tests {
            let mut total_size_add = ZERO;
            let mut total_size_add_assign = ZERO;
            let mut full_string = String::new();
            for s in *strings {
                total_size_add = total_size_add + Size::from(*s);
                total_size_add_assign += Size::from(*s);
                full_string.push_str(s);
            }
            assert_eq!(total_size_add, Size::from(full_string.as_str()));
            assert_eq!(total_size_add_assign, Size::from(full_string.as_str()));
        }
    }
}
