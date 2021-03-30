//! The code for 'normal-mode', similar to that of Vim

use super::dag::{Insertable, LogMessage};
use super::{keystroke_log::Category, state, Editor};
use crate::ast::Ast;
use crate::config::KeyMap;
use crate::core::{keystrokes_to_string, Direction, Side};

use std::borrow::Cow;
use std::io::prelude::*;
use std::iter::Peekable;

use crossterm::event::{KeyCode, KeyEvent};

/// The struct covering all the [`State`](state::State)s which correspond to Sapling being in
/// normal mode.
#[derive(Debug, Clone)]
pub struct State {
    keystroke_buffer: Vec<KeyEvent>,
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
        key: KeyEvent,
        editor: &mut Editor<'arena, Node>,
    ) -> (
        Box<dyn state::State<'arena, Node>>,
        Option<(String, Category)>,
    ) {
        self.keystroke_buffer.push(key);

        let tree = &mut editor.tree;

        let log_entry = match parse_command(&editor.config.keymap, &self.keystroke_buffer) {
            // If the command buffer is a valid and complete command, then we execute the resulting
            // 'action'
            Ok((count, action)) => {
                // If the count is 0, then the command does not execute.  So we short-circuit in
                // this case
                if count == 0 {
                    return (self, Some(("no action".to_owned(), Category::Undefined)));
                }
                match action {
                    // If the command was a 'quit', then immediately make a state transition to the
                    // 'Quitted' state.  It doesn't matter what the count is, because quitting is
                    // idempotent
                    Action::Quit => {
                        return (
                            Box::new(state::Quit),
                            Some((action.description(), action.category())),
                        );
                    }
                    Action::Write => {
                        if let Some(path) = &editor.file_path {
                            // If the editor was given a file-path, then write to it
                            let mut file = std::fs::File::create(path).unwrap();
                            let mut content = tree.to_text(&editor.format_style);
                            // Force the file to finish with a newline.  BTW, <str>.chars().last()
                            // is O(1), regardless of the length of the string.
                            if content.chars().last() != Some('\n') {
                                content.push('\n');
                            }
                            file.write_all(content.as_bytes()).unwrap();
                        } else {
                            // Otherwise, log a warning and do nothing
                            log::warn!("No file to write to!");
                        }
                        // If we haven't returned yet, then clear the buffer
                        self.keystroke_buffer.clear();
                        return (self, Some((action.description(), action.category())));
                    }
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
}

/// The possible keystroke typed by user without any parameters.  Each `KeyStroke` can be mapped to
/// an individual [`char`].
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum CmdType {
    /// Quit Sapling
    Quit,
    /// Write current buffer to disk
    Write,
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

impl CmdType {
    /// Returns a lower-case summary string of the given keystroke
    pub fn summary_string(&self) -> &'static str {
        match self {
            CmdType::Quit => "quit",
            CmdType::Write => "write",
            CmdType::Replace => "replace",
            CmdType::InsertChild => "insert child",
            CmdType::InsertBefore => "insert before",
            CmdType::InsertAfter => "insert after",
            CmdType::Delete => "delete",
            CmdType::MoveCursor(Direction::Down) => "move to child",
            CmdType::MoveCursor(Direction::Up) => "move to parent",
            CmdType::MoveCursor(Direction::Prev) => "move to previous sibling",
            CmdType::MoveCursor(Direction::Next) => "move to next sibling",
            CmdType::Undo => "undo",
            CmdType::Redo => "redo",
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
    /// Quit Sapling
    Quit,
    /// Write current buffer to disk
    Write,
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
            Action::MoveCursor(Direction::Down) => "move to child".to_string(),
            Action::MoveCursor(Direction::Up) => "move to parent".to_string(),
            Action::MoveCursor(Direction::Prev) => "move to previous sibling".to_string(),
            Action::MoveCursor(Direction::Next) => "move to next sibling".to_string(),
            Action::Undo => "undo a change".to_string(),
            Action::Redo => "redo a change".to_string(),
            Action::Quit => "quit Sapling".to_string(),
            Action::Write => "write to disk".to_string(),
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
            Action::Write => Category::IO,
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
///
/// Note that this parser will return as soon as a valid command is reached.  Therefore,
/// `"q489flshb"` will be treated like `"q"`, and will return [`Action::Quit`] even though
/// `"q489flshb"` is not technically valid.  However, the command buffer is parsed every time the
/// user types a keystroke character, so the user would not be able to input `"q489flshb"` in one
/// go because doing so would require them to first input every possible prefix of `"q489flshb"`,
/// including `"q"`.
fn parse_command(keymap: &KeyMap, keys: &[KeyEvent]) -> ParseResult<(usize, Action)> {
    // Generate an iterator of keystrokes, which are treated similar to tokens by the parser.
    let mut key_iter = keys.iter().map(|ev| ev.code).peekable();

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
            // "q" quits Sapling
            CmdType::Quit => Action::Quit,
            CmdType::Write => Action::Write,
        },
    ))
}

/// Attempt to parse a sequence of [`KeyCode`]strokes into an [`Insertable`].
///
/// Currently an [`Insertable`] only has one form ([`Insertable::CountedNode`]), and so this is a
/// simple matter of attempting to parse a count and then taking one char of the keystroke.
fn parse_insertable(
    keystroke_char_iter: &mut Peekable<impl Iterator<Item = KeyCode>>,
) -> ParseResult<Insertable> {
    // Parse a count before reading the char
    let count = parse_count(keystroke_char_iter);
    // Consume the next key or return incompleteness
    let key = keystroke_char_iter.next().ok_or(ParseErr::Incomplete)?;
    // If the next keystroke is a `char`, then return it with success otherwise the command is
    // invalid
    if let KeyCode::Char(c) = key {
        Ok(Insertable::CountedNode(count, c))
    } else {
        Err(ParseErr::Invalid)
    }
}

/// Parse a 'count' off the front of an sequence of [`KeyCode`]strokes.  This cannot fail, because if
/// the first [`KeyCode`] is not a numeral, this returns `1`.
fn parse_count(keystroke_char_iter: &mut Peekable<impl Iterator<Item = KeyCode>>) -> usize {
    // accumulated_count tracks the number that is represented by the keystrokes already consumed
    // or None if no numbers have been consumed
    let mut accumulated_count: Option<usize> = None;
    loop {
        let new_digit = match keystroke_char_iter.peek() {
            Some(KeyCode::Char('0')) => 0,
            Some(KeyCode::Char('1')) => 1,
            Some(KeyCode::Char('2')) => 2,
            Some(KeyCode::Char('3')) => 3,
            Some(KeyCode::Char('4')) => 4,
            Some(KeyCode::Char('5')) => 5,
            Some(KeyCode::Char('6')) => 6,
            Some(KeyCode::Char('7')) => 7,
            Some(KeyCode::Char('8')) => 8,
            Some(KeyCode::Char('9')) => 9,
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
    use crate::config::default_keymap;
    use crate::core::Direction;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn to_char_keys(string: &str) -> Vec<KeyEvent> {
        string
            .chars()
            .map(|c| KeyEvent {
                code: KeyCode::Char(c),
                modifiers: KeyModifiers::empty(),
            })
            .collect::<Vec<_>>()
    }

    #[test]
    fn parse_single_cmd_valid() {
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
            ("i0X", Action::InsertBefore(Insertable::CountedNode(0, 'X'))),
            ("ii", Action::InsertBefore(Insertable::CountedNode(1, 'i'))),
            (
                "a15x",
                Action::InsertAfter(Insertable::CountedNode(15, 'x')),
            ),
            ("q", Action::Quit),
        ] {
            assert_eq!(
                parse_command(&keymap, &to_char_keys(keystrokes)),
                Ok((1, expected_effect.clone()))
            );
        }
    }

    #[test]
    fn parse_counted_command() {
        let keymap = default_keymap();
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
        let keymap = default_keymap();
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
        let keymap = default_keymap();
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
