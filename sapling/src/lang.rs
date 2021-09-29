use std::{
    path::{Path, PathBuf},
    rc::Rc,
};

use sapling_grammar::{parser, tokenizer::Tokenizer, Grammar, SpecGrammar, TypeId};
use serde::Deserialize;

use crate::ast::Tree;

/// The data required for Sapling to parse and edit a programming language.
#[derive(Debug, Clone)]
pub struct Lang {
    header: Header,
    grammar: Grammar,
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
        Ok(Self {
            header: lang_file.header,
            grammar,
        })
    }

    pub fn grammar(&self) -> &Grammar {
        &self.grammar
    }

    // TODO: Remove this
    pub fn tokenize<'s, 't>(&'t self, s: &'s str) -> (&'s str, Tokenizer<'s, 't>) {
        Tokenizer::new(self.grammar(), s)
    }

    pub fn parse(&self, type_id: TypeId, s: &str) -> Result<Tree, parser::Error> {
        self.grammar
            .parse(type_id, s)
            .map(|(leading_ws, root)| Tree {
                leading_ws: leading_ws.to_owned(),
                root: Rc::new(root),
            })
    }

    pub fn parse_root(&self, s: &str) -> Result<Tree, parser::Error> {
        self.parse(self.grammar.root_type(), s)
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

/// The errors generated when loading a [`Lang`] from a TOML file.
#[derive(Debug)]
pub enum LoadError {
    Io(PathBuf, std::io::Error),
    Parse(toml::de::Error),
    Convert(sapling_grammar::spec::convert::Error),
}
