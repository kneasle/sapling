use crate::ast_spec::Reference;

// An import solely used by doc-comments
#[allow(unused_imports)]
use super::EditableTree;

/// A small type used as a reference into Vec-powered [EditableTree]s.  `Ref` acts as a type-safe
/// alternative to just using [usize], and can only be created and used by code in the
/// editable_tree module - to the rest of the code `Ref`s are essentially black boxes.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Ref(usize);

impl Reference for Ref {}

impl From<usize> for Ref {
    fn from(val: usize) -> Ref {
        Ref(val)
    }
}

impl Ref {
    #[inline]
    pub(crate) fn as_usize(self) -> usize {
        self.0
    }
}
