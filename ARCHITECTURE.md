# Sapling's Architecture

This file is a high-level overview of Sapling's internal architecture - if you want to familiarise
yourself with Sapling's codebase, this is a good place to start.

## Bird's Eye View

Sapling is a general modal structured editor inspired by Vi/Vim and Kakoune.  What does that
mean?  Let's break it down into pieces:
- **"general"** means Sapling can, in theory, open files of any language; it is not language specific.
- **"modal"** means that Sapling has multiple modes of operation (same as Vim).  I.e., the same
  keystrokes will do different things depending on what state the editor is in.
- **"structured editor"** means that Sapling edits the syntax tree of your code, but stores the code
  on disk purely as text, with no other data.

## Code Map

This covers code roughly in 'top-down' order.

### `fn main::main`

The entry point of Sapling.  It does relatively little - it opens and parses a file (if a path is
given), creates an `Editor` singleton (along with its dependencies) and finally passes control into
the editor's mainloop, which won't return until Sapling closes.

### `struct editor::Editor`

A singleton struct that handles direct user input and displays everything to the user.  This mostly
delegates responsibility to other parts of the code (`Dag` for tree storage/editing, `Ast` for
rendering, and the various `State`s for deciding what action each keystroke should have).

The most important part of the struct is `editor::Editor::mainloop`, which is a very classic UI
mainloop - it consumes events (usually keystrokes), and updates the display whenever the user
presses a key.

### `struct config::Config`

Holds all the global editor configuration state of Sapling (e.g. keybindings, syntax highlight
colors).  The user currently cannot specify the configuration without changing this file and
recompiling.

### `trait editor::state::State`

A trait that determines what Sapling should do with keystrokes.  The meat of this is the
`transition` function, which consumes a keystroke and allows the `State` to either mutate itself or
return a new `State` for Sapling to use.  See also `editor::normal_mode::State` and
`editor::state::Quit`.

The different modes are at `editor::normal_mode`, `editor::command_mode` (tbc).

### `struct editor::dag::Dag`

This struct represents a single buffer open in memory - i.e. an AST along with an entire edit
history for that tree.  It stores this history as a DAG (Directed Acyclic Graph) to prevent
unnecessary duplication of nodes.  It provides convenient functions to do common edits (such as
inserting, deleting and replacing AST nodes), all of which use `Dag::perform_edit` to handle the
functionality common to all edits (e.g. cloning the required nodes to generate a new tree, adding
the new changes to the history).

### `trait ast::Ast`

This trait is the key to the generalness of Sapling, and is used to specify everything about the
language that Sapling is editing.  This includes rendering to text (with help from `DisplayToken`),
parsing and enforcing tree correctness.  `ast::json::Json` is an implementation of this
representing JSON.

### `mod core`

Core datatypes that will be used all across Sapling.  This includes things like `Path` (a
datatype that stores a path down a syntax tree), `Size` (which stores the on-screen size of an AST
node) and extension traits such as `KeyDisplay` (which makes it nicer to display terminal
keystrokes).

### `mod arena`

An 'arena' allocator which is used to store all the AST nodes as a Dag.  We use an arena because we
are going to be allocating tons of AST nodes as a graph, but very very few of these will be freed
before the editor closes.  Arenas are a very performant allocator for this use case, and it allows
all the nodes to have the same lifetime (the lifetime of the arena) which makes sure that the code
compiles.
