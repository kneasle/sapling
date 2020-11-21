//! Specification of an editable, undoable buffer of trees and some implementations thereof.

pub mod cursor_path;
pub mod dag;

/// The possible ways you can move the cursor
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Direction {
    Up,
    Down,
    Prev,
    Next,
}

/// An enum to represent the two sides of a cursor
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Side {
    Prev,
    Next,
}
