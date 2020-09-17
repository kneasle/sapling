use crate::ast::AST;
use tuikit::prelude::*;

/// A struct to hold the top-level components of the editor.
pub struct Editor<T: AST> {
    ast: T,
    format_style: T::FormatStyle,
    term: Term,
    command: String,
}

impl<T: AST> Editor<T> {
    /// Create a new [Editor] to edit a given `ast`.
    pub fn new(ast: T, style: T::FormatStyle) -> Editor<T> {
        let term = Term::new().unwrap();
        Editor {
            ast,
            format_style: style,
            term,
            command: String::new(),
        }
    }

    pub fn mainloop(mut self) {
        while let Ok(event) = self.term.poll_event() {
            // Put the terminal size into some convenient variables
            let (width, height) = self.term.term_size().unwrap();

            /* RESPOND TO THE USER'S INPUT */
            
            // Close the editor if the user presses 'q'
            if let Event::Key(key) = event {
                match key {
                    // If the user types 'q' then quit the program
                    Key::Char('q') => {
                        break;
                    }
                    Key::Char(c) => {
                        if self.command == "r" {
                            if let Some(node) = self.ast.from_replace_char(c) {
                                self.ast = node;
                            }
                            self.command.clear();
                        } else {
                            self.command.push(c);
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
                .print(0, 0, &self.ast.to_text(&self.format_style))
                .unwrap();
            // Render the bottom bar of the editor
            self.term
                .print(height - 1, 0, "Press 'q' to exit.")
                .unwrap();
            self.term
                .print(height - 1, width - 5 - self.command.len(), &self.command)
                .unwrap();
            // Update the terminal screen
            self.term.present().unwrap();
        }
    }
}
