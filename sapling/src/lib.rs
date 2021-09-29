//! Front-end- and language-independent code for the Sapling **editor**.

pub mod ast; // Tree storage
mod buffer; // A single 'file' (which may or may not correspond to an OS file)
mod cursors; // Cursor location
mod history; // A history of syntax `Tree`s
mod lang; // Language definition

pub use lang::Lang;
