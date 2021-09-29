//! Front-end- and language-independent code for the Sapling **editor**.

pub mod ast; // Tree storage
mod cursors; // Cursor location
mod lang; // Language definition

pub use lang::Lang;
