use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use sapling_grammar::{ConvertError, Grammar, Parser, SpecGrammar};
use serde::Deserialize;

/// The data required for Sapling to parse and edit a programming language.
#[derive(Debug, Clone)]
pub struct Lang {
    header: Header,
    // This is stored in an `Arc` it is jointly owned by the `Parser`
    grammar: Arc<Grammar>,
    parser: Parser,
}

impl Lang {
    pub fn load_toml_file(path: impl AsRef<Path>) -> Result<Self, LoadError> {
        let path = path.as_ref();
        let toml_string =
            std::fs::read_to_string(path).map_err(|e| LoadError::Io(path.to_owned(), e))?;
        Self::from_toml(&toml_string)
    }

    pub fn from_toml(s: &str) -> Result<Self, LoadError> {
        let lang_file: LangFile = toml::from_str(s).map_err(LoadError::Parse)?;
        let grammar = lang_file
            .grammar
            .into_grammar()
            .map_err(LoadError::Convert)?;
        let grammar = Arc::new(grammar);
        Ok(Self {
            header: lang_file.header,
            parser: Parser::new(grammar.clone()),
            grammar,
        })
    }
}

/// Data relating to this language that is parsed from the file but not dependent on the
/// [`Grammar`]
#[derive(Debug, Clone, Deserialize)]
struct Header {
    name: String,
}

//////////////////////////
// FILE PARSING/LOADING //
//////////////////////////

/// Data structure into which TOML files get [`Deserialize`]d.
#[derive(Debug, Clone, Deserialize)]
struct LangFile {
    #[serde(rename = "lang")]
    header: Header,
    grammar: SpecGrammar,
}

#[derive(Debug)]
pub enum LoadError {
    Io(PathBuf, std::io::Error),
    Parse(toml::de::Error),
    Convert(ConvertError),
}
