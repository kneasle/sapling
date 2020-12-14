//! The top-level functionality of Sapling

pub mod normal_mode;

use crate::ast::display_token::DisplayToken;
use crate::ast::{size, Ast};
use crate::editable_tree::{EditResult, LogMessage, DAG};
use normal_mode::{keystroke_log, parse_keystroke, KeyMap};
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use tuikit::prelude::*;

/// A struct to hold the top-level components of the editor.
pub struct Editor<'arena, Node: Ast<'arena>> {
    /// The [`EditableTree`] that the `Editor` is editing
    tree: &'arena mut DAG<'arena, Node>,
    /// The style that the tree is being printed to the screen
    format_style: Node::FormatStyle,
    /// The `tuikit` terminal that the `Editor` is rendering to
    term: Term,
    /// The current contents of the keystroke buffer
    keystroke: String,
    /// The configured key map
    keymap: KeyMap,
    /// A list of the keystrokes that have been executed, along with a summary of what they mean
    keystroke_log: keystroke_log::KeyStrokeLog,
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
            keystroke: String::new(),
            keymap,
            keystroke_log: keystroke_log::KeyStrokeLog::new(10),
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

        self.keystroke_log.render(&self.term, 0, width / 2);

        /* RENDER BOTTOM BAR */

        // Add the `Press 'q' to exit.` message
        self.term
            .print(height - 1, 0, "Press 'q' to exit.")
            .unwrap();
        // Draw the current keystroke buffer
        self.term
            .print(
                height - 1,
                width - 5 - self.keystroke.chars().count(),
                &self.keystroke,
            )
            .unwrap();

        /* UPDATE THE TERMINAL SCREEN */

        self.term.present().unwrap();
    }

    /// Consumes a [`char`] and adds it to the keystroke buffer.  If the keystroke buffer contains a
    /// valid keystroke, then execute that keystroke.
    ///
    /// This returns a tuple of:
    /// 1. A [`bool`] value that determines whether or not Sapling should quit
    /// 2. The [`EditResult`] of the edit, or `None` if the keystroke is incomplete
    fn consume_keystroke(&mut self, c: char) -> (bool, Option<EditResult>) {
        // Add the new keypress to the keystroke
        self.keystroke.push(c);
        // Attempt to parse the keystroke, and take action if the keystroke is
        // complete
        match parse_keystroke(&self.keymap, &self.keystroke) {
            Some(action) => {
                let (should_quit, result) = self.tree.execute_action(action);
                // Add the keystroke to the keystroke log and clear the keystroke buffer
                self.keystroke_log
                    .push(self.keystroke.clone(), &self.keymap);
                self.keystroke.clear();
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
                        let (should_quit, result) = self.consume_keystroke(c);
                        // Write the result's message to the log if the keystroke was complete
                        if let Some(res) = result {
                            res.log_message();
                        }
                        // `self.add_char_to_keystroke` returns `true` if the editor should quit
                        if should_quit {
                            break;
                        }
                    }
                    Key::ESC => {
                        self.keystroke.clear();
                    }
                    _ => {}
                }
            }

            // Make sure that the logger isn't taller than the screen
            self.keystroke_log
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
    use super::normal_mode::{default_keymap, parse_keystroke, Action};
    use crate::editable_tree::Direction;

    #[test]
    fn parse_keystroke_complete() {
        let keymap = default_keymap();
        for (keystroke, expected_effect) in &[
            ("q", Action::Quit),
            ("x", Action::Delete),
            ("d", Action::Undefined("d".to_string())),
            ("h", Action::MoveCursor(Direction::Prev)),
            ("j", Action::MoveCursor(Direction::Next)),
            ("k", Action::MoveCursor(Direction::Prev)),
            ("l", Action::MoveCursor(Direction::Next)),
            ("pajlbsi", Action::MoveCursor(Direction::Up)),
            ("Pxx", Action::Undefined("Pxx".to_string())),
            ("Qsx", Action::Undefined("Qsx".to_string())),
            ("ra", Action::Replace('a')),
            ("rg", Action::Replace('g')),
            ("oX", Action::InsertChild('X')),
            ("oP", Action::InsertChild('P')),
        ] {
            assert_eq!(
                parse_keystroke(&keymap, *keystroke),
                Some(expected_effect.clone())
            );
        }
    }

    #[test]
    fn parse_keystroke_incomplete() {
        let keymap = super::normal_mode::default_keymap();
        for keystroke in &["", "r", "o"] {
            assert_eq!(parse_keystroke(&keymap, *keystroke), None);
        }
    }
}
