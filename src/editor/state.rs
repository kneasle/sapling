//! Definition of the state machine of Sapling's editor modes

use super::dag::Dag;
use super::keystroke_log::Category;
use crate::ast::Ast;
use crate::config::Config;

use std::borrow::Cow;

use tuikit::prelude::Key;

/// The [`State`] that Sapling enters to quit the mainloop and exit
#[derive(Debug, Copy, Clone)]
pub struct Quit;

impl<'arena, Node: Ast<'arena>> State<'arena, Node> for Quit {
    fn transition(
        self: Box<Self>,
        _key: Key,
        _config: &Config,
        _tree: &mut Dag<'arena, Node>,
    ) -> (Box<dyn State<'arena, Node>>, Option<(String, Category)>) {
        (self, None)
    }

    fn is_quit(&self) -> bool {
        true
    }
}

/// A trait which should be implemented for every `State` in Sapling's state machine.
///
/// The current states are:
/// - [`Quit`]
/// - [`crate::editor::normal_mode::State`]
/// - `crate::editor::IntermediateState` (link doesn't work because `IntermediateState` is private)
pub trait State<'arena, Node: Ast<'arena>>: std::fmt::Debug {
    /// Consume a keystroke, returning the `State` after this transition
    fn transition(
        self: Box<Self>,
        key: Key,
        config: &Config,
        tree: &mut Dag<'arena, Node>,
    ) -> (Box<dyn State<'arena, Node>>, Option<(String, Category)>);

    /// Return the keystroke buffer that should be displayed in the bottom right corner of the
    /// screen
    fn keystroke_buffer(&self) -> Cow<'_, str> {
        Cow::from("")
    }

    /// Returns `true` if Sapling should quit.  By default, this returns `false`.  This should
    /// **only** be `true` for [`Quit`].
    fn is_quit(&self) -> bool {
        false
    }
}
