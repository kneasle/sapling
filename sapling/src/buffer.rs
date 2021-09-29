use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    string::FromUtf8Error,
    sync::Arc,
};

use sapling_grammar::parser;

use crate::{history::History, Lang};

/// A single file syntax tree, along with its undo history.  This roughly corresponds to a file
/// on-disk, but buffers can be opened without having a corresponding file (like loading `vim` with
/// no arguments creates a text buffer but no corresponding file).
///
/// (NOTE: This hasn't been implemented yet)  As with Vim and Kakoune, the same `Buffer` can be
/// opened in several different [`View`]s (aka windows or panes).  Therefore, the `Buffer` doesn't
/// store the cursor locations because each [`View`] has its own [`Cursors`].
#[derive(Debug)]
pub struct Buffer {
    /// The [`File`] that this `Buffer` is connected to (i.e. `:e`, `:w` will read from/write to
    /// this file).
    file: Option<File>,
    /// The [`Lang`]uage being edited in this `Buffer`.
    lang: Arc<Lang>,
    /// The sequence of [`Tree`]s which records the undo history for this `Buffer`
    history: History,
}

impl Buffer {
    /// Creates a new `Buffer` which owns the file at a given [`Path`]
    ///
    /// TODO: Figure out what to do if the file doesn't exist (Vim opens an empty file and waits
    /// for you to save, but we'd probably want to do something like have the languages specify a
    /// default tree...)
    pub fn load_file(path: impl AsRef<Path>, lang: Arc<Lang>) -> Result<Self, Error> {
        // Helper closure to combine an `io::Error` with the file path to get a `self::Error`
        let wrap_io_err = |io_err| Error::Io(path.as_ref().to_owned(), io_err);

        // Load file
        let mut file = File::open(path.as_ref()).map_err(wrap_io_err)?;
        // Read file contents
        let file_len = file.metadata().map_err(wrap_io_err)?.len();
        let mut file_contents = Vec::<u8>::with_capacity(file_len as usize);
        file.read(&mut file_contents).map_err(wrap_io_err)?;
        let file_contents = String::from_utf8(file_contents).map_err(Error::Utf8)?;
        // Parse file contents
        let tree = lang.parse_root(&file_contents).map_err(Error::Parse)?;

        Ok(Self {
            file: Some(file),
            lang,
            history: History::new(tree, 10_000), // TODO: Make max_undo_depth configurable
        })
    }
}

/// The errors generated when loading files
#[derive(Debug)]
pub enum Error {
    Io(PathBuf, std::io::Error),
    Utf8(FromUtf8Error),
    Parse(parser::Error),
}
