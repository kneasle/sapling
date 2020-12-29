//! Module to hold all user-configurable parameters, until we find a better way to handle
//! configuration

use crate::ast::display_token::{syntax_category::*, SyntaxCategory};
use crate::core::Direction;
use crate::editor::normal_mode::KeyStroke;

use tuikit::prelude::Color;

/* DEBUG FLAGS */

/// Setting this flag to `true` will override the current syntax highlighting with a debug view
/// where every node is highlighted according to its hash value.
///
/// This mode is not useful for text editing, but is very useful for debugging.
pub const DEBUG_HIGHLIGHTING: bool = false;

/* COLOR SCHEME */

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

/* KEY BINDINGS */

/// Mapping of keys to keystrokes.
/// Shortcut definition, also allows us to change the type if needed.
pub type KeyMap = std::collections::HashMap<char, KeyStroke>;

/// Generates a 'canonical' [`KeyMap`].  These keybindings will be very similar to those of Vim.
pub fn default_keymap() -> KeyMap {
    hmap::hmap! {
        'q' => KeyStroke::Quit,
        'i' => KeyStroke::InsertBefore,
        'a' => KeyStroke::InsertAfter,
        'o' => KeyStroke::InsertChild,
        'r' => KeyStroke::Replace,
        'x' => KeyStroke::Delete,
        'c' => KeyStroke::MoveCursor(Direction::Down),
        'p' => KeyStroke::MoveCursor(Direction::Up),
        'h' => KeyStroke::MoveCursor(Direction::Prev),
        'j' => KeyStroke::MoveCursor(Direction::Next),
        'k' => KeyStroke::MoveCursor(Direction::Prev),
        'l' => KeyStroke::MoveCursor(Direction::Next),
        'u' => KeyStroke::Undo,
        'R' => KeyStroke::Redo
    }
}

