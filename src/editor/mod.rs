//! The top-level functionality of Sapling

pub mod normal_mode;

use crate::ast::display_token::DisplayToken;
use crate::ast::{size, Ast};
use crate::editable_tree::{EditErr, EditResult, EditSuccess, LogMessage, Side, DAG};
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use tuikit::prelude::*;
use normal_mode::{KeyMap, command_log, Action, parse_command};

/// A struct to hold the top-level components of the editor.
pub struct Editor<'arena, Node: Ast<'arena>> {
    /// The [`EditableTree`] that the `Editor` is editing
    tree: &'arena mut DAG<'arena, Node>,
    /// The style that the tree is being printed to the screen
    format_style: Node::FormatStyle,
    /// The `tuikit` terminal that the `Editor` is rendering to
    term: Term,
    /// The current contents of the command buffer
    command: String,
    /// The configured key map
    keymap: KeyMap,
    /// A list of the commands that have been executed, along with a summary of what they mean
    command_log: command_log::KeyStrokeLog,
}

impl<'arena, Node: Ast<'arena> + 'arena> Editor<'arena, Node> {
    /// Create a new [`Editor`] with a given tree
    pub fn new(
        tree: &'arena mut DAG<'arena, Node>,
        format_style: Node::FormatStyle,
        keymap: KeyMap,
    ) -> Editor<'arena, Node> {
        let term = Term::new().unwrap();
        Editor {
            tree,
            term,
            format_style,
            command: String::new(),
            keymap,
            command_log: command_log::KeyStrokeLog::new(10),
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

    /// Execute an [`Action`] generated by user's keystrokes.  Returns `true` if the user executed
    /// [`Action::Quit`], false otherwise
    fn execute_action(&mut self, action: Action) -> (bool, EditResult) {
        let mut should_quit = false;
        // Respond to the action
        let result = match action {
            // Undefined command
            Action::Undefined => Err(EditErr::Invalid(self.command.clone())),
            // History commands
            Action::Undo => self.tree.undo(),
            Action::Redo => self.tree.redo(),
            // Move command
            Action::MoveCursor(direction) => self.tree.move_cursor(direction),
            // Edit commands
            Action::Replace(c) => self.tree.replace_cursor(c),
            Action::InsertChild(c) => self.tree.insert_child(c),
            Action::InsertBefore(c) => self.tree.insert_next_to_cursor(c, Side::Prev),
            Action::InsertAfter(c) => self.tree.insert_next_to_cursor(c, Side::Next),
            Action::Delete => self.tree.delete_cursor(),
            // Quit Sapling
            Action::Quit => {
                should_quit = true;
                Ok(EditSuccess::Quit)
            }
        };
        (should_quit, result)
    }

    /// Consumes a [`char`] and adds it to the command buffer.  If the command buffer contains a
    /// valid command, then execute that command.
    ///
    /// This returns a tuple of:
    /// 1. A [`bool`] value that determines whether or not Sapling should quit
    /// 2. The [`EditResult`] of the edit, or `None` if the command is incomplete
    fn consume_command_char(&mut self, c: char) -> (bool, Option<EditResult>) {
        // Add the new keypress to the command
        self.command.push(c);
        // Attempt to parse the command, and take action if the command is
        // complete
        match parse_command(&self.keymap, &self.command) {
            Some(action) => {
                let (should_quit, result) = self.execute_action(action);
                // Add the command to the command log and clear the command buffer
                self.command_log.push(self.command.clone(), &self.keymap);
                self.command.clear();
                // Return the result of the action
                (should_quit, Some(result))
            }
            None => (false, None),
        }
    }

    fn mainloop(&mut self) {
        log::trace!("Starting mainloop");
        // Sit in the infinte mainloop
        while let Ok(event) = self.term.poll_event() {
            /* RESPOND TO THE USER'S INPUT */
            if let Event::Key(key) = event {
                match key {
                    Key::Char(c) => {
                        // Consume the new keystroke
                        let (should_quit, result) = self.consume_command_char(c);
                        // Write the result's message to the log if the command was complete
                        if let Some(res) = result {
                            res.log_message();
                        }
                        // `self.add_char_to_command` returns `true` if the editor should quit
                        if should_quit {
                            break;
                        }
                    }
                    Key::ESC => {
                        self.command.clear();
                    }
                    _ => {}
                }
            }

            // Make sure that the logger isn't taller than the screen
            self.command_log
                .set_max_entries(self.term.term_size().unwrap().1.min(10));

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
    use super::normal_mode::{default_keymap, Action, parse_command};
    use crate::editable_tree::Direction;

    #[test]
    fn parse_command_complete() {
        let keymap = default_keymap();
        for (command, expected_effect) in &[
            ("q", Action::Quit),
            ("x", Action::Delete),
            ("d", Action::Undefined),
            ("h", Action::MoveCursor(Direction::Prev)),
            ("j", Action::MoveCursor(Direction::Next)),
            ("k", Action::MoveCursor(Direction::Prev)),
            ("l", Action::MoveCursor(Direction::Next)),
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
        let keymap = super::normal_mode::default_keymap();
        for command in &["", "r", "o"] {
            assert_eq!(parse_command(&keymap, *command), None);
        }
    }
}