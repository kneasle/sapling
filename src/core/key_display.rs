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

            Key::Up => Cow::from("<UP>"),
            Key::Down => Cow::from("<DOWN>"),
            Key::Left => Cow::from("<LEFT>"),
            Key::Right => Cow::from("<RIGHT>"),

            Key::F(num) => Cow::from(format!("<F{}>", num)),
            Key::ESC => Cow::from("<ESC>"),

            _ => unimplemented!(),
        }
    }
}
