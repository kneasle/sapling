//! The code for 'normal-mode', similar to that of Vim

use super::dag::{Dag, Insertable, LogMessage};
use super::{keystroke_log::Category, state};
use crate::ast::Ast;
use crate::config::{Config, KeyMap};
use crate::core::{Direction, Side};

use std::borrow::Cow;

use tuikit::prelude::Key;

/// The struct covering all the [`State`](state::State)s which correspond to Sapling being in
/// normal mode.
#[derive(Debug, Clone)]
pub struct State {
    keystroke_buffer: Vec<Key>,
}

impl Default for State {
    fn default() -> Self {
        State {
            keystroke_buffer: Vec::new(),
        }
    }
}

impl<'arena, Node: Ast<'arena>> state::State<'arena, Node> for State {
    // TODO: Fix some of the jank of this function
    fn transition(
        mut self: Box<Self>,
        key: Key,
        config: &Config,
        tree: &mut Dag<'arena, Node>,
    ) -> (
        Box<dyn state::State<'arena, Node>>,
        Option<(String, Category)>,
    ) {
        self.keystroke_buffer.push(key);

        let log_entry = match parse_keystroke(&config.keymap, &self.keystroke_buffer) {
            Ok(action) => {
                match action {
                    // If the command was a 'quit', then immediately make a state transition to the
                    // 'Quitted' state
                    Action::Quit => {
                        return (
                            Box::new(state::Quit),
                            Some((action.description(), action.category())),
                        )
                    }
                    // Otherwise, we perform the action on the `Dag`.  This returns the
                    // `EditResult`, which is logged outside the `match`
                    Action::Undo => tree.undo(),
                    Action::Redo => tree.redo(),
                    Action::MoveCursor(direction) => tree.move_cursor(direction),
                    Action::Replace(c) => tree.replace_cursor(c),
                    Action::InsertChild(c) => tree.insert_child(c),
                    Action::InsertBefore(c) => tree.insert_next_to_cursor(c, Side::Prev),
                    Action::InsertAfter(c) => tree.insert_next_to_cursor(c, Side::Next),
                    Action::Delete => tree.delete_cursor(),
                }
                .log_message();
                (action.description(), action.category())
            }
            Err(ParseErr::Incomplete) => return (self, None),
            Err(ParseErr::Invalid) => (
                format!("Undefined command '{:?}'", self.keystroke_buffer),
                Category::Undefined,
            ),
        };

        // If we haven't returned yet, then clear the buffer
        self.keystroke_buffer.clear();

        (self, Some(log_entry))
    }

    fn keystroke_buffer<'s>(&'s self) -> Cow<'s, str> {
        Cow::from(format!("{:?}", self.keystroke_buffer))
    }
}

/// The possible keystroke typed by user without any parameters.  Each `KeyStroke` can be mapped to
/// an individual [`char`].
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
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
    /// Move cursor in given direction.  The direction is part of the keystroke, since movements in
    /// all 4 directions are mapped to single characters.
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

/// A single [`Action`] that can be actioned by [`Dag`]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum Action {
    /// Replace the selected node with a node represented by some [`char`]
    Replace(Insertable),
    /// Insert a new node (given by some [`char`]) as the first child of the selected node
    InsertChild(Insertable),
    /// Insert a new node (given by some [`char`]) before the cursor
    InsertBefore(Insertable),
    /// Insert a new node (given by some [`char`]) after the cursor
    InsertAfter(Insertable),
    /// Remove the node under the cursor
    Delete,
    /// Move the node in a given direction
    MoveCursor(Direction),
    /// Undo the last change
    Undo,
    /// Redo a change
    Redo,
    /// Quit Sapling
    Quit,
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
            Action::Quit => "quit Sapling".to_string(),
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
            Action::Quit => Category::Quit,
        }
    }
}

type ParseResult<T> = Result<T, ParseErr>;

/// The possible ways a parsing operation could fail
#[derive(Debug, Clone, Eq, PartialEq)]
enum ParseErr {
    Invalid,
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
fn parse_insertable(
    keystroke_char_iter: &mut impl Iterator<Item = Key>,
) -> ParseResult<Insertable> {
    // Consume the next key or claim incompleteness
    let key = keystroke_char_iter.next().ok_or(ParseErr::Incomplete)?;
    // If the next keystroke is a `char`, then return it with success otherwise the command is
    // invalid
    if let Key::Char(c) = key {
        Ok(Insertable::CountedNode(1, c))
    } else {
        Err(ParseErr::Invalid)
    }
}

fn parse_keystroke(keymap: &KeyMap, keys: &[Key]) -> ParseResult<Action> {
    let mut key_iter = keys.iter().copied();

    // Consume the first char of the keystroke
    let first_key = key_iter.next().ok_or(ParseErr::Incomplete)?;

    Ok(match keymap.get(&first_key).ok_or(ParseErr::Invalid)? {
        KeyStroke::InsertChild => Action::InsertChild(parse_insertable(&mut key_iter)?),
        KeyStroke::InsertBefore => Action::InsertBefore(parse_insertable(&mut key_iter)?),
        KeyStroke::InsertAfter => Action::InsertAfter(parse_insertable(&mut key_iter)?),
        KeyStroke::Delete => Action::Delete,
        KeyStroke::Replace => Action::Replace(parse_insertable(&mut key_iter)?),
        KeyStroke::MoveCursor(direction) => Action::MoveCursor(*direction),
        KeyStroke::Undo => Action::Undo,
        KeyStroke::Redo => Action::Redo,
        // "q" quits Sapling
        KeyStroke::Quit => return Ok(Action::Quit),
    })
}

#[cfg(test)]
mod tests {
    use super::{parse_keystroke, Action, Insertable, ParseErr};
    use crate::config::default_keymap;
    use crate::core::Direction;
    use tuikit::prelude::Key;

    fn to_char_keys(string: &str) -> Vec<Key> {
        string.chars().map(|c| Key::Char(c)).collect::<Vec<_>>()
    }

    #[test]
    fn parse_keystroke_valid() {
        let keymap = default_keymap();
        for (keystrokes, expected_effect) in &[
            ("x", Action::Delete),
            ("h", Action::MoveCursor(Direction::Prev)),
            ("j", Action::MoveCursor(Direction::Next)),
            ("k", Action::MoveCursor(Direction::Prev)),
            ("l", Action::MoveCursor(Direction::Next)),
            ("pajlbsi", Action::MoveCursor(Direction::Up)),
            ("ra", Action::Replace(Insertable::CountedNode(1, 'a'))),
            ("rg", Action::Replace(Insertable::CountedNode(1, 'g'))),
            ("oX", Action::InsertChild(Insertable::CountedNode(1, 'X'))),
            ("oP", Action::InsertChild(Insertable::CountedNode(1, 'P'))),
            ("a3t", Action::InsertAfter(Insertable::CountedNode(3, 't'))),
            ("an", Action::InsertAfter(Insertable::CountedNode(1, 'n'))),
            ("a1n", Action::InsertAfter(Insertable::CountedNode(1, 'n'))),
            ("i0X", Action::InsertAfter(Insertable::CountedNode(0, 'X'))),
            ("ii", Action::InsertAfter(Insertable::CountedNode(1, 'i'))),
            ("a1x", Action::InsertAfter(Insertable::CountedNode(1, 'x'))),
            ("q", Action::Quit),
        ] {
            assert_eq!(
                parse_keystroke(&keymap, &to_char_keys(keystrokes)),
                Ok(expected_effect.clone())
            );
        }
    }

    #[test]
    fn parse_keystroke_invalid() {
        let keymap = default_keymap();
        for keystroke in &["d", "Pxx", "Qsx", "t", "Y", "X", "1", "3", "\""] {
            assert_eq!(
                parse_keystroke(&keymap, &to_char_keys(keystroke)),
                Err(ParseErr::Invalid)
            );
        }
    }

    #[test]
    fn parse_keystroke_incomplete() {
        let keymap = default_keymap();
        for keystroke in &["", "r", "o", "o3", "3", "1o", "a"] {
            assert_eq!(
                parse_keystroke(&keymap, &to_char_keys(keystroke)),
                Err(ParseErr::Incomplete)
            );
        }
    }
}
