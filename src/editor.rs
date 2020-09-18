use crate::ast_spec::{ASTSpec, Reference};
use crate::editable_tree::EditableTree;
use tuikit::prelude::*;

use std::marker::PhantomData;

/// A struct to hold the top-level components of the editor.
pub struct Editor<R: Reference, T: ASTSpec<R>, E: EditableTree<R, T>> {
    tree: E,
    format_style: T::FormatStyle,
    term: Term,
    command: String,
    phantom_ref: PhantomData<R>,
}

impl<R: Reference, T: ASTSpec<R>, E: EditableTree<R, T>> Editor<R, T, E> {
    /// Create a new [Editor] to edit a given `ast`.
    pub fn new(tree: E, format_style: T::FormatStyle) -> Editor<R, T, E> {
        let term = Term::new().unwrap();
        Editor {
            tree,
            term,
            format_style,
            command: String::new(),
            phantom_ref: PhantomData::default(),
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
                        self.command.push(c);
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
                .print(height - 1, width - 5 - self.command.len(), &self.command)
                .unwrap();
            // Update the terminal screen
            self.term.present().unwrap();
        }
    }
}
