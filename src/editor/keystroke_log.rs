//! A utility datastructure to store and render a log of keystrokes.  This is mostly used to give
//! the viewers of my streams feedback for what I'm typing.

#[allow(unused_imports)] // used solely for doc-comments
use super::normal_mode::Action;

use tuikit::prelude::*;

/// A category grouping similar actions
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Category {
    /// An [`Action`] that moves the cursor
    Move,
    /// Either [`Action::Undo`] or [`Action::Redo`]
    History,
    /// An [`Action`] that inserts extra nodes into the tree
    Insert,
    /// An [`Action`] that replaces some nodes in the tree
    Replace,
    /// An [`Action`] that causes nodes to be deleted from the tree
    Delete,
    /// The action of the keystrokes is that Sapling should quit
    Quit,
    /// The keystrokes did not correspond to a well-defined action
    Undefined,
}

/// Returns the [`Color`] that all [`Action`]s of a given [`Category`] should be
/// displayed.  This is not implemented as a method on [`Category`], because doing so
/// would require [`Category`] to rely on the specific terminal backend used.  This way,
/// we keep the terminal backend as encapsulated as possible.
impl Category {
    fn term_color(self) -> Color {
        match self {
            Category::Move => Color::LIGHT_BLUE,
            Category::History => Color::LIGHT_YELLOW,
            Category::Insert => Color::LIGHT_GREEN,
            Category::Replace => Color::CYAN,
            Category::Delete => Color::RED,
            Category::Quit => Color::MAGENTA,
            Category::Undefined => Color::LIGHT_RED,
        }
    }
}

/// One entry in the log.  This usually represts a single keystroke, but could represent an
/// accumulation of many identical keystrokes that are executed consecutively.
struct Entry {
    count: usize,
    keystroke: String,
    description: String,
    color: Color,
}

/// A utility struct to store and display a log of which keystrokes have been executed recently.
/// This is mostly used to give the viewers of my streams feedback for what I'm typing.
pub struct KeyStrokeLog {
    /// A list of keystrokes that have been run
    keystrokes: Vec<Entry>,
    max_entries: usize,
}

impl KeyStrokeLog {
    /// Create a new (empty) keystroke log
    pub fn new(max_entries: usize) -> KeyStrokeLog {
        KeyStrokeLog {
            keystrokes: vec![],
            max_entries,
        }
    }

    /// Sets and enforces the max entry limit
    pub fn set_max_entries(&mut self, max_entries: usize) {
        self.max_entries = max_entries;
        self.enforce_entry_limit();
    }

    /// Draw a log of recent keystrokes to a given terminal at a given location
    pub fn render(&self, term: &Term, row: usize, col: usize) {
        // Calculate how wide the numbers column should be, enforcing that it is at least two
        // chars wide.
        let count_col_width = self
            .keystrokes
            .iter()
            .map(|e| match e.count {
                1 => 0,
                c => format!("{}x", c).len(),
            })
            .max()
            .unwrap_or(0)
            .max(2);
        // Calculate the width of the keystroke column, and make sure that it is at least two
        // chars wide.
        let cmd_col_width = self
            .keystrokes
            .iter()
            .map(|e| e.keystroke.len())
            .max()
            .unwrap_or(0)
            .max(2);
        // Render the keystrokes
        for (i, e) in self.keystrokes.iter().enumerate() {
            // Print the count if greater than 1
            if e.count > 1 {
                term.print(row + i, col, &format!("{}x", e.count)).unwrap();
            }
            // Print the keystrokes in one column
            term.print_with_attr(
                row + i,
                col + count_col_width + 1,
                &e.keystroke,
                Attr::default().fg(Color::WHITE),
            )
            .unwrap();
            // Print a `=>`
            term.print(row + i, col + count_col_width + 1 + cmd_col_width + 1, "=>")
                .unwrap();
            // Print the meanings in another column
            term.print_with_attr(
                row + i,
                col + count_col_width + 1 + cmd_col_width + 4,
                &e.description,
                Attr::default().fg(e.color),
            )
            .unwrap();
        }
    }

    /// Repeatedly remove keystrokes until the entry limit is satisfied
    fn enforce_entry_limit(&mut self) {
        while self.keystrokes.len() > self.max_entries {
            self.keystrokes.remove(0);
        }
    }

    /// Pushes a new keystroke to the log.
    pub fn push(&mut self, keystroke: String, description: String, category: Category) {
        // If the keystroke is identical to the last log entry, incrememnt that counter by one
        if Some(&keystroke) == self.keystrokes.last().map(|e| &e.keystroke) {
            // We can safely unwrap here, because the guard of the `if` statement guaruntees
            // that `self.keystroke.last()` is `Some(_)`
            self.keystrokes.last_mut().unwrap().count += 1;
            return;
        }
        self.keystrokes.push(Entry {
            count: 1,
            keystroke,
            description,
            color: category.term_color(),
        });
        // Since we added an item, we should enforce the entry limit
        self.enforce_entry_limit();
    }
}
