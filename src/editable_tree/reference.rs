use crate::ast_spec::Reference;

// An import solely used by doc-comments
#[allow(unused_imports)]
use super::EditableTree;

/// A small type used as a reference into Vec-powered [EditableTree]s.  `Ref` acts as a type-safe
/// alternative to just using [usize], and can only be created and used by code in the
/// editable_tree module - to the rest of the code `Ref`s are essentially black boxes.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Index(usize);

impl Reference for Index {}

impl From<usize> for Index {
    fn from(val: usize) -> Index {
        Index(val)
    }
}

impl Index {
    #[inline]
    pub(crate) fn as_usize(self) -> usize {
        self.0
    }
}
