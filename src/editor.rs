use crate::ast_spec::{ASTSpec, Reference};
use crate::editable_tree::EditableTree;
use tuikit::prelude::*;

/// The possible log levels
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum LogLevel {
    /// Logs that give lots of minute details.  Intended to be used only when debugging Sapling.
    VerboseDebug = 0,
    /// Less verbose debugging, intended to be used only when debugging Sapling.
    Debug = 1,
    /// Logs that just give information to the user
    Info = 2,
    /// Logs that represent warnings about an Error that is either going to happen or might have
    /// already happened.
    Warning = 3,
    /// Logs that will always be logged.
    Error = 4,
}

impl LogLevel {
    /// Returns a [`Color`] with which to display this log entry
    pub fn to_color(&self) -> Color {
        match self {
            LogLevel::VerboseDebug => Color::BLUE,
            LogLevel::Debug => Color::LIGHT_BLUE,
            LogLevel::Info => Color::GREEN,
            LogLevel::Warning => Color::YELLOW,
            LogLevel::Error => Color::RED,
        }
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
}

/// Attempt to convert a command as a `&`[`str`] into an [`Action`].
/// This parses the string from the start, and returns when it finds a valid command.
/// Therefore, `"q489flshb"` will be treated like `"q"`, and will return `Some(Action::Quit)`.
/// This returns:
/// - [`None`] if the command is incomplete.
/// - [`Action::Undefined`] if the command is not defined (like the command "X").
/// - The corresponding [`Action`], otherwise.
fn parse_command(command: &str) -> Option<Action> {
    let mut command_char_iter = command.chars();

    // Consume the first char of the command
    if let Some(c) = command_char_iter.next() {
        match c {
            // "q" quits Sapling
            'q' => {
                return Some(Action::Quit);
            }
            'i' => {
                // Consume the second char of the iterator
                if let Some(insert_char) = command_char_iter.next() {
                    return Some(Action::InsertChild(insert_char));
                }
            }
            'r' => {
                // Consume the second char of the iterator
                if let Some(replace_char) = command_char_iter.next() {
                    return Some(Action::Replace(replace_char));
                }
            }
            _ => {
                return Some(Action::Undefined);
            }
        }
    }

    None
}

/// A struct to hold the top-level components of the editor.
pub struct Editor<R: Reference, T: ASTSpec<R>, E: EditableTree<R, T>> {
    tree: E,
    log: Vec<(LogLevel, String)>,
    format_style: T::FormatStyle,
    term: Term,
    command: String,
}

impl<R: Reference, T: ASTSpec<R>, E: EditableTree<R, T>> Editor<R, T, E> {
    /// Create a new [`Editor`] with the default AST.
    pub fn new(tree: E, format_style: T::FormatStyle) -> Editor<R, T, E> {
        let term = Term::new().unwrap();
        Editor {
            tree,
            log: Vec::new(),
            term,
            format_style,
            command: String::new(),
        }
    }

    /// Log a message to whatever console is appropriate
    fn log(&mut self, level: LogLevel, message: String) {
        self.log.push((level, message));
    }

    /* ===== COMMAND FUNCTIONS ===== */

    /// Replace the node under the cursor with the node represented by a given [`char`]
    fn replace_cursor(&mut self, c: char) {
        if let Some(new_node) = self.tree.cursor_node().from_replace_char(c) {
            self.log(
                LogLevel::Debug,
                format!("Replacing with '{}'/{:?}", c, new_node),
            );
            self.tree.replace_cursor(new_node);
        } else {
            self.log(
                LogLevel::Warning,
                format!("Cannot replace node with '{}'", c),
            );
        }
    }

    /// Insert new child as the first child of the selected node
    fn insert_child(&mut self, c: char) {
        if self.tree.cursor_node().insert_chars().any(|x| x == c) {
            self.log(LogLevel::Debug, format!("Inserting with '{}'", c));
        } else {
            self.log(
                LogLevel::Warning,
                format!("Cannot replace node with '{}'", c),
            );
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
        let text = self.tree.to_text(&self.format_style);
        for (i, line) in text.lines().enumerate() {
            self.term.print(i, 0, line).unwrap();
        }

        /* RENDER LOG SECTION */
        for (i, (level, message)) in self.log.iter().enumerate() {
            self.term
                .print_with_attr(i, width / 2, message, Attr::default().fg(level.to_color()))
                .unwrap();
        }

        /* RENDER BOTTOM BAR */
        self.term
            .print(height - 1, 0, "Press 'q' to exit.")
            .unwrap();
        self.term
            .print(
                height - 1,
                width - 5 - self.command.chars().count(),
                &self.command,
            )
            .unwrap();

        // Update the terminal screen
        self.term.present().unwrap();
    }

    fn mainloop(&mut self) {
        // Sit in the infinte mainloop
        while let Ok(event) = self.term.poll_event() {
            /* RESPOND TO THE USER'S INPUT */
            if let Event::Key(key) = event {
                match key {
                    Key::Char(c) => {
                        // Add the new keypress to the command
                        self.command.push(c);
                        // Attempt to parse the command, and take action if the command is
                        // complete
                        if let Some(action) = parse_command(&self.command) {
                            // Respond to the action
                            match action {
                                Action::Undefined => {
                                    self.log(
                                        LogLevel::Warning,
                                        format!("'{}' not a command.", self.command),
                                    );
                                }
                                Action::Quit => {
                                    // Break the mainloop to quit
                                    break;
                                }
                                Action::Replace(c) => {
                                    self.replace_cursor(c);
                                }
                                Action::InsertChild(c) => {
                                    self.insert_child(c);
                                }
                            }
                            // Clear the command box
                            self.command.clear();
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
        // Log the startup of the code
        self.log(LogLevel::Info, "Starting Up...".to_string());
        // Start the mainloop
        self.mainloop();
        // Log that the editor is closing
        self.log(LogLevel::Info, "Closing...".to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_command, Action};

    #[test]
    fn parse_command_complete() {
        for (command, expected_effect) in &[
            ("q", Action::Quit),
            ("x", Action::Undefined),
            ("pajlbsi", Action::Undefined),
            ("Pxx", Action::Undefined),
            ("Qsx", Action::Undefined),
            ("ra", Action::Replace('a')),
            ("rg", Action::Replace('g')),
            ("iX", Action::InsertChild('X')),
            ("iP", Action::InsertChild('P')),
        ] {
            assert_eq!(parse_command(*command), Some(expected_effect.clone()));
        }
    }

    #[test]
    fn parse_command_incomplete() {
        for command in &["", "r", "i"] {
            assert_eq!(parse_command(*command), None);
        }
    }
}
