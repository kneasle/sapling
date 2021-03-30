//! The top-level functionality of Sapling

pub mod dag;
pub mod keystroke_log;
pub mod normal_mode;
pub mod state;
mod widgets;

use crate::ast::Ast;
use crate::config::{Config, DEBUG_HIGHLIGHTING};

use dag::Dag;
use keystroke_log::KeyStrokeLog;
use state::State;

use std::borrow::Cow;
use std::io;
use std::path::PathBuf;

use crossterm::{
    cursor,
    event::{Event, KeyEvent},
    terminal,
};
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
};

pub(crate) type Terminal = tui::Terminal<CrosstermBackend<io::Stdout>>;

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
        _key: KeyEvent,
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
}

/// A singleton struct to hold the top-level components of Sapling.
pub struct Editor<'arena, Node: Ast<'arena>> {
    /// The `Dag` that is storing the history of the `Editor`
    tree: &'arena mut Dag<'arena, Node>,
    /// The style that the tree is being printed to the screen
    format_style: Node::FormatStyle,
    /// The `tuikit` terminal that the `Editor` is rendering to
    term: Terminal,
    /// The current state-machine [`State`] that Sapling is in
    state: Box<dyn State<'arena, Node>>,
    /// The current user configuration
    config: Config,
    /// A list of the keystrokes that have been executed, along with a summary of what they mean
    keystroke_log: KeyStrokeLog,
    file_path: Option<PathBuf>,
    log: tui_logger::TuiWidgetState,
}

impl<'arena, Node: Ast<'arena> + 'arena> Editor<'arena, Node> {
    /// Create a new [`Editor`] with a given tree
    pub fn new(
        tree: &'arena mut Dag<'arena, Node>,
        format_style: Node::FormatStyle,
        config: Config,
        file_path: Option<PathBuf>,
    ) -> Editor<'arena, Node> {
        let mut term = Terminal::new(CrosstermBackend::new(io::stdout())).unwrap();
        crossterm::execute!(term.backend_mut(), terminal::EnterAlternateScreen).unwrap();
        terminal::enable_raw_mode().unwrap();
        Editor {
            tree,
            term,
            format_style,
            state: Box::new(normal_mode::State::default()),
            config,
            keystroke_log: KeyStrokeLog::new(10),
            file_path,
            log: tui_logger::TuiWidgetState::default(),
        }
    }

    /* ===== MAIN FUNCTIONS ===== */

    /// Update the terminal UI display
    fn update_display(&mut self) {
        let Self {
            term,
            state,
            tree,
            keystroke_log,
            config,
            format_style,
            log,
            ..
        } = self;
        term.draw(|f| {
            let area = f.size();
            let rows = Layout::default()
                .constraints(vec![Constraint::Min(0), Constraint::Length(1)])
                .split(area);
            let cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(vec![Constraint::Min(40), Constraint::Percentage(30)])
                .split(rows[0]);
            let details = Layout::default()
                .constraints(vec![Constraint::Percentage(60), Constraint::Min(5)])
                .split(cols[1]);

            f.render_widget(tui::widgets::Clear, area);
            f.render_widget(
                widgets::StatusBar {
                    keystroke_buffer: &state.keystroke_buffer(),
                },
                rows[1],
            );
            f.render_widget(
                widgets::TextView {
                    tree: &*tree,
                    color_scheme: &config.color_scheme,
                    format_style: &*format_style,
                },
                cols[0],
            );
            f.render_widget(&*keystroke_log, details[0]);
            let mut logger = tui_logger::TuiLoggerWidget::default();
            logger.state(&*log);
            f.render_widget(logger, details[1]);
        })
        .unwrap();
    }

    fn mainloop(&mut self) {
        log::trace!("Starting mainloop");
        self.update_display();
        // Sit in the infinte mainloop
        while let Ok(event) = crossterm::event::read() {
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
                .set_max_entries(self.term.size().unwrap().height.into());
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
        crossterm::execute!(
            self.term.backend_mut(),
            terminal::LeaveAlternateScreen,
            cursor::Show
        )
        .unwrap();
        terminal::disable_raw_mode().unwrap();
    }
}
