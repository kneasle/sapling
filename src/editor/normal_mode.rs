//! The code for 'normal-mode', similar to that of Vim

use super::dag::{LogMessage, DAG};
use super::{keystroke_log::Category, state};
use crate::ast::Ast;
use crate::config::{Config, KeyMap};
use crate::core::Direction;

use std::borrow::Cow;

use tuikit::prelude::Key;

/// The struct covering all the [`State`](state::State)s which correspond to Sapling being in
/// normal mode.
#[derive(Debug, Clone)]
pub struct State {
    keystroke_buffer: String,
}

impl Default for State {
    fn default() -> Self {
        State {
            keystroke_buffer: String::new(),
        }
    }
}

impl<'arena, Node: Ast<'arena>> state::State<'arena, Node> for State {
    // TODO: Fix some of the jank of this function
    fn transition(
        mut self: Box<Self>,
        key: Key,
        config: &Config,
        tree: &mut DAG<'arena, Node>,
    ) -> (
        Box<dyn state::State<'arena, Node>>,
        Option<(String, Category)>,
    ) {
        let c = match key {
            Key::Char(c) => c,
            _ => {
                self.keystroke_buffer.clear();
                return (
                    self,
                    Some(("Invalid command".to_owned(), Category::Undefined)),
                );
            }
        };

        self.keystroke_buffer.push(c);

        let log_entry = match parse_keystroke(&config.keymap, &self.keystroke_buffer) {
            ParseResult::Action(action) => {
                tree.execute_action(action).log_message();
                (action.description(), action.category())
            }
            ParseResult::Quit => {
                self.keystroke_buffer.clear();
                return (
                    Box::new(state::Quit),
                    Some(("Quit Sapling".to_owned(), Category::Quit)),
                );
            }
            ParseResult::Incomplete => return (self, None),
            ParseResult::Undefined(s) => {
                (format!("Undefined command '{}'", s), Category::Undefined)
            }
        };

        // If we haven't returned yet, then clear the buffer
        self.keystroke_buffer.clear();

        (self, Some(log_entry))
    }

    fn keystroke_buffer<'s>(&'s self) -> Cow<'s, str> {
        Cow::from(&self.keystroke_buffer)
    }
}

/// The possible keystroke typed by user without any parameters.  Each `KeyStroke` can be mapped to
/// an individual [`char`].
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum KeyStroke {
    /// Quit Sapling
    Quit,
    /// Replace the selected node, expects an argument
    Replace,
    /// Insert a new node as the last child of the cursor, expects an argument
    InsertChild,
    /// Insert a new node before the cursor, expects an argument
    InsertBefore,
    /// Insert a new node after the cursor, expects an argument
    InsertAfter,
    /// Delete the cursor
    Delete,
    /// Move cursor in given direction.  The direction is part of the keystroke, since the directions
    /// all correspond to single key presses.
    MoveCursor(Direction),
    /// Undo the last change
    Undo,
    /// Redo a change
    Redo,
}

impl KeyStroke {
    /// Returns a lower-case summary string of the given keystroke
    pub fn summary_string(&self) -> &'static str {
        match self {
            KeyStroke::Quit => "quit",
            KeyStroke::Replace => "replace",
            KeyStroke::InsertChild => "insert child",
            KeyStroke::InsertBefore => "insert before",
            KeyStroke::InsertAfter => "insert after",
            KeyStroke::Delete => "delete",
            KeyStroke::MoveCursor(Direction::Down) => "move to first child",
            KeyStroke::MoveCursor(Direction::Up) => "move to parent",
            KeyStroke::MoveCursor(Direction::Prev) => "move to previous sibling",
            KeyStroke::MoveCursor(Direction::Next) => "move to next sibling",
            KeyStroke::Undo => "undo",
            KeyStroke::Redo => "redo",
        }
    }
}

/// A single [`Action`] that can be actioned by [`DAG`]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum Action {
    /// Replace the selected node with a node represented by some [`char`]
    Replace(char),
    /// Insert a new node (given by some [`char`]) as the first child of the selected node
    InsertChild(char),
    /// Insert a new node (given by some [`char`]) before the cursor
    InsertBefore(char),
    /// Insert a new node (given by some [`char`]) after the cursor
    InsertAfter(char),
    /// Remove the node under the cursor
    Delete,
    /// Move the node in a given direction
    MoveCursor(Direction),
    /// Undo the last change
    Undo,
    /// Redo a change
    Redo,
}

impl Action {
    /// Returns a lower-case summary of the given keystroke, along with the color with which it
    /// should be displayed in the log.
    pub fn description(&self) -> String {
        match self {
            Action::Replace(c) => format!("replace cursor with '{}'", c),
            Action::InsertChild(c) => format!("insert '{}' as last child", c),
            Action::InsertBefore(c) => format!("insert '{}' before cursor", c),
            Action::InsertAfter(c) => format!("insert '{}' after cursor", c),
            Action::Delete => "delete cursor".to_string(),
            Action::MoveCursor(Direction::Down) => "move to first child".to_string(),
            Action::MoveCursor(Direction::Up) => "move to parent".to_string(),
            Action::MoveCursor(Direction::Prev) => "move to previous sibling".to_string(),
            Action::MoveCursor(Direction::Next) => "move to next sibling".to_string(),
            Action::Undo => "undo a change".to_string(),
            Action::Redo => "redo a change".to_string(),
        }
    }

    /// Returns the [`Category`] of this `Action`
    pub fn category(&self) -> Category {
        match self {
            Action::Replace(_) => Category::Replace,
            Action::InsertChild(_) | Action::InsertBefore(_) | Action::InsertAfter(_) => {
                Category::Insert
            }
            Action::Delete => Category::Delete,
            Action::MoveCursor(_) => Category::Move,
            Action::Undo | Action::Redo => Category::History,
        }
    }
}

/// The possible results of parsing a command
#[derive(Debug, Clone, Eq, PartialEq)]
enum ParseResult {
    /// An [`Action`] that can be executed by a [`DAG`]
    Action(Action),
    /// The user wanted to quit Sapling
    Quit,
    /// The command was complete but undefined
    Undefined(String),
    /// The command has not been finished yet
    Incomplete,
}

/// Attempt to convert a keystroke as a `&`[`str`] into an [`Action`].
/// This parses the string from the start, and returns when it finds a valid keystroke.
///
/// Therefore, `"q489flshb"` will be treated like `"q"`, and will return [`ParseResult::Quit`] even
/// though `"q489flshb"` is not technically valid.
/// This function is run every time the user types a keystroke character, and so the user would not
/// be able to input `"q489flshb"` to this function because doing so would require them to first
/// input every possible prefix of `"q489flshb"`, including `"q"`.
fn parse_keystroke(keymap: &KeyMap, keystroke: &str) -> ParseResult {
    parse_keystroke_opt(keymap, keystroke).unwrap_or(ParseResult::Incomplete)
}

fn parse_keystroke_opt(keymap: &KeyMap, keystroke: &str) -> Option<ParseResult> {
    let mut keystroke_char_iter = keystroke.chars();

    // Consume the first char of the keystroke
    let c = keystroke_char_iter.next()?;

    Some(ParseResult::Action(match keymap.get(&c) {
        // "q" quits Sapling
        Some(KeyStroke::Quit) => return Some(ParseResult::Quit),
        // this pattern is used several times: `keystroke_char_iter.next().map()
        // This consumes the second char of the iterator and, if it exists, returns
        // Some(Action::ThisAction(char))
        Some(KeyStroke::InsertChild) => Action::InsertChild(keystroke_char_iter.next()?),
        Some(KeyStroke::InsertBefore) => Action::InsertBefore(keystroke_char_iter.next()?),
        Some(KeyStroke::InsertAfter) => Action::InsertAfter(keystroke_char_iter.next()?),
        Some(KeyStroke::Delete) => Action::Delete,
        Some(KeyStroke::Replace) => Action::Replace(keystroke_char_iter.next()?),
        Some(KeyStroke::MoveCursor(direction)) => Action::MoveCursor(*direction),
        Some(KeyStroke::Undo) => Action::Undo,
        Some(KeyStroke::Redo) => Action::Redo,
        None => return Some(ParseResult::Undefined(keystroke.to_string())),
    }))
}

#[cfg(test)]
mod tests {
    use super::{parse_keystroke, Action, ParseResult};
    use crate::config::default_keymap;
    use crate::core::Direction;

    #[test]
    fn parse_keystroke_valid() {
        let keymap = default_keymap();
        for (keystroke, expected_effect) in &[
            ("x", Action::Delete),
            ("h", Action::MoveCursor(Direction::Prev)),
            ("j", Action::MoveCursor(Direction::Next)),
            ("k", Action::MoveCursor(Direction::Prev)),
            ("l", Action::MoveCursor(Direction::Next)),
            ("pajlbsi", Action::MoveCursor(Direction::Up)),
            ("ra", Action::Replace('a')),
            ("rg", Action::Replace('g')),
            ("oX", Action::InsertChild('X')),
            ("oP", Action::InsertChild('P')),
        ] {
            assert_eq!(
                parse_keystroke(&keymap, *keystroke),
                ParseResult::Action(expected_effect.clone())
            );
        }
    }

    #[test]
    fn parse_keystroke_quit() {
        assert_eq!(parse_keystroke(&default_keymap(), "q"), ParseResult::Quit);
    }

    #[test]
    fn parse_keystroke_invalid() {
        let keymap = default_keymap();
        for keystroke in &["d", "Pxx", "Qsx"] {
            assert_eq!(
                parse_keystroke(&keymap, *keystroke),
                ParseResult::Undefined(keystroke.to_string())
            );
        }
    }

    #[test]
    fn parse_keystroke_incomplete() {
        let keymap = default_keymap();
        for keystroke in &["", "r", "o"] {
            assert_eq!(
                parse_keystroke(&keymap, *keystroke),
                ParseResult::Incomplete
            );
        }
    }
}
