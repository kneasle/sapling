use crate::ast_spec::{ASTSpec, Reference};
use crate::editable_tree::EditableTree;
use tuikit::prelude::*;

/// The possible outcomes of a user-typed command
#[derive(Debug, Clone, Eq, PartialEq)]
enum Action {
    /// The user typed a command that isn't defined, but the command box should still be cleared
    Undefined,
    /// Quit Sapling
    Quit,
    /// Replace the currently selected node with a node represented by some [char]
    Replace(char),
}

/// Attempt to convert a command as a [str] into an [Action].
/// This parses the string from the start, and returns when it finds a valid command.
/// Therefore, `"q489flshb"` will be treated like `"q"`, and will return `Some(Action::Quit)`.
/// This returns:
/// - [None] if the command is incomplete.
/// - [Action::Undefined] if the command is not defined (like the command "X").
/// - The corresponding [Action], otherwise.
fn interpret_command(command: &str) -> Option<Action> {
    let mut command_char_iter = command.chars();

    // Consume the first char of the command
    if let Some(c) = command_char_iter.next() {
        match c {
            // "q" quits Sapling
            'q' => {
                return Some(Action::Quit);
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
    format_style: T::FormatStyle,
    term: Term,
    command: String,
}

impl<R: Reference, T: ASTSpec<R>, E: EditableTree<R, T>> Editor<R, T, E> {
    /// Create a new [Editor] with the default AST.
    pub fn new(tree: E, format_style: T::FormatStyle) -> Editor<R, T, E> {
        let term = Term::new().unwrap();
        Editor {
            tree,
            term,
            format_style,
            command: String::new(),
        }
    }

    pub fn mainloop(mut self) {
        while let Ok(event) = self.term.poll_event() {
            // Put the terminal size into some convenient variables
            let (width, height) = self.term.term_size().unwrap();

            /* RESPOND TO THE USER'S INPUT */

            if let Event::Key(key) = event {
                match key {
                    Key::Char(c) => {
                        self.command.push(c);
                        if let Some(action) = interpret_command(&self.command) {
                            // Clear the command box
                            self.command.clear();
                            // Respond to the command
                            match action {
                                Action::Undefined => {}
                                Action::Quit => {
                                    break;
                                }
                                Action::Replace(_c) => {}
                            }
                        }
                    }
                    Key::ESC => {
                        self.command.clear();
                    }
                    _ => {}
                }
            }

            /* RENDER THE EDITOR UI */

            self.term.clear().unwrap();
            // Print the AST to the terminal
            self.term
                .print(0, 0, &self.tree.to_text(&self.format_style))
                .unwrap();
            // Render the bottom bar of the editor
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
    }
}

#[cfg(test)]
mod tests {
    use super::{interpret_command, Action};

    #[test]
    fn interpret_command_complete() {
        for (command, expected_effect) in &[
            ("q", Action::Quit),
            ("x", Action::Undefined),
            ("pajlbsi", Action::Undefined),
            ("ra", Action::Replace('a')),
            ("rg", Action::Replace('g')),
        ] {
            assert_eq!(interpret_command(*command), Some(expected_effect.clone()));
        }
    }

    #[test]
    fn interpret_command_incomplete() {
        for command in &["r"] {
            assert_eq!(interpret_command(*command), None);
        }
    }
}
