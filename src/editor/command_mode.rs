//! The code for 'normal-mode', similar to that of Vim

use super::{keystroke_log::Category, normal_mode, state, Editor};
use crate::ast::Ast;
use crate::config::CommandModeKeyMap;
use crate::core::keystrokes_to_string;

use std::borrow::Cow;
use std::io::prelude::*;
use std::iter::Peekable;

use tuikit::prelude::Key;

/// The struct covering all the [`State`](state::State)s which correspond to Sapling being in
/// command mode.
#[derive(Debug, Clone)]
pub struct State {
    name: &'static str,
    keystroke_buffer: Vec<Key>,
}

impl Default for State {
    fn default() -> Self {
        State {
            name: "COMMAND",
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
            match parse_command(&editor.config.command_mode_keymap, &self.keystroke_buffer) {
                Ok((_, action)) => {
                    match action {
                        // If the command was a 'quit', then immediately make a state transition to the
                        // 'Quitted' state.  It doesn't matter what the count is, because quitting is
                        // idempotent
                        Action::Quit => {
                            return (
                                Box::new(Quit),
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
                            return (
                                Box::new(normal_mode::State::default()),
                                Some((action.description(), action.category())),
                            );
                        }
                        Action::NormalMode => {
                            return (
                                Box::new(normal_mode::State::default()),
                                Some((action.description(), action.category())),
                            );
                        }
                        Action::DotGraph => {
                            //TODO write dot file to a different file
                            log::debug!("{}", tree.to_dot_code());
                            return (
                                Box::new(normal_mode::State::default()),
                                Some((action.description(), action.category())),
                            );
                        }
                    }
                }

                Err(ParseErr::Incomplete) => return (self, None),
                Err(ParseErr::Invalid) => (
                    format!(
                        "Undefined command '{}'",
                        keystrokes_to_string(&self.keystroke_buffer)
                    ),
                    Category::Undefined,
                ),
            };

        self.keystroke_buffer.clear();
        (self, Some(log_entry))
    }

    fn keystroke_buffer(&self) -> Cow<'_, str> {
        Cow::from(keystrokes_to_string(&self.keystroke_buffer))
    }

    fn name(&self) -> &'arena str {
        return &self.name;
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
    /// Quit command mode
    NormalMode,
    /// To dot graph
    DotGraph,
}

impl CmdType {
    /// Returns a lower-case summary string of the given keystroke
    pub fn summary_string(&self) -> &'static str {
        match self {
            CmdType::Quit => "quit",
            CmdType::Write => "write",
            CmdType::NormalMode => "switch to normal mode",
            CmdType::DotGraph => "write to dot graph",
        }
    }
}

/// The [`Action`] generated by a single command-mode 'command'.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum Action {
    /// Quit Sapling
    Quit,
    /// Write current buffer to disk
    Write,
    /// Write dot graph to disk
    DotGraph,
    /// Write current buffer to disk
    NormalMode,
}

impl Action {
    /// Returns a lower-case summary of the given keystroke, along with the color with which it
    /// should be displayed in the log.
    pub fn description(&self) -> String {
        match self {
            Action::Quit => "quit Sapling".to_string(),
            Action::Write => "write to disk".to_string(),
            Action::DotGraph => "write to .dot".to_string(),
            Action::NormalMode => "switch to normal mode".to_string(),
        }
    }

    /// Returns the [`Category`] of this `Action`
    pub fn category(&self) -> Category {
        match self {
            Action::Quit => Category::Quit,
            Action::Write => Category::IO,
            Action::DotGraph => Category::IO,
            Action::NormalMode => Category::NormalMode,
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
/// is a recursive descent parser, where there is a separate function for [`parse_count`] for count.
///
/// Note that this parser will return as soon as a valid command is reached.  Therefore,
/// `"q489flshb"` will be treated like `"q"`, and will return [`Action::Quit`] even though
/// `"q489flshb"` is not technically valid.  However, the command buffer is parsed every time the
/// user types a keystroke character, so the user would not be able to input `"q489flshb"` in one
/// go because doing so would require them to first input every possible prefix of `"q489flshb"`,
/// including `"q"`.
fn parse_command(keymap: &CommandModeKeyMap, keys: &[Key]) -> ParseResult<(usize, Action)> {
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
            // "q" quits Sapling
            CmdType::Quit => Action::Quit,
            CmdType::Write => Action::Write,
            CmdType::DotGraph => Action::DotGraph,
            CmdType::NormalMode => Action::NormalMode,
        },
    ))
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

/// The [`State`] that Sapling enters to quit the mainloop and exit
#[derive(Debug, Copy, Clone)]
struct Quit;

impl<'arena, Node: Ast<'arena>> state::State<'arena, Node> for Quit {
    fn transition(
        self: Box<Self>,
        _key: Key,
        _editor: &mut Editor<'arena, Node>,
    ) -> (
        Box<dyn state::State<'arena, Node>>,
        Option<(String, Category)>,
    ) {
        (self, None)
    }

    fn is_quit(&self) -> bool {
        true
    }
}
