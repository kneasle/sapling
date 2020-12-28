//! The code for 'normal-mode', similar to that of Vim

use crate::core::Direction;

/// The possible keystroke typed by user without any parameters.
/// It can be mapped to a single key.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum KeyStroke {
    /// Quit Sapling
    Quit,
    /// Replace the selected node, expects an argument
    Replace,
    /// Insert a new node as the last child of the cursor, expects an argument
    InsertChild,
    /// Insert a new node before the cursor, expects an argument
    InsertBefore,
    /// Insert a new node after the cursor, expects an argument
    InsertAfter,
    /// Delete the cursor
    Delete,
    /// Move cursor in given direction.  The direction is part of the keystroke, since the directions
    /// all correspond to single key presses.
    MoveCursor(Direction),
    /// Undo the last change
    Undo,
    /// Redo a change
    Redo,
}

impl KeyStroke {
    /// Returns a lower-case summary string of the given keystroke
    pub fn summary_string(&self) -> &'static str {
        match self {
            KeyStroke::Quit => "quit",
            KeyStroke::Replace => "replace",
            KeyStroke::InsertChild => "insert child",
            KeyStroke::InsertBefore => "insert before",
            KeyStroke::InsertAfter => "insert after",
            KeyStroke::Delete => "delete",
            KeyStroke::MoveCursor(Direction::Down) => "move to first child",
            KeyStroke::MoveCursor(Direction::Up) => "move to parent",
            KeyStroke::MoveCursor(Direction::Prev) => "move to previous sibling",
            KeyStroke::MoveCursor(Direction::Next) => "move to next sibling",
            KeyStroke::Undo => "undo",
            KeyStroke::Redo => "redo",
        }
    }
}

/// A category grouping similar actions
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum ActionCategory {
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
    /// The [`Action`] was to [`Quit`](Action::Quit)
    Quit,
    /// The [`Action`] was [`Undefined`](Action::Undefined)
    Undefined,
}

impl ActionCategory {}

/// Mapping of keys to keystrokes.
/// Shortcut definition, also allows us to change the type if needed.
pub type KeyMap = std::collections::HashMap<char, KeyStroke>;

/// Generates a 'canonical' [`KeyMap`].  These keybindings will be very similar to those of Vim.
pub fn default_keymap() -> KeyMap {
    hmap::hmap! {
        'q' => KeyStroke::Quit,
        'i' => KeyStroke::InsertBefore,
        'a' => KeyStroke::InsertAfter,
        'o' => KeyStroke::InsertChild,
        'r' => KeyStroke::Replace,
        'x' => KeyStroke::Delete,
        'c' => KeyStroke::MoveCursor(Direction::Down),
        'p' => KeyStroke::MoveCursor(Direction::Up),
        'h' => KeyStroke::MoveCursor(Direction::Prev),
        'j' => KeyStroke::MoveCursor(Direction::Next),
        'k' => KeyStroke::MoveCursor(Direction::Prev),
        'l' => KeyStroke::MoveCursor(Direction::Next),
        'u' => KeyStroke::Undo,
        'R' => KeyStroke::Redo
    }
}

/// The possible meanings of a user-typed keystroke
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Action {
    /// The user typed a keystroke combination that isn't defined and the keystroke buffer should
    /// be cleared
    Undefined(String),
    /// Quit Sapling
    Quit,
    /// Replace the selected node with a node represented by some [`char`]
    Replace(char),
    /// Insert a new node (given by some [`char`]) as the first child of the selected node
    InsertChild(char),
    /// Insert a new node (given by some [`char`]) before the cursor
    InsertBefore(char),
    /// Insert a new node (given by some [`char`]) after the cursor
    InsertAfter(char),
    /// Remove the node under the cursor
    Delete,
    /// Move the node in a given direction
    MoveCursor(Direction),
    /// Undo the last change
    Undo,
    /// Redo a change
    Redo,
}

impl Action {
    /// Returns a lower-case summary of the given keystroke, along with the color with which it
    /// should be displayed in the log.
    pub fn description(&self) -> String {
        match self {
            Action::Undefined(keys) => format!("undefined keystrokes '{}'", keys),
            Action::Quit => "quit Sapling".to_string(),
            Action::Replace(c) => format!("replace cursor with '{}'", c),
            Action::InsertChild(c) => format!("insert '{}' as last child", c),
            Action::InsertBefore(c) => format!("insert '{}' before cursor", c),
            Action::InsertAfter(c) => format!("insert '{}' after cursor", c),
            Action::Delete => "delete cursor".to_string(),
            Action::MoveCursor(Direction::Down) => "move to first child".to_string(),
            Action::MoveCursor(Direction::Up) => "move to parent".to_string(),
            Action::MoveCursor(Direction::Prev) => "move to previous sibling".to_string(),
            Action::MoveCursor(Direction::Next) => "move to next sibling".to_string(),
            Action::Undo => "undo a change".to_string(),
            Action::Redo => "redo a change".to_string(),
        }
    }

    /// Returns the [`ActionCategory`] of this `Action`
    pub fn category(&self) -> ActionCategory {
        match self {
            Action::Undefined(_) => ActionCategory::Undefined,
            Action::Quit => ActionCategory::Quit,
            Action::Replace(_) => ActionCategory::Replace,
            Action::InsertChild(_) | Action::InsertBefore(_) | Action::InsertAfter(_) => {
                ActionCategory::Insert
            }
            Action::Delete => ActionCategory::Delete,
            Action::MoveCursor(_) => ActionCategory::Move,
            Action::Undo | Action::Redo => ActionCategory::History,
        }
    }
}

/// Attempt to convert a keystroke as a `&`[`str`] into an [`Action`].
/// This parses the string from the start, and returns when it finds a valid keystroke.
///
/// Therefore, `"q489flshb"` will be treated like `"q"`, and will return `Some(Action::Quit)` even
/// though `"q489flshb"` is not technically valid.
/// This function is run every time the user types a keystroke character, and so the user would not
/// be able to input `"q489flshb"` to this function because doing so would require them to first
/// input every possible prefix of `"q489flshb"`, including `"q"`.
///
/// This returns:
/// - [`None`] if the keystroke is incomplete.
/// - [`Action::Undefined`] if the keystroke is not defined (like the keystroke "X").
/// - The corresponding [`Action`], otherwise.
pub fn parse_keystroke(keymap: &KeyMap, keystroke: &str) -> Option<Action> {
    let mut keystroke_char_iter = keystroke.chars();

    // Consume the first char of the keystroke
    let c = keystroke_char_iter.next()?;

    match keymap.get(&c) {
        // "q" quits Sapling
        Some(KeyStroke::Quit) => Some(Action::Quit),
        // this pattern is used several times: `keystroke_char_iter.next().map()
        // This consumes the second char of the iterator and, if it exists, returns
        // Some(Action::ThisAction(char))
        Some(KeyStroke::InsertChild) => keystroke_char_iter.next().map(Action::InsertChild),
        Some(KeyStroke::InsertBefore) => keystroke_char_iter.next().map(Action::InsertBefore),
        Some(KeyStroke::InsertAfter) => keystroke_char_iter.next().map(Action::InsertAfter),
        Some(KeyStroke::Delete) => Some(Action::Delete),
        Some(KeyStroke::Replace) => keystroke_char_iter.next().map(Action::Replace),
        Some(KeyStroke::MoveCursor(direction)) => Some(Action::MoveCursor(*direction)),
        Some(KeyStroke::Undo) => Some(Action::Undo),
        Some(KeyStroke::Redo) => Some(Action::Redo),
        None => Some(Action::Undefined(keystroke.to_string())),
    }
}
