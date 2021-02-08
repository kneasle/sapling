//! The top-level functionality of Sapling

pub mod command_mode;
pub mod dag;
pub mod keystroke_log;
pub mod normal_mode;
pub mod state;

use crate::ast::display_token::{DisplayToken, SyntaxCategory};
use crate::ast::Ast;
use crate::config::{Config, DEBUG_HIGHLIGHTING};
use crate::core::Size;

use dag::Dag;
use keystroke_log::KeyStrokeLog;
use state::State;

use std::borrow::{Borrow, Cow};
use std::collections::{hash_map::DefaultHasher, HashSet};
use std::hash::Hasher;
use std::path::PathBuf;

use tuikit::prelude::*;

/// The [`State`] that Sapling is in during a transition function.  This has to exist, but
/// none of the methods should ever be called, since doing so would require the transition function
/// to unexpectedly fail, which is not possible (since the transition function must return a new
/// [`State`] or `panic`, in which case execution stops and the `IntermediateState` is never used).
/// This is a zero-sized type, so constructing a `Box<IntermediateState as State>` does not perform
/// any heap allocations.
#[derive(Debug, Copy, Clone)]
struct IntermediateState;

impl<'arena, Node: Ast<'arena>> State<'arena, Node> for IntermediateState {
    fn transition(
        self: Box<Self>,
        _key: Key,
        _tree: &mut Editor<'arena, Node>,
    ) -> (
        Box<dyn State<'arena, Node>>,
        Option<(String, keystroke_log::Category)>,
    ) {
        panic!("Invalid state should never exist except during state transitions.");
    }

    fn is_quit(&self) -> bool {
        panic!("Invalid state should never exist except during state transitions.");
    }

    fn keystroke_buffer(&self) -> Cow<'_, str> {
        panic!("Invalid state should never exist except during state transitions.");
    }

    fn name(&self) -> &'static str {
        return "";
    }
}

/// A singleton struct to hold the top-level components of Sapling.
pub struct Editor<'arena, Node: Ast<'arena>> {
    /// The `Dag` that is storing the history of the `Editor`
    tree: &'arena mut Dag<'arena, Node>,
    /// The style that the tree is being printed to the screen
    format_style: Node::FormatStyle,
    /// The `tuikit` terminal that the `Editor` is rendering to
    term: Term,
    /// The current state-machine [`State`] that Sapling is in
    state: Box<dyn State<'arena, Node>>,
    /// The current user configuration
    config: Config,
    /// A list of the keystrokes that have been executed, along with a summary of what they mean
    keystroke_log: KeyStrokeLog,
    file_path: Option<PathBuf>,
}

impl<'arena, Node: Ast<'arena> + 'arena> Editor<'arena, Node> {
    /// Create a new [`Editor`] with a given tree
    pub fn new(
        tree: &'arena mut Dag<'arena, Node>,
        format_style: Node::FormatStyle,
        config: Config,
        file_path: Option<PathBuf>,
    ) -> Editor<'arena, Node> {
        let term = Term::new().unwrap();
        Editor {
            tree,
            term,
            format_style,
            state: Box::new(normal_mode::State::default()),
            config,
            keystroke_log: KeyStrokeLog::new(10),
            file_path,
        }
    }

    /// Render the tree to the screen
    fn render_tree(&self, row: usize, col: usize) {
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

        // Mutable variables to track where the terminal cursor should go
        let mut row = row;
        let mut col = col;
        let mut indentation_amount = 0;

        let mut unknown_categories: HashSet<SyntaxCategory> = HashSet::with_capacity(0);

        /// A cheeky macro to print a string to the terminal
        macro_rules! term_print {
            ($string: expr) => {{
                let string = $string;
                // Print the string
                self.term.print(row, col, string).unwrap();
                // Move the cursor to the end of the string
                let size = Size::from(string);
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
                let size = Size::from(string);
                if size.lines() == 0 {
                    col += size.last_line_length();
                } else {
                    row += size.lines();
                    col = size.last_line_length();
                }
            }};
        }

        for (node, tok) in self.tree.root().display_tokens(&self.format_style) {
            match tok {
                DisplayToken::Text(s, category) => {
                    let col = if DEBUG_HIGHLIGHTING {
                        // Hash the ref to decide on the colour
                        let mut hasher = DefaultHasher::new();
                        node.hash(&mut hasher);
                        let hash = hasher.finish();
                        cols[hash as usize % cols.len()]
                    } else {
                        *self.config.color_scheme.get(category).unwrap_or_else(|| {
                            unknown_categories.insert(category);
                            &Color::LIGHT_MAGENTA
                        })
                    };
                    // Generate the display attributes depending on if the node is selected
                    let attr = if std::ptr::eq(node, self.tree.cursor()) {
                        Attr::default().fg(Color::BLACK).bg(col)
                    } else {
                        Attr::default().fg(col)
                    };
                    // Print the token
                    term_print!(s.borrow(), attr);
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

        // Print warning messages for unknown syntax categories
        for c in unknown_categories {
            log::error!("Unknown highlight category '{}'", c);
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
        self.term.print(height - 1, 0, &self.state.name()).unwrap();
        // Draw the current keystroke buffer
        let keystroke_buffer = self.state.keystroke_buffer();
        self.term
            .print(
                height - 1,
                width - 5 - keystroke_buffer.chars().count(),
                keystroke_buffer.borrow(),
            )
            .unwrap();

        /* UPDATE THE TERMINAL SCREEN */

        self.term.present().unwrap();
    }

    fn mainloop(&mut self) {
        log::trace!("Starting mainloop");
        // Sit in the infinte mainloop
        while let Ok(event) = self.term.poll_event() {
            /* RESPOND TO THE USER'S INPUT */
            if let Event::Key(key) = event {
                // Consume the key and use it to move through the state machine.  Here, we use
                // `std::mem::replace` to allow us to move `self.state` into `State::transition` by
                // replacing it with the temporary value of `Box::new(IntermediateState)`.
                //
                // `Box::new(IntermediateState)` is creating a `Box` of a zero-size type, which
                // according to the docs
                // (https://doc.rust-lang.org/std/boxed/struct.Box.html#method.new) does not
                // perform a heap allocation.
                let (new_state, log_entry) = State::transition(
                    std::mem::replace(
                        &mut self.state,
                        Box::new(IntermediateState) as Box<dyn State<'arena, Node>>,
                    ),
                    key,
                    self,
                );

                self.state = new_state;

                // Log the key to the keystroke log, and create a log message if required
                self.keystroke_log.push_key(key);
                if let Some((description, category)) = log_entry {
                    self.keystroke_log.log_entry(description, category);
                }
            }

            // If we have reached `state::Quit` then we should exit the main loop
            if self.state.is_quit() {
                break;
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
        // Start the mainloop, which will not exit until Sapling is ready to close
        self.mainloop();
        // Show the cursor before closing so that the cursor isn't permanently disabled
        // (see issue `lotabout/tuikit#28`: https://github.com/lotabout/tuikit/issues/28)
        log::trace!("Making the cursor reappear.");
        self.term.show_cursor(true).unwrap();
        self.term.present().unwrap();
    }
}
