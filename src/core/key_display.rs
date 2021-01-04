//! An extension trait to add a convenient way to display keystrokes

use std::borrow::Cow;

use tuikit::prelude::Key;

/// An extension trait to add a convenient way to display keystrokes
pub trait KeyDisplay {
    /// Return a compact string representing this keystroke
    fn compact_string(self) -> Cow<'static, str>;
}

impl KeyDisplay for Key {
    fn compact_string(self) -> Cow<'static, str> {
        match self {
            Key::Char(c) => Cow::from(String::from(c)),
            Key::Ctrl(c) => Cow::from(format!("^{}", c)),

            Key::Up => Cow::from("<Up>"),
            Key::Down => Cow::from("<Down>"),
            Key::Left => Cow::from("<Left>"),
            Key::Right => Cow::from("<Right>"),

            Key::F(num) => Cow::from(format!("<F{}>", num)),
            Key::ESC => Cow::from("<Esc>"),

            Key::Backspace => Cow::from("<BS>"),
            Key::Delete => Cow::from("<Del>"),

            Key::Tab => Cow::from("<Tab>"),
            Key::Enter => Cow::from("<CR>"),

            Key::Insert => Cow::from("<Insert>"),
            Key::Home => Cow::from("<Home>"),
            Key::End => Cow::from("<End>"),
            Key::PageUp => Cow::from("<PageUp>"),
            Key::PageDown => Cow::from("<PageDown>"),

            _ => Cow::from(format!("{:?}", self)),
        }
    }
}
