//! Module to hold all user-configurable parameters, until we find a better way to handle
//! configuration

use crate::ast::display_token::{syntax_category::*, SyntaxCategory};
use tuikit::prelude::Color;

/// Setting this flag to `true` will override the current syntax highlighting with a debug view
/// where every node is highlighted according to its hash value.
///
/// This mode is not useful for text editing, but is very useful for debugging.
pub const DEBUG_HIGHLIGHTING: bool = false;

/// A mapping from syntax highlighting categories to terminal [`Color`]s
pub type ColorScheme = std::collections::HashMap<SyntaxCategory, Color>;

/// Return the default [`ColorScheme`] of Sapling
pub fn default_color_scheme() -> ColorScheme {
    hmap::hmap! {
        DEFAULT => Color::WHITE,
        CONST => Color::RED,
        LITERAL => Color::YELLOW,
        COMMENT => Color::GREEN,
        IDENT => Color::CYAN,
        KEYWORD => Color::BLUE,
        PRE_PROC => Color::MAGENTA,
        TYPE => Color::LIGHT_YELLOW,
        SPECIAL => Color::LIGHT_GREEN,
        UNDERLINED => Color::LIGHT_RED,
        ERROR => Color::LIGHT_RED
    }
}
