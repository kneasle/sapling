use crate::editable_tree::Direction;

    pub mod command_log {
        //! A utility datastructure to store and render a log of commands.  This is mostly used to give
        //! the viewers of my streams feedback for what I'm typing.
    
        use super::ActionCategory;
        use tuikit::prelude::*;
    
        /// Returns the [`Color`] that all [`Action`]s of a given [`ActionCategory`] should be
        /// displayed.  This is not implemented as a method on [`ActionCategory`], because doing so
        /// would require [`ActionCategory`] to rely on the specific terminal backend used.  This way,
        /// we keep the terminal backend as encapsulated as possible.
        pub fn term_color(category: ActionCategory) -> Color {
            match category {
                ActionCategory::Move => Color::LIGHT_BLUE,
                ActionCategory::History => Color::LIGHT_YELLOW,
                ActionCategory::Insert => Color::LIGHT_GREEN,
                ActionCategory::Replace => Color::CYAN,
                ActionCategory::Delete => Color::RED,
                ActionCategory::Quit => Color::MAGENTA,
                ActionCategory::Undefined => Color::LIGHT_RED,
            }
        }
    
        /// One entry in the log.  This usually represts a single command, but could represent an
        /// accumulation of many identical commands that are executed consecutively.
        struct Entry {
            count: usize,
            command: String,
            description: String,
            color: Color,
        }
    
        /// A utility struct to store and display a log of which commands have been executed recently.
        /// This is mostly used to give the viewers of my streams feedback for what I'm typing.
        pub struct KeyStrokeLog {
            /// A list of commands that have been run
            keystrokes: Vec<Entry>,
            max_entries: usize,
        }
    
        impl KeyStrokeLog {
            /// Create a new (empty) command log
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
    
            /// Draw a log of recent commands to a given terminal at a given location
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
                // Calculate the width of the command column, and make sure that it is at least two
                // chars wide.
                let cmd_col_width = self
                    .keystrokes
                    .iter()
                    .map(|e| e.command.len())
                    .max()
                    .unwrap_or(0)
                    .max(2);
                // Render the commands
                for (i, e) in self.keystrokes.iter().enumerate() {
                    // Print the count if greater than 1
                    if e.count > 1 {
                        term.print(row + i, col, &format!("{}x", e.count)).unwrap();
                    }
                    // Print the commands in one column
                    term.print_with_attr(
                        row + i,
                        col + count_col_width + 1,
                        &e.command,
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
    
            /// Repeatedly remove commands until the entry limit is satisfied
            fn enforce_entry_limit(&mut self) {
                while self.keystrokes.len() > self.max_entries {
                    self.keystrokes.remove(0);
                }
            }
    
            /// Pushes a new command to the log.
            pub fn push(&mut self, command: String, keymap: &super::KeyMap) {
                // If the command is identical to the last log entry, incrememnt that counter by one
                if Some(&command) == self.keystrokes.last().map(|e| &e.command) {
                    // We can safely unwrap here, because the guard of the `if` statement guaruntees
                    // that `self.command.last()` is `Some(_)`
                    self.keystrokes.last_mut().unwrap().count += 1;
                    return;
                }
                // If the command is different, then we should add a new entry for it
                let (description, color) = if command.is_empty() {
                    log::error!("Empty command executed!");
                    ("<empty command>".to_string(), Color::LIGHT_RED)
                } else if let Some(action) = super::parse_command(&keymap, &command) {
                    (action.description(), term_color(action.category()))
                } else {
                    log::error!("Incomplete command executed!");
                    ("<incomplete command>".to_string(), Color::LIGHT_RED)
                };
                self.keystrokes.push(Entry {
                    count: 1,
                    command,
                    description,
                    color,
                });
                // Since we added an item, we should enforce the entry limit
                self.enforce_entry_limit();
            }
        }
    }
    
    /// The possible command typed by user without any parameters.
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
        /// Move cursor in given direction.  The direction is part of the command, since the directions
        /// all correspond to single key presses.
        MoveCursor(Direction),
        /// Undo the last change
        Undo,
        /// Redo a change
        Redo,
    }
    
    impl KeyStroke {
        /// Returns a lower-case summary string of the given command
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
    
    /// Mapping of keys to commands.
    /// Shortcut definition, also allows us to change the type if needed.
    pub type KeyMap = std::collections::HashMap<char, KeyStroke>;
    
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
    
    /// The possible meanings of a user-typed command
    #[derive(Debug, Clone, Eq, PartialEq, Hash)]
    pub enum Action {
        /// The user typed a command that isn't defined, but the command box should still be cleared
        Undefined,
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
        /// Returns a lower-case summary of the given command, along with the color with which it
        /// should be displayed in the log.
        pub fn description(&self) -> String {
            match self {
                Action::Undefined => "undefined command".to_string(),
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
                Action::Undefined => ActionCategory::Undefined,
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
    
    /// Attempt to convert a command as a `&`[`str`] into an [`Action`].
    /// This parses the string from the start, and returns when it finds a valid command.
    ///
    /// Therefore, `"q489flshb"` will be treated like `"q"`, and will return `Some(Action::Quit)` even
    /// though `"q489flshb"` is not technically valid.
    /// This function is run every time the user types a command character, and so the user would not
    /// be able to input `"q489flshb"` to this function because doing so would require them to first
    /// input every possible prefix of `"q489flshb"`, including `"q"`.
    ///
    /// This returns:
    /// - [`None`] if the command is incomplete.
    /// - [`Action::Undefined`] if the command is not defined (like the command "X").
    /// - The corresponding [`Action`], otherwise.
    pub fn parse_command(keymap: &KeyMap, command: &str) -> Option<Action> {
        let mut command_char_iter = command.chars();
    
        // Consume the first char of the command
        let c = command_char_iter.next()?;
    
        match keymap.get(&c) {
            // "q" quits Sapling
            Some(KeyStroke::Quit) => Some(Action::Quit),
            // this pattern is used several times: `command_char_iter.next().map()
            // This consumes the second char of the iterator and, if it exists, returns
            // Some(Action::ThisAction(char))
            Some(KeyStroke::InsertChild) => command_char_iter.next().map(Action::InsertChild),
            Some(KeyStroke::InsertBefore) => command_char_iter.next().map(Action::InsertBefore),
            Some(KeyStroke::InsertAfter) => command_char_iter.next().map(Action::InsertAfter),
            Some(KeyStroke::Delete) => Some(Action::Delete),
            Some(KeyStroke::Replace) => command_char_iter.next().map(Action::Replace),
            Some(KeyStroke::MoveCursor(direction)) => Some(Action::MoveCursor(*direction)),
            Some(KeyStroke::Undo) => Some(Action::Undo),
            Some(KeyStroke::Redo) => Some(Action::Redo),
            None => Some(Action::Undefined),
        }
    }


