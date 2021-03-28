//! An extension trait to add a convenient way to display keystrokes

use std::borrow::Cow;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// An extension trait to add a convenient way to display keystrokes
pub trait KeyDisplay {
    /// Return a compact string representing this keystroke
    fn compact_string(self) -> Cow<'static, str>;
}

impl KeyDisplay for KeyEvent {
    fn compact_string(self) -> Cow<'static, str> {
        if self.modifiers.contains(KeyModifiers::CONTROL) {
            if let KeyCode::Char(c) = self.code {
                return Cow::from(format!("^{}", c));
            }
        }
        if self.modifiers.is_empty() {
            match self.code {
                KeyCode::Char(c) => Cow::from(String::from(c)),

                KeyCode::Up => Cow::from("<Up>"),
                KeyCode::Down => Cow::from("<Down>"),
                KeyCode::Left => Cow::from("<Left>"),
                KeyCode::Right => Cow::from("<Right>"),

                KeyCode::F(num) => Cow::from(format!("<F{}>", num)),
                KeyCode::Esc => Cow::from("<Esc>"),

                KeyCode::Backspace => Cow::from("<BS>"),
                KeyCode::Delete => Cow::from("<Del>"),

                KeyCode::Tab => Cow::from("<Tab>"),
                KeyCode::Enter => Cow::from("<CR>"),

                KeyCode::Insert => Cow::from("<Insert>"),
                KeyCode::Home => Cow::from("<Home>"),
                KeyCode::End => Cow::from("<End>"),
                KeyCode::PageUp => Cow::from("<PageUp>"),
                KeyCode::PageDown => Cow::from("<PageDown>"),

                _ => Cow::from(format!("{:?}", self)),
            }
        } else {
            Cow::from(format!("{:?}", self))
        }
    }
}

/// converts a `Key`s keystroke_buffer into a `String`
pub fn keystrokes_to_string(keystroke_buffer: &[KeyEvent]) -> String {
    keystroke_buffer
        .iter()
        .map(|x| x.compact_string())
        .collect()
}
