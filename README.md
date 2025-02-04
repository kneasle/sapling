<img src="https://raw.githubusercontent.com/kneasle/sapling/master/resources/sapling.gif" alt="Sapling Logo" align="right" />

# Sapling

A highly experimental code editor where you edit code, not text.

_I (@kneasle) don't currently have time to work on Sapling (I have exams coming up
and other projects are requiring my time).  This doesn't mean the project is dead or that I've
forgotten about it - I still think it's cool and the community support has been crazy.  I'm also
not currently streaming development, but all vods are still on
[my YouTube channel](https://www.youtube.com/channel/UCKl0T4IDZC3vUz152hDAzGw) for anyone
interested.  There's also a discord server for discussions about Sapling and structured editing in
general ([invite link](https://discord.gg/Amv3Tsb4fV)). OK enough from me, onto the real README!_

Most of the ideas for this project come from my friend Shtanton's
[blog post](http://shtanton.com/posts/ex.html).  The concept of directly editing syntax trees is called
['structured editing'](https://en.wikipedia.org/wiki/Structure_editor) and is not new;
the purpose of Sapling is to use ideas from structured editing to **speed up** moment-to-moment
code editing, much how editors like Vim and Emacs speed up editing.  Sapling's editing model will
be largely inspired by [Vim](https://github.com/vim/vim)/[NeoVim](https://github.com/neovim/neovim)
and [kakoune](https://github.com/mawww/kakoune).  Sapling also aims to be **general purpose** -
Sapling should be able to edit any language, given that a suitable grammar is provided.

_Contributions of all kinds are very welcome!_

It is worth noting that Sapling is primarily **an experiment** to determine whether or not such an
editor could work.  Therefore, for the time being, Sapling can be expected to change at any time.
Hopefully the design of Sapling will converge over time - its current state is similar to how
pre-1.0 Rust was continually evolving and making potentially-breaking changes so that post-1.0 Rust
could be as useful as possible.

## Contents

- [**But Why?**](#but-why)
- [**Goals of Sapling**](#goals-of-sapling)
- [**Inspirations**](#inspirations)
- [**Quick Start**](#quick-startplay-with-sapling)
- [**Pros of AST-based Editing**](#pros-of-ast-based-editing)
- [**'Extra Fun Challenges'**](#cons-of-ast-based-editing-otherwise-known-as-extra-fun-challenges)
- [**What's an AST?**](#whats-an-ast)

---

## But why?

When writing code with any text editor, you are usually only interested in a tiny subset of all the
possible strings of text - those that correspond to valid programs in whatever language you're
writing.  In a **text** editor, you will spend the overwhelming majority of your time with the text
in your editor being invalid as you make edits to move between valid programs.  This is inefficient
for the programmer, and causes lots of issues for software like Language Servers which have to cope
as best they can with these invalid states.

To be fair, editors like Vim, Emacs and Kakoune do better than most by providing shortcuts to do 
common text manipulations, which is a step in the right direction.  Interestingly, though, the most 
useful of these shortcuts are those correspond to modifications of the syntax tree (e.g. `ci)` to
remove the replace the contents of `()` in Vim), and so it seems logical to apply modal editing to
directly modifying the syntax trees of programs.

Sapling takes the idea of keystrokes primarily modifying text, but instead applies those keystrokes
as actions to the syntax tree of your program.  I have no idea if this will be useful, but it seems
worth a try.

## Goals of Sapling

These goals are roughly in order of importance, with the most important first:

- **Editing Speed**: Sapling should be an editor that allows power users to edit code as close
  to their thinking speed as possible.  Flattening the learning curve is also important, but
  Sapling is not trying to be an editor for every single developer and is designed primarily with
  power users in mind.
- **Stability**: Sapling should not, under any circumstances, corrupt the user's data or crash.
  Either of these are considered critical bugs and should be reported.
- **Generality**: Sapling should, in theory, be able to edit any language.  This will likely be done
  with making the language plug-in-able and probably specified by some kind of grammar.
- **Familiarity**: Sapling should feel familiar to people who are used to modal editors such as Vim
  and Kakoune.  However, some alterations are required for Sapling to edit ASTs and not just text.
- **Interactivity**: Sapling should always give the user immediate feedback about their actions.
  Kakoune is a model example of this, and Vim/NeoVim does pretty well too.
- **Performance**: The user should not have to wait for Sapling to do anything.  Sapling should also
  have a small resource footprint - an editor should not have to use several hundred megabytes of
  RAM when idling.

## Inspirations:

- _[Vim](https://github.com/vim/vim), [NeoVim](https://github.com/neovim/neovim) and
  [Kakoune](https://github.com/mawww/kakoune)_:
  'Modal' editors where keystrokes can correspond to _actions_ on the text rather than always
  inserting directly to the text buffer.  Shoutout in particular to Kakoune for its beautiful
  multi-selection based editing model.
- _[Tree Sitter](https://github.com/tree-sitter/tree-sitter)_: A generic, flexible, error-handling
  parser that is not language specific.  Designed primarily to provide better syntax highlighting
  for [the Atom text editor](https://github.com/atom/atom).
- _[grasp](http://www.graspjs.com/)_: A regex-like language for searching JavaScript ASTs.
- _[r/nosyntax](https://reddit.com/r/nosyntax)_: A subreddit for strutured and projectional editors.  They also have
  [a list](https://www.reddit.com/r/nosyntax/comments/6xv61b/list_of_projects/) of such projects.
- _[Barista](https://www.researchgate.net/publication/221518157_Barista_An_implementation_framework_for_enabling_new_tools_interaction_techniques_and_views_in_code_editors)_:
  A structured editor that allows the user to fall back on text editing if required, which is
  something I'd like to explore for Sapling.  The source code is
  [here](https://github.com/amyjko/citrus-barista), but since this was a research project it
  seems to be unmaintained.
  
## Quick Start/Play with Sapling

### Installation

Sapling is not yet on [crates.io](crates.io) and is very much still in early development, but if you
want to play around with Sapling as it currently stands, the best way is to clone the repository and
build from source (you'll need [Rust](https://www.rust-lang.org/learn/get-started) installed in
order to do this):
```bash
git clone https://github.com/kneasle/sapling.git
cargo run 2> log
```

If you have Nix you can also run `nix build` and `./result/bin/sapling 2> log`, or if you don't mind
seeing stderr output even quicker with `nix run`. 

### Demo

![Demo GIF](https://user-images.githubusercontent.com/60934058/112751246-d8e23f00-8fc4-11eb-9a15-8a3ef32d54a4.gif)

### Current Keybindings

#### Misc

- `q`: Quit Sapling
- `u`: Undo a change
- `R`: Redo a change

#### Cursor Movement

- `h`/`k`: Move the cursor to the previous sibling of the current node
- `j`/`l`: Move the cursor to the next sibling of the current node
- `c`: Move the cursor to the first child of the current node (if it exists)
- `p`: Move the cursor to the parent of the node it's currently at

#### Modify the tree
- `r*`: Replace the node under the cursor with the node represented by the key `*`
- `x`: Delete the node under the cursor
- `o*`: Insert a new node represented by `*` as a **child** of the cursor
- `a*`/`i*`: Insert a new node represented by `*` before or after the cursor respectively

As with Vim, all commands can be repeated by inserting a count before them.  For example, `3u` will
undo 3 steps in one go.

Sapling can currently only edit JSON with the following keys: `[a]rray`, `[o]bject`, `[t]rue`,
`[f]alse`, `[n]ull`, `[s]tring`.  There is currently no way to insert text into a string or to open
and close files (yet!).

Sapling handle multiple nodes in one go by adding a count before the node name, for example `i3t`
will insert 3 `true`s before the cursor.

## Pros of AST-based editing

- Because the editor already knows the syntactic structure of your program, the following are
  **much** easier to implement for every language supported by Sapling:
  - Syntax highlighting
  - Code folding
  - Auto-formatting of code (in fact, this is nearly automatic and elegantly preserving code
    formatting is hard)
- It will hopefully be **FAST** to edit code
- It might actually be more intuitive than text-based editing

## Cons of AST-based editing (otherwise known as 'Extra Fun Challenges')

Because the editor *has* to hold a valid program, the following things that other editors take for
granted are hard to implement:
- Searching a file - because only syntax tree nodes can be selected, we need a way to concisely
  search for nodes in a tree.  [grasp](http://www.graspjs.com/) seems like it'd be good inspiration
  for this.
- Just opening a file - opening a syntactically correct file is essentially the same as writing a
  compiler-esque parser for every language you want to load (not an easy task but there's plenty of
  literature/libraries already existing for this).  The real issue is that Sapling has to at least 
  attempt to open any file, regardless of syntactic correctness, and this essentially boils down to
  building an error-correcting parser that's generic enough to parse any language.
  
  [Tree Sitter](https://github.com/tree-sitter/tree-sitter) has already had a good crack at this
  problem, but Tree Sitter is geared towards providing accurate syntax highlighting and has a few
  missing features that Sapling needs:
  - Sapling needs comments to be preserved when parsing (but whitespace is perhaps not so essential)
  - Sapling needs to be able to render ASTs back to text, which I don't think Tree Sitter's grammars
    can handle
  
  For the sake of pragmatism, I think we should initially write a wrapper around tree-sitter for
  parsing/reading files so that Sapling at least works whilst we decide if a custom grammar is
  required (and if it is, how it should work).
  
## What's an AST?

AST stands for ['Abstract Syntax Tree'](https://en.wikipedia.org/wiki/Abstract_syntax_tree), and in
essence it is a tree-like representation of only the structure of a program, without any details
about formatting.

For example, the following Rust code:
```rust
fn foo(y: u64, z: u32) {
    let x = y * 3 + z as u64;
    combine(x, y);
}
```
would correspond to a syntax tree something like the following (simplified for demonstration
purposes).  Notice how each 'element' of the code corresponds to one 'node' in the syntax tree:

![Example tree](/resources/example_tree.png)
