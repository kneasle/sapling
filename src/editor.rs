//! The top-level functionality of Sapling

use crate::ast::display_token::DisplayToken;
use crate::ast::{size, Ast};
use crate::editable_tree::{Direction, EditableTree};
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use tuikit::prelude::*;

mod command_log {
    //! A utility datastructure to store and render a log of commands.  This is mostly used to give
    //! the viewers of my streams feedback for what I'm typing.

    use tuikit::prelude::*;

    /// A utility struct to store and display a log of which commands have been executed recently.
    /// This is mostly used to give the viewers of my streams feedback for what I'm typing.
    pub struct CommandLog {
        /// A list of commands that have been run
        commands: Vec<(String, String, Color)>,
    }

    impl CommandLog {
        /// Create a new (empty) command log
        pub fn new() -> CommandLog {
            CommandLog { commands: vec![] }
        }

        /// Draw a log of recent commands to a given terminal at a given location
        pub fn render(&self, term: &Term, row: usize, col: usize) {
            // Calculate the max command length to make all the columns aligned.  We can unwrap,
            // because we add '2' to the lengths, so the 'max()' function is guarunteed to be run over
            // at least one element.
            let cmd_col_width = self
                .commands
                .iter()
                .map(|(cmd, _, _)| cmd.len())
                .chain(std::iter::once(2))
                .max()
                .unwrap();
            // Render the commands
            for (i, (cmd, meaning, color)) in self.commands.iter().enumerate() {
                // Print the commands in one column
                term.print(row + i, col, cmd).unwrap();
                // Print a `=>`
                term.print(row + i, col + cmd_col_width + 1, "=>").unwrap();
                // Print the meanings in another column
                term.print_with_attr(
                    row + i,
                    col + cmd_col_width + 4,
                    meaning,
                    Attr::default().fg(*color),
                )
                .unwrap();
            }
        }

        /// Pushes a new command to the log.
        pub fn push(&mut self, command: String, keymap: &super::KeyMap) {
            let (message, color) = {
                if command.is_empty() {
                    log::error!("Empty command executed!");
                    ("<empty command>".to_string(), Color::LIGHT_RED)
                } else {
                    if let Some(action) = super::parse_command(&keymap, &command) {
                        action.meaning_and_color()
                    } else {
                        log::error!("Incomplete command executed!");
                        ("<incomplete command>".to_string(), Color::LIGHT_RED)
                    }
                }
            };
            self.commands.push((command, message, color));
        }
    }
}

/// The possible command typed by user without any parameters.
/// It can be mapped to a single key.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Command {
    /// Quit Sapling
    Quit,
    /// Replace the selected node, expects an argument
    Replace,
    /// Insert a new node, expects an argument
    InsertChild,
    /// Move cursor in given direction
    /// This is not considered a parameter as the direction is still specified by pressing specific
    /// key.
    MoveCursor(Direction),
    /// Undo the last change
    Undo,
    /// Redo a change
    Redo,
}

impl Command {
    /// Returns a lower-case summary string of the given command
    pub fn summary_string(&self) -> &'static str {
        match self {
            Command::Quit => "quit",
            Command::Replace => "replace",
            Command::InsertChild => "insert child",
            Command::MoveCursor(Direction::Down) => "move to first child",
            Command::MoveCursor(Direction::Up) => "move to parent",
            Command::MoveCursor(Direction::Prev) => "move to previous sibling",
            Command::MoveCursor(Direction::Next) => "move to next sibling",
            Command::Undo => "undo",
            Command::Redo => "redo",
        }
    }
}

/// Mapping of keys to commands.
/// Shortcut definition, also allows us to change the type if needed.
pub type KeyMap = std::collections::HashMap<char, Command>;

pub fn default_keymap() -> KeyMap {
    hmap::hmap! {
        'q' => Command::Quit,
        'o' => Command::InsertChild,
        'r' => Command::Replace,
        'c' => Command::MoveCursor(Direction::Down),
        'p' => Command::MoveCursor(Direction::Up),
        'k' => Command::MoveCursor(Direction::Prev),
        'j' => Command::MoveCursor(Direction::Next),
        'u' => Command::Undo,
        'R' => Command::Redo
    }
}

/// The possible meanings of a user-typed command
#[derive(Debug, Clone, Eq, PartialEq)]
enum Action {
    /// The user typed a command that isn't defined, but the command box should still be cleared
    Undefined,
    /// Quit Sapling
    Quit,
    /// Replace the selected node with a node represented by some [`char`]
    Replace(char),
    /// Insert a new node (given by some [`char`]) as the first child of the selected node
    InsertChild(char),
    /// Move the node in a given direction
    MoveCursor(Direction),
    /// Undo the last change
    Undo,
    /// Redo a change
    Redo,
}

impl Action {
    /// Returns a lower-case summary string of the given command, along with the color it should be
    /// displayed as
    pub fn meaning_and_color(&self) -> (String, Color) {
        const COL_MOVE: Color = Color::LIGHT_BLUE; // Color of the commands that move the cursor
        const COL_HISTORY: Color = Color::LIGHT_YELLOW; // Color of undo/redo
        const COL_INSERT: Color = Color::LIGHT_GREEN; // Colour of any insert command

        match self {
            Action::Undefined => ("undefined command".to_string(), Color::LIGHT_RED),
            Action::Quit => ("quit Sapling".to_string(), Color::LIGHT_RED),
            Action::Replace(c) => (format!("replace cursor with '{}'", c), Color::CYAN),
            Action::InsertChild(c) => (format!("insert '{}' as last child", c), COL_INSERT),
            Action::MoveCursor(Direction::Down) => ("move to first child".to_string(), COL_MOVE),
            Action::MoveCursor(Direction::Up) => ("move to parent".to_string(), COL_MOVE),
            Action::MoveCursor(Direction::Prev) => {
                ("move to previous sibling".to_string(), COL_MOVE)
            }
            Action::MoveCursor(Direction::Next) => ("move to next sibling".to_string(), COL_MOVE),
            Action::Undo => ("undo a change".to_string(), COL_HISTORY),
            Action::Redo => ("redo a change".to_string(), COL_HISTORY),
        }
    }
}

/// Attempt to convert a command as a `&`[`str`] into an [`Action`].
/// This parses the string from the start, and returns when it finds a valid command.
///
/// Therefore, `"q489flshb"` will be treated like `"q"`, and will return `Some(Action::Quit)` even
/// though `"q489flshb"` is not technically valid.
/// This function is run every time the user types a command character, and so the user would not
/// be able to input `"q489flshb"` to this function because doing so would require them to first
/// input every possible prefix of `"q489flshb"`, including `"q"`.
///
/// This returns:
/// - [`None`] if the command is incomplete.
/// - [`Action::Undefined`] if the command is not defined (like the command "X").
/// - The corresponding [`Action`], otherwise.
fn parse_command(keymap: &KeyMap, command: &str) -> Option<Action> {
    let mut command_char_iter = command.chars();

    // Consume the first char of the command
    if let Some(c) = command_char_iter.next() {
        match keymap.get(&c) {
            // "q" quits Sapling
            Some(Command::Quit) => {
                return Some(Action::Quit);
            }
            Some(Command::InsertChild) => {
                // Consume the second char of the iterator
                if let Some(insert_char) = command_char_iter.next() {
                    return Some(Action::InsertChild(insert_char));
                }
            }
            Some(Command::Replace) => {
                // Consume the second char of the iterator
                if let Some(replace_char) = command_char_iter.next() {
                    return Some(Action::Replace(replace_char));
                }
            }
            Some(Command::MoveCursor(direction)) => {
                return Some(Action::MoveCursor(*direction));
            }
            Some(Command::Undo) => {
                return Some(Action::Undo);
            }
            Some(Command::Redo) => {
                return Some(Action::Redo);
            }
            None => {
                return Some(Action::Undefined);
            }
        }
    }

    None
}

/// A struct to hold the top-level components of the editor.
pub struct Editor<'arena, Node: Ast<'arena>, E: EditableTree<'arena, Node> + 'arena> {
    /// The [`EditableTree`] that the `Editor` is editing
    tree: &'arena mut E,
    /// The style that the tree is being printed to the screen
    format_style: Node::FormatStyle,
    /// The `tuikit` terminal that the `Editor` is rendering to
    term: Term,
    /// The current contents of the command buffer
    command: String,
    /// The configured key map
    keymap: KeyMap,
    /// A list of the commands that have been executed, along with a summary of what they mean
    command_log: command_log::CommandLog,
}

impl<'arena, Node: Ast<'arena> + 'arena, E: EditableTree<'arena, Node> + 'arena>
    Editor<'arena, Node, E>
{
    /// Create a new [`Editor`] with a given tree
    pub fn new(
        tree: &'arena mut E,
        format_style: Node::FormatStyle,
        keymap: KeyMap,
    ) -> Editor<'arena, Node, E> {
        let term = Term::new().unwrap();
        Editor {
            tree,
            term,
            format_style,
            command: String::new(),
            keymap,
            command_log: command_log::CommandLog::new(),
        }
    }

    /* ===== COMMAND FUNCTIONS ===== */

    /// Replace the node under the cursor with the node represented by a given [`char`]
    fn replace_cursor(&mut self, c: char) {
        if self.tree.cursor().is_replace_char(c) {
            // We know that `c` corresponds to a valid node, so we can unwrap
            let new_node = self.tree.cursor().from_char(c).unwrap();
            log::debug!("Replacing with '{}'/{:?}", c, new_node);
            self.tree.replace_cursor(new_node);
        } else {
            log::warn!("Cannot replace node with '{}'", c);
        }
    }

    /// Move the cursor
    fn move_cursor(&mut self, direction: Direction) {
        if let Some(error_message) = self.tree.move_cursor(direction) {
            log::warn!("{}", error_message);
        }
    }

    /// Insert new child as the first child of the selected node
    fn insert_child(&mut self, c: char) {
        let cursor = self.tree.cursor();
        if cursor.is_insert_char(c) {
            if let Some(node) = cursor.from_char(c) {
                if let Err(e) = self.tree.insert_child(node) {
                    log::error!("{}", e);
                } else {
                    log::debug!("Inserting with '{}'", c);
                }
            } else {
                log::warn!("Char '{}' does not correspond to a valid node", c);
            }
        } else {
            log::warn!("Cannot insert node with '{}'", c);
        }
    }

    /// Undo the latest change
    fn undo(&mut self) {
        if self.tree.undo() {
            log::debug!("Undo successful");
        } else {
            log::warn!("No changes to undo");
        }
    }

    /// Move one change forward in the history
    fn redo(&mut self) {
        if self.tree.redo() {
            log::debug!("Redo successful");
        } else {
            log::warn!("No changes to redo");
        }
    }

    /// Render the tree to the screen
    fn render_tree(&self, row: usize, col: usize) {
        // Mutable variables to track where the terminal cursor should go
        let mut row = row;
        let mut col = col;
        let mut indentation_amount = 0;

        let cols = [
            Color::MAGENTA,
            Color::RED,
            Color::YELLOW,
            Color::GREEN,
            Color::CYAN,
            Color::BLUE,
            Color::WHITE,
            Color::LIGHT_RED,
            Color::LIGHT_BLUE,
            Color::LIGHT_CYAN,
            Color::LIGHT_GREEN,
            Color::LIGHT_YELLOW,
            Color::LIGHT_MAGENTA,
            Color::LIGHT_WHITE,
        ];

        /// A cheeky macro to print a string to the terminal
        macro_rules! term_print {
            ($string: expr) => {{
                let string = $string;
                // Print the string
                self.term.print(row, col, string).unwrap();
                // Move the cursor to the end of the string
                let size = size::Size::from(string);
                if size.lines() == 0 {
                    col += size.last_line_length();
                } else {
                    row += size.lines();
                    col = size.last_line_length();
                }
            }};
            ($string: expr, $attr: expr) => {{
                let string = $string;
                // Print the string
                self.term.print_with_attr(row, col, string, $attr).unwrap();
                // Move the cursor to the end of the string
                let size = size::Size::from(string);
                if size.lines() == 0 {
                    col += size.last_line_length();
                } else {
                    row += size.lines();
                    col = size.last_line_length();
                }
            }};
        };

        for (node, tok) in self.tree.root().display_tokens(&self.format_style) {
            match tok {
                DisplayToken::Text(s) => {
                    // Hash the ref to decide on the colour
                    let col = {
                        let mut hasher = DefaultHasher::new();
                        node.hash(&mut hasher);
                        let hash = hasher.finish();
                        cols[hash as usize % cols.len()]
                    };
                    // Generate the display attributes depending on if the node is selected
                    let attr = if std::ptr::eq(node, self.tree.cursor()) {
                        Attr::default().fg(Color::BLACK).bg(col)
                    } else {
                        Attr::default().fg(col)
                    };
                    // Print the token
                    term_print!(s.as_str(), attr);
                }
                DisplayToken::Whitespace(n) => {
                    col += n;
                }
                DisplayToken::Newline => {
                    row += 1;
                    col = indentation_amount;
                }
                DisplayToken::Indent => {
                    indentation_amount += 4;
                }
                DisplayToken::Dedent => {
                    indentation_amount -= 4;
                }
            }
        }
    }

    /* ===== MAIN FUNCTIONS ===== */

    /// Update the terminal UI display
    fn update_display(&self) {
        // Put the terminal size into some convenient variables
        let (width, height) = self.term.term_size().unwrap();
        // Clear the terminal
        self.term.clear().unwrap();

        /* RENDER MAIN TEXT VIEW */

        self.render_tree(0, 0);

        /* RENDER LOG SECTION */

        self.command_log.render(&self.term, 0, width / 2);

        /* RENDER BOTTOM BAR */

        // Add the `Press 'q' to exit.` message
        self.term
            .print(height - 1, 0, "Press 'q' to exit.")
            .unwrap();
        // Draw the current command buffer
        self.term
            .print(
                height - 1,
                width - 5 - self.command.chars().count(),
                &self.command,
            )
            .unwrap();

        /* UPDATE THE TERMINAL SCREEN */

        self.term.present().unwrap();
    }

    /// Consumes a [`char`] and adds it to the command buffer.  If the command buffer contains a
    /// valid command, then execute that command.  This returns `true` if the command 'Quit' was
    /// executed, otherwise `false` is returned.
    fn consume_command_char(&mut self, c: char) -> bool {
        let mut should_quit = false;
        // Add the new keypress to the command
        self.command.push(c);
        // Attempt to parse the command, and take action if the command is
        // complete
        if let Some(action) = parse_command(&self.keymap, &self.command) {
            // Respond to the action
            match action {
                Action::Undefined => {
                    log::warn!("'{}' is not a command.", self.command);
                }
                Action::Quit => {
                    // Break the mainloop to quit
                    log::trace!("Recieved command 'Quit', so exiting mainloop");
                    should_quit = true;
                }
                Action::MoveCursor(direction) => {
                    self.move_cursor(direction);
                }
                Action::Replace(c) => {
                    self.replace_cursor(c);
                }
                Action::InsertChild(c) => {
                    self.insert_child(c);
                }
                Action::Undo => {
                    self.undo();
                }
                Action::Redo => {
                    self.redo();
                }
            }
            // Add the command to the command log
            self.command_log.push(self.command.clone(), &self.keymap);
            // Clear the command box
            self.command.clear();
        }
        should_quit
    }

    fn mainloop(&mut self) {
        log::trace!("Starting mainloop");
        // Sit in the infinte mainloop
        while let Ok(event) = self.term.poll_event() {
            /* RESPOND TO THE USER'S INPUT */
            if let Event::Key(key) = event {
                match key {
                    Key::Char(c) => {
                        // `self.add_char_to_command` returns `true` if the editor should quit
                        if self.consume_command_char(c) {
                            break;
                        }
                    }
                    Key::ESC => {
                        self.command.clear();
                    }
                    _ => {}
                }
            }

            // Update the screen after every input (if this becomes a bottleneck then we can
            // optimise the number of calls to `update_display` but for now it's not worth the
            // added complexity)
            self.update_display();
        }
    }

    /// Start the editor and enter the mainloop
    pub fn run(mut self) {
        // Start the mainloop
        self.mainloop();
        log::trace!("Making the cursor reappear.");
        // Show the cursor before closing so that the cursor isn't permanently disabled
        // (see issue https://github.com/lotabout/tuikit/issues/28)
        self.term.show_cursor(true).unwrap();
        self.term.present().unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_command, Action};
    use crate::editable_tree::Direction;

    #[test]
    fn parse_command_complete() {
        let keymap = super::default_keymap();
        for (command, expected_effect) in &[
            ("q", Action::Quit),
            ("x", Action::Undefined),
            ("pajlbsi", Action::MoveCursor(Direction::Up)),
            ("Pxx", Action::Undefined),
            ("Qsx", Action::Undefined),
            ("ra", Action::Replace('a')),
            ("rg", Action::Replace('g')),
            ("oX", Action::InsertChild('X')),
            ("oP", Action::InsertChild('P')),
        ] {
            assert_eq!(
                parse_command(&keymap, *command),
                Some(expected_effect.clone())
            );
        }
    }

    #[test]
    fn parse_command_incomplete() {
        let keymap = super::default_keymap();
        for command in &["", "r", "o"] {
            assert_eq!(parse_command(&keymap, *command), None);
        }
    }
}
