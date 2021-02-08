//! Definition of the state machine of Sapling's editor modes

use super::{keystroke_log::Category, Editor};
use crate::ast::Ast;

use std::borrow::Cow;

use tuikit::prelude::Key;

/// A trait which should be implemented for every `State` in Sapling's state machine.
///
/// The current states are:
/// - `crate::editor::command_mode::Quit`
/// - [`crate::editor::normal_mode::State`]
/// - [`crate::editor::command_mode::State`]
/// - `crate::editor::IntermediateState` (link doesn't work because `IntermediateState` is private)
pub trait State<'arena, Node: Ast<'arena>>: std::fmt::Debug {
    /// Consume a keystroke, returning the `State` after this transition
    fn transition(
        self: Box<Self>,
        key: Key,
        editor: &mut Editor<'arena, Node>,
    ) -> (Box<dyn State<'arena, Node>>, Option<(String, Category)>);

    /// Return the keystroke buffer that should be displayed in the bottom right corner of the
    /// screen
    fn keystroke_buffer(&self) -> Cow<'_, str> {
        Cow::from("")
    }

    /// Returns `true` if Sapling should quit.  By default, this returns `false`.  This should
    /// **only** be `true` for `crate::editor::command_mode::Quit`.(link doesn't work because 'Quit' is private)
    fn is_quit(&self) -> bool {
        false
    }

    /// Returns name of the current mode
    fn name(&self) -> &'arena str {
        return "";
    }
}
