//! The code for 'normal-mode', similar to that of Vim

use super::dag::{Insertable, LogMessage};
use super::{command_mode, keystroke_log::Category, state, Editor};
use crate::ast::Ast;
use crate::config::NormalModeKeyMap;
use crate::core::{keystrokes_to_string, Direction, Side};

use std::borrow::Cow;
use std::iter::Peekable;

use tuikit::prelude::Key;

/// The struct covering all the [`State`](state::State)s which correspond to Sapling being in
/// normal mode.
#[derive(Debug, Clone)]
pub struct State {
    name: &'static str,
    keystroke_buffer: Vec<Key>,
}

impl Default for State {
    fn default() -> Self {
        State {
            name: "NORMAL",
            keystroke_buffer: Vec::new(),
        }
    }
}

impl<'arena, Node: Ast<'arena>> state::State<'arena, Node> for State {
    fn transition(
        mut self: Box<Self>,
        key: Key,
        editor: &mut Editor<'arena, Node>,
    ) -> (
        Box<dyn state::State<'arena, Node>>,
        Option<(String, Category)>,
    ) {
        self.keystroke_buffer.push(key);

        let tree = &mut editor.tree;

        let log_entry =
            match parse_command(&editor.config.normal_mode_keymap, &self.keystroke_buffer) {
                // If the command buffer is a valid and complete command, then we execute the resulting
                // 'action'
                Ok((count, action)) => {
                    // If the count is 0, then the command does not execute.  So we short-circuit in
                    // this case
                    if count == 0 {
                        return (self, Some(("no action".to_owned(), Category::Undefined)));
                    }
                    match action {
                        // Otherwise, we perform the action on the `Dag`.  This returns the
                        // `EditResult`, which is logged outside the `match`
                        Action::Undo => tree.undo(count),
                        Action::Redo => tree.redo(count),
                        Action::MoveCursor(direction) => tree.move_cursor(count, direction),
                        Action::Replace(c) => tree.replace_cursor(count, c),
                        Action::InsertChild(c) => tree.insert_child(count, c),
                        Action::InsertBefore(c) => tree.insert_next_to_cursor(count, c, Side::Prev),
                        Action::InsertAfter(c) => tree.insert_next_to_cursor(count, c, Side::Next),
                        Action::Delete => tree.delete_cursor(count),
                        Action::CommandMode => {
                            return (
                                Box::new(command_mode::State::default()),
                                Some((action.description(), action.category())),
                            );
                        } // TODO fix this
                    }
                    .log_message();
                    (action.description(), action.category())
                }
                // If the command is incomplete, we early return without clearing the buffer or logging
                // any messages
                Err(ParseErr::Incomplete) => return (self, None),
                // If the command is invalid, we report the invalid command as a log message
                Err(ParseErr::Invalid) => (
                    format!(
                        "Undefined command '{}'",
                        keystrokes_to_string(&self.keystroke_buffer)
                    ),
                    Category::Undefined,
                ),
            };

        // If we haven't returned yet, then clear the buffer
        self.keystroke_buffer.clear();
        (self, Some(log_entry))
    }

    fn keystroke_buffer(&self) -> Cow<'_, str> {
        Cow::from(keystrokes_to_string(&self.keystroke_buffer))
    }

    fn name(&self) -> &'static str {
        return self.name;
    }
}

/// The possible keystroke typed by user without any parameters.  Each `KeyStroke` can be mapped to
/// an individual [`char`].
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum CmdType {
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
    /// Command mode
    CommandMode,
}

impl CmdType {
    /// Returns a lower-case summary string of the given keystroke
    pub fn summary_string(&self) -> &'static str {
        match self {
            CmdType::Replace => "replace",
            CmdType::InsertChild => "insert child",
            CmdType::InsertBefore => "insert before",
            CmdType::InsertAfter => "insert after",
            CmdType::Delete => "delete",
            CmdType::MoveCursor(Direction::Down) => "move to first child",
            CmdType::MoveCursor(Direction::Up) => "move to parent",
            CmdType::MoveCursor(Direction::Prev) => "move to previous sibling",
            CmdType::MoveCursor(Direction::Next) => "move to next sibling",
            CmdType::Undo => "undo",
            CmdType::Redo => "redo",
            CmdType::CommandMode => "switch to command mode",
        }
    }
}

/// The [`Action`] generated by a single normal-mode 'command'.
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
    /// Switch the command mode
    CommandMode,
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
            Action::CommandMode => "switch to command mode".to_string(),
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
            Action::CommandMode => Category::CommandMode,
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

/// Attempt to parse an entire command.  This is the entry point to the parsing code.  This parser
/// is a recursive descent parser, where there is a separate function for each syntactic element
/// ([`parse_insertable`], [`parse_count`], etc.).
fn parse_command(keymap: &NormalModeKeyMap, keys: &[Key]) -> ParseResult<(usize, Action)> {
    // Generate an iterator of keystrokes, which are treated similar to tokens by the parser.
    let mut key_iter = keys.iter().copied().peekable();

    // Parse a count off the front of the command
    let count = parse_count(&mut key_iter);
    // The first non-count keystroke represents the command name.  No keystrokes is an incomplete
    // command.
    let first_key = key_iter.next().ok_or(ParseErr::Incomplete)?;

    Ok((
        count,
        match keymap.get(&first_key).ok_or(ParseErr::Invalid)? {
            CmdType::InsertChild => Action::InsertChild(parse_insertable(&mut key_iter)?),
            CmdType::InsertBefore => Action::InsertBefore(parse_insertable(&mut key_iter)?),
            CmdType::InsertAfter => Action::InsertAfter(parse_insertable(&mut key_iter)?),
            CmdType::Delete => Action::Delete,
            CmdType::Replace => Action::Replace(parse_insertable(&mut key_iter)?),
            CmdType::MoveCursor(direction) => Action::MoveCursor(*direction),
            CmdType::Undo => Action::Undo,
            CmdType::Redo => Action::Redo,
            CmdType::CommandMode => Action::CommandMode,
        },
    ))
}

/// Attempt to parse a sequence of [`Key`]strokes into an [`Insertable`].
///
/// Currently an [`Insertable`] only has one form ([`Insertable::CountedNode`]), and so this is a
/// simple matter of attempting to parse a count and then taking one char of the keystroke.
fn parse_insertable(
    keystroke_char_iter: &mut Peekable<impl Iterator<Item = Key>>,
) -> ParseResult<Insertable> {
    // Parse a count before reading the char
    let count = parse_count(keystroke_char_iter);
    // Consume the next key or return incompleteness
    let key = keystroke_char_iter.next().ok_or(ParseErr::Incomplete)?;
    // If the next keystroke is a `char`, then return it with success otherwise the command is
    // invalid
    if let Key::Char(c) = key {
        Ok(Insertable::CountedNode(count, c))
    } else {
        Err(ParseErr::Invalid)
    }
}

/// Parse a 'count' off the front of an sequence of [`Key`]strokes.  This cannot fail, because if
/// the first [`Key`] is not a numeral, this returns `1`.
fn parse_count(keystroke_char_iter: &mut Peekable<impl Iterator<Item = Key>>) -> usize {
    // accumulated_count tracks the number that is represented by the keystrokes already consumed
    // or None if no numbers have been consumed
    let mut accumulated_count: Option<usize> = None;
    loop {
        let new_digit = match keystroke_char_iter.peek() {
            Some(Key::Char('0')) => 0,
            Some(Key::Char('1')) => 1,
            Some(Key::Char('2')) => 2,
            Some(Key::Char('3')) => 3,
            Some(Key::Char('4')) => 4,
            Some(Key::Char('5')) => 5,
            Some(Key::Char('6')) => 6,
            Some(Key::Char('7')) => 7,
            Some(Key::Char('8')) => 8,
            Some(Key::Char('9')) => 9,
            _ => break,
        };
        // Pop the digit.  We use lookahead so that we leave the future keystrokes untouched for
        // the next parsing.
        keystroke_char_iter.next();
        // Since we read a new digit, we accumulate it to the count
        accumulated_count = Some(accumulated_count.map_or(new_digit, |x| x * 10 + new_digit));
    }
    accumulated_count.unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::{parse_command, Action, Insertable, ParseErr};
    use crate::config::normal_mode_default_keymap;
    use crate::core::Direction;
    use tuikit::prelude::Key;

    fn to_char_keys(string: &str) -> Vec<Key> {
        string.chars().map(|c| Key::Char(c)).collect::<Vec<_>>()
    }

    #[test]
    fn parse_single_cmd_valid() {
        let keymap = normal_mode_default_keymap();
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
            ("i0X", Action::InsertBefore(Insertable::CountedNode(0, 'X'))),
            ("ii", Action::InsertBefore(Insertable::CountedNode(1, 'i'))),
            (
                "a15x",
                Action::InsertAfter(Insertable::CountedNode(15, 'x')),
            ),
        ] {
            assert_eq!(
                parse_command(&keymap, &to_char_keys(keystrokes)),
                Ok((1, expected_effect.clone()))
            );
        }
    }

    #[test]
    fn parse_counted_command() {
        let keymap = normal_mode_default_keymap();
        for (keystrokes, exp_count, exp_action) in &[
            ("1x", 1, Action::Delete),
            ("0ra", 0, Action::Replace(Insertable::CountedNode(1, 'a'))),
            (
                "12o5p",
                12,
                Action::InsertChild(Insertable::CountedNode(5, 'p')),
            ),
        ] {
            assert_eq!(
                parse_command(&keymap, &to_char_keys(keystrokes)),
                Ok((*exp_count, exp_action.clone()))
            );
        }
    }

    #[test]
    fn parse_keystroke_invalid() {
        let keymap = normal_mode_default_keymap();
        for keystroke in &["d", "Pxx", "Qsx", "t", "Y", "X", "\""] {
            println!("Testing {}", keystroke);
            assert_eq!(
                parse_command(&keymap, &to_char_keys(keystroke)),
                Err(ParseErr::Invalid)
            );
        }
    }

    #[test]
    fn parse_keystroke_incomplete() {
        let keymap = normal_mode_default_keymap();
        for keystroke in &[
            "", "r", "o", "a", "i", "o3", "i34", "3", "1o", "0o3", "41523",
        ] {
            println!("Testing {}", keystroke);
            assert_eq!(
                parse_command(&keymap, &to_char_keys(keystroke)),
                Err(ParseErr::Incomplete)
            );
        }
    }
}
