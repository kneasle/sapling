//! Module containing code for the 'arena' that stores AST nodes.

use typed_arena::Arena as TyArena;

/// An item that is stored in the [`Arena`].  This allows the [`Arena`] to build on
/// [`typed_arena::Arena`] by storing extra detail about the nodes stored in the arena.
struct Item<T> {
    node: T,
}

impl<T> Item<T> {
    /// Constructs a new `Item` that contains a given node
    pub fn new(node: T) -> Self {
        Item { node }
    }
}

/// An arena allocator for syntax tree nodes.  Sapling needs a way to efficiently store AST nodes,
/// because editing code in Sapling will result in many many nodes being created.  However, they
/// are not deallocated very often (if at all) so it makes sense to store them in an arena so that
/// when the user closes Sapling all the nodes can be destroyed quickly without requiring lots of
/// heap cleanup.
///
/// This also differs from standard arena allocators in the following ways:
/// - Nodes added to an [`Arena`] are **always immutable**.  Once they are added they can be cloned
///   but not changed.
/// - This does not merge syntax tree nodes (where rustc does).  Sapling relies on the fact that
///   within a given tree in the arena, all the nodes in that tree must have unique references.
///   Nodes **can** exist inside multiple trees at once.
pub struct Arena<T> {
    base_arena: TyArena<Item<T>>,
}

impl<T> Arena<T> {
    /// Creates an empty `Arena` of a given type.
    pub fn new() -> Arena<T> {
        Arena {
            base_arena: TyArena::new(),
        }
    }

    /// Add a new node to the `Arena`, and returns an immutable reference to its final location.
    pub fn alloc(&mut self, node: T) -> &T {
        &self.base_arena.alloc(Item::new(node)).node
    }
}
