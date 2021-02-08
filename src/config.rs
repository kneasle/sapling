//! Module to hold all user-configurable parameters, until we find a better way to handle
//! configuration

//use std::array::IntoIter;

use crate::ast::display_token::{syntax_category::*, SyntaxCategory};
use crate::core::Direction;
use crate::editor::{command_mode, normal_mode}; //::CmdType;

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

/// Mapping of keys to keystrokes in normal mode.
/// Shortcut definition, also allows us to change the type if needed.
pub type NormalModeKeyMap = std::collections::HashMap<Key, normal_mode::CmdType>;

/// Generates a 'canonical' [`NormalModeKeyMap`].  These keybindings will be very similar to those of Vim.
pub fn normal_mode_default_keymap() -> NormalModeKeyMap {
    hmap::hmap! {
        Key::Char('i') => normal_mode::CmdType::InsertBefore,
        Key::Char('a') => normal_mode::CmdType::InsertAfter,
        Key::Char('o') => normal_mode::CmdType::InsertChild,
        Key::Char('r') => normal_mode::CmdType::Replace,
        Key::Char('x') => normal_mode::CmdType::Delete,
        Key::Char('c') => normal_mode::CmdType::MoveCursor(Direction::Down),
        Key::Char('p') => normal_mode::CmdType::MoveCursor(Direction::Up),
        Key::Char('h') => normal_mode::CmdType::MoveCursor(Direction::Prev),
        Key::Char('j') => normal_mode::CmdType::MoveCursor(Direction::Next),
        Key::Char('k') => normal_mode::CmdType::MoveCursor(Direction::Prev),
        Key::Char('l') => normal_mode::CmdType::MoveCursor(Direction::Next),
        Key::Char('u') => normal_mode::CmdType::Undo,
        Key::Char('R') => normal_mode::CmdType::Redo,
        Key::Char(':') => normal_mode::CmdType::CommandMode
    }
}

/// /// Mapping of keys to keystrokes in command mode.
pub type CommandModeKeyMap = std::collections::HashMap<Key, command_mode::CmdType>;

/// Generates a 'canonical' [`CommandModeKeyMap`].  These keybindings will be very similar to those of Vim.
pub fn command_mode_default_keymap() -> CommandModeKeyMap {
    hmap::hmap! {
        Key::Char('q') => command_mode::CmdType::Quit,
        Key::Char('w') => command_mode::CmdType::Write,
        Key::Char('d') => command_mode::CmdType::DotGraph,
        Key::ESC => command_mode::CmdType::NormalMode
    }
}
/* COMPLETE CONFIG */

/// A struct to hold the entire run-time configuration of Sapling
#[derive(Debug, Clone)]
pub struct Config {
    /// A mapping between [`char`]s and [`crate::editor::normal_mode::CmdType`]s in normal mode
    pub normal_mode_keymap: NormalModeKeyMap,
    /// A mapping between [`char`]s and [`crate::editor::command_mode::CmdType`]s in command mode
    pub command_mode_keymap: CommandModeKeyMap,
    /// The current [`ColorScheme`] of Sapling
    pub color_scheme: ColorScheme,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            normal_mode_keymap: normal_mode_default_keymap(),
            command_mode_keymap: command_mode_default_keymap(),
            color_scheme: default_color_scheme(),
        }
    }
}
