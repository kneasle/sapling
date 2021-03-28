//! Module to hold all user-configurable parameters, until we find a better way to handle
//! configuration

use crate::ast::display_token::{syntax_category::*, SyntaxCategory};
use crate::core::Direction;
use crate::editor::normal_mode::CmdType;

use crossterm::event::{KeyCode, KeyEvent};
use tui::style::Color;

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
        DEFAULT => Color::White,
        CONST => Color::Red,
        LITERAL => Color::Yellow,
        COMMENT => Color::Green,
        IDENT => Color::Cyan,
        KEYWORD => Color::Black,
        PRE_PROC => Color::Magenta,
        TYPE => Color::LightYellow,
        SPECIAL => Color::LightGreen,
        UNDERLINED => Color::LightRed,
        ERROR => Color::LightRed
    }
}

/* KEY BINDINGS */

/// Mapping of keys to keystrokes.
/// Shortcut definition, also allows us to change the type if needed.
pub type KeyMap = std::collections::HashMap<KeyCode, CmdType>;

/// Generates a 'canonical' [`KeyMap`].  These keybindings will be very similar to those of Vim.
pub fn default_keymap() -> KeyMap {
    hmap::hmap! {
        KeyCode::Char('q') => CmdType::Quit,
        KeyCode::Char('w') => CmdType::Write,
        KeyCode::Char('i') => CmdType::InsertBefore,
        KeyCode::Char('a') => CmdType::InsertAfter,
        KeyCode::Char('o') => CmdType::InsertChild,
        KeyCode::Char('r') => CmdType::Replace,
        KeyCode::Char('x') => CmdType::Delete,
        KeyCode::Char('c') => CmdType::MoveCursor(Direction::Down),
        KeyCode::Char('p') => CmdType::MoveCursor(Direction::Up),
        KeyCode::Char('h') => CmdType::MoveCursor(Direction::Prev),
        KeyCode::Char('j') => CmdType::MoveCursor(Direction::Next),
        KeyCode::Char('k') => CmdType::MoveCursor(Direction::Prev),
        KeyCode::Char('l') => CmdType::MoveCursor(Direction::Next),
        KeyCode::Char('u') => CmdType::Undo,
        KeyCode::Char('R') => CmdType::Redo
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
