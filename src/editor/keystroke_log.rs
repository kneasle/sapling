//! A utility datastructure to store and render a log of keystrokes.  This is mostly used to give
//! the viewers of my streams feedback for what I'm typing.

use crate::core::keystrokes_to_string;

#[allow(unused_imports)] // Only used by doc-comments, which rustc can't see
use super::normal_mode::Action;

use crossterm::event::KeyEvent;
use tui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    style::{Color, Style},
    widgets::{Cell, Row, Table, Widget},
};

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
    /// An [`Action`] that handles reading and writing from disk
    IO,
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
            Category::Move => Color::LightBlue,
            Category::History => Color::LightYellow,
            Category::Insert => Color::LightGreen,
            Category::Replace => Color::Cyan,
            Category::Delete => Color::Red,
            Category::Quit => Color::Magenta,
            Category::IO => Color::Green,
            Category::Undefined => Color::LightRed,
        }
    }
}

/// One entry in the log.  This usually represts a single keystroke, but could represent an
/// accumulation of many identical keystrokes that are executed consecutively.
struct Entry {
    count: usize,
    keystrokes: Vec<KeyEvent>,
    description: String,
    color: Color,
}

impl Entry {
    fn keystroke_string(&self) -> String {
        keystrokes_to_string(&self.keystrokes)
    }
}

/// A utility struct to store and display a log of which keystrokes have been executed recently.
/// This is mostly used to give the viewers of my streams feedback for what I'm typing.
pub struct KeyStrokeLog {
    /// A list of keystrokes that have been run
    keystrokes: Vec<Entry>,
    /// The maximum number of entries that should be displayed at once
    max_entries: usize,
    /// The keystrokes that will be included in the next log entry
    unlogged_keystrokes: Vec<KeyEvent>,
}

impl Widget for &'_ KeyStrokeLog {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let rows_displayed = self
            .keystrokes
            .len()
            .checked_sub(area.height as usize)
            .unwrap_or(0);
        Table::new(self.keystrokes[rows_displayed..].iter().map(|entry| {
            Row::new(vec![
                if entry.count > 1 {
                    format!("{}x", entry.count).into()
                } else {
                    Cell::default()
                },
                Cell::from(entry.keystroke_string()).style(Style::default().fg(Color::White)),
                "=>".into(),
                Cell::from(&*entry.description).style(Style::default().fg(entry.color)),
            ])
        }))
        .widths(&[
            Constraint::Min(2),
            Constraint::Min(2),
            Constraint::Length(2),
            Constraint::Min(0),
        ])
        .render(area, buf);
    }
}

impl KeyStrokeLog {
    /// Create a new (empty) keystroke log
    pub fn new(max_entries: usize) -> KeyStrokeLog {
        KeyStrokeLog {
            keystrokes: vec![],
            max_entries,
            unlogged_keystrokes: vec![],
        }
    }

    /// Sets and enforces the max entry limit
    pub fn set_max_entries(&mut self, max_entries: usize) {
        self.max_entries = max_entries;
        self.enforce_entry_limit();
    }

    /// Repeatedly remove keystrokes until the entry limit is satisfied
    fn enforce_entry_limit(&mut self) {
        while self.keystrokes.len() > self.max_entries {
            self.keystrokes.remove(0);
        }
    }

    /// Log a new [`KeyEvent`] that should be added to the next log entry.
    pub fn push_key(&mut self, key: KeyEvent) {
        self.unlogged_keystrokes.push(key);
    }

    /// Creates a new entry in the log, which occured as a result of the [`KeyEvent`]s already
    /// [`push_key`](Self::push_key)ed.
    pub fn log_entry(&mut self, description: String, category: Category) {
        // If the keystroke is identical to the last log entry, incrememnt that counter by one
        if Some(&self.unlogged_keystrokes) == self.keystrokes.last().map(|e| &e.keystrokes) {
            // We can safely unwrap here, because the guard of the `if` statement guaruntees
            // that `self.keystroke.last()` is `Some(_)`
            self.keystrokes.last_mut().unwrap().count += 1;
        } else {
            self.keystrokes.push(Entry {
                count: 1,
                keystrokes: self.unlogged_keystrokes.clone(),
                description,
                color: category.term_color(),
            });
            // Since we added an item, we should enforce the entry limit
            self.enforce_entry_limit();
        }
        self.unlogged_keystrokes.clear();
    }
}
