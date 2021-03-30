use super::DEBUG_HIGHLIGHTING;
use crate::ast::{
    display_token::{DisplayToken, SyntaxCategory},
    Ast,
};

use std::collections::{hash_map::DefaultHasher, HashSet};
use std::hash::Hasher;

use tui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Style},
    widgets::{Paragraph, Widget},
};

pub struct StatusBar<'a> {
    pub keystroke_buffer: &'a str,
}
impl Widget for StatusBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Paragraph::new(self.keystroke_buffer)
            .alignment(Alignment::Right)
            .render(area, buf);
        buf.set_string(area.x, area.y, "Press 'q' to exit", Style::default());
    }
}

pub struct TextView<'a, 'arena, Node: Ast<'arena>> {
    pub tree: &'a super::Dag<'arena, Node>,
    pub color_scheme: &'a crate::config::ColorScheme,
    pub format_style: &'a Node::FormatStyle,
}
impl<'arena, Node: Ast<'arena>> Widget for TextView<'_, 'arena, Node> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let cols = [
            Color::Magenta,
            Color::Red,
            Color::Yellow,
            Color::Gray,
            Color::Cyan,
            Color::Blue,
            Color::White,
            Color::LightRed,
            Color::LightBlue,
            Color::LightCyan,
            Color::LightGreen,
            Color::LightYellow,
            Color::LightMagenta,
        ];

        /* RENDER MAIN TEXT VIEW */
        // Mutable variables to track where the terminal cursor should go
        let mut row = area.top();
        let mut col = area.left();
        let mut indentation_amount = 0;

        let mut unknown_categories: HashSet<SyntaxCategory> = HashSet::with_capacity(0);

        for (node, tok) in self.tree.root().display_tokens(self.format_style) {
            match tok {
                DisplayToken::Text(s, category) => {
                    let color = if DEBUG_HIGHLIGHTING {
                        // Hash the ref to decide on the colour
                        let mut hasher = DefaultHasher::new();
                        node.hash(&mut hasher);
                        let hash = hasher.finish();
                        cols[hash as usize % cols.len()]
                    } else {
                        *self.color_scheme.get(category).unwrap_or_else(|| {
                            unknown_categories.insert(category);
                            &Color::LightMagenta
                        })
                    };
                    // Generate the display attributes depending on if the node is selected
                    let style = if std::ptr::eq(node, self.tree.cursor()) {
                        Style::default().fg(Color::Black).bg(color)
                    } else {
                        Style::default().fg(color)
                    };
                    let mut lines = s.lines();
                    col = buf
                        .set_stringn(
                            col,
                            row,
                            lines.next().unwrap(),
                            (area.right() - col).into(),
                            style,
                        )
                        .0;
                    for _ in lines {
                        todo!("implement multiline tokens");
                    }
                }
                DisplayToken::Whitespace(n) => {
                    col += n as u16;
                }
                DisplayToken::Newline => {
                    row += 1;
                    if row == area.bottom() {
                        break;
                    }
                    col = indentation_amount;
                }
                DisplayToken::Indent => {
                    indentation_amount += 4;
                }
                DisplayToken::Dedent => {
                    indentation_amount -= 4;
                }
            }
        }

        // Print warning messages for unknown syntax categories
        for c in unknown_categories {
            log::error!("Unknown highlight category '{}'", c);
        }
    }
}
