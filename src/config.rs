//! Module to hold all user-configurable parameters, until we find a better way to handle
//! configuration

use crate::ast::display_token::{syntax_category::*, SyntaxCategory};
use crate::core::Direction;
use crate::editor::normal_mode::CmdType;

use tuikit::prelude::{Color, Key};

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
pub type KeyMap = std::collections::HashMap<Key, CmdType>;

/// Generates a 'canonical' [`KeyMap`].  These keybindings will be very similar to those of Vim.
pub fn default_keymap() -> KeyMap {
    hmap::hmap! {
        Key::Char('q') => CmdType::Quit,
        Key::Char('i') => CmdType::InsertBefore,
        Key::Char('a') => CmdType::InsertAfter,
        Key::Char('o') => CmdType::InsertChild,
        Key::Char('r') => CmdType::Replace,
        Key::Char('x') => CmdType::Delete,
        Key::Char('c') => CmdType::MoveCursor(Direction::Down),
        Key::Char('p') => CmdType::MoveCursor(Direction::Up),
        Key::Char('h') => CmdType::MoveCursor(Direction::Prev),
        Key::Char('j') => CmdType::MoveCursor(Direction::Next),
        Key::Char('k') => CmdType::MoveCursor(Direction::Prev),
        Key::Char('l') => CmdType::MoveCursor(Direction::Next),
        Key::Char('u') => CmdType::Undo,
        Key::Char('R') => CmdType::Redo
    }
}

/* COMPLETE CONFIG */

/// A struct to hold the entire run-time configuration of Sapling
#[derive(Debug, Clone)]
pub struct Config {
    /// A mapping between [`char`]s and [`CmdType`]s
    pub keymap: KeyMap,
    /// The current [`ColorScheme`] of Sapling
    pub color_scheme: ColorScheme,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            keymap: default_keymap(),
            color_scheme: default_color_scheme(),
        }
    }
}
