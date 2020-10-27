# Sapling
![Sapling logo](https://raw.githubusercontent.com/kneasle/sapling/master/resources/sapling.gif)

A highly experimental code editor where you edit code, not text.

Most of the ideas for this project come from my friend Shtanton's [blog post](http://shtanton.com/ex.html), and Sapling's editing model will be largely inspired by that of Vim.

This project is highly experimental in nature - many (if not all) of these ideas and datastructures have never been used before (as far as I'm aware).
Contributions/issues are welcome, but Sapling is currently so young and the codebase so small that the code churn of my rapid iterating would probably cause PRs to immediately generate merge conflicts.
Once the codebase becomes remotely stable, I will gladly accept PRs.

I'm also treating this current codebase as a kind of prototype - I'm using it to explore implentation options before commiting to the best one and then aggressively refactoring out the unused code.

## Goals of Sapling
- Sapling's main goal is to make an editor that allows power users to edit code as close to their thinking speed as possible.
  Sapling is willing to sacrifice a potentially steep learning curve in favour of increased editing power.
- Sapling's default key bindings should be familiar to people used to Vim/Vi, although some alterations will be necessary as they have to edit ASTs not text.
- Sapling should be as snappy and resource light as possible without sacrificing safety.
  Spicy data structures are absolutely allowed so long as they increase the performance and don't hinder safety.
  
## What's an AST?
AST stands for ['Abstract Syntax Tree'](https://en.wikipedia.org/wiki/Abstract_syntax_tree), and in essence it is a tree-like representation of only the structure of a program, without any details about formatting.

For example, the following Rust code:
```rust
fn foo(y: u64, z: u32) {
    let x = y * 3 + z as u64;
    combine(x, y);
}
```
would correspond to a syntax tree something like the following (simplified for demonstration purposes):
![Example tree](/resources/example_tree.png)


## But why?
When writing code with any text editor, you are usually only interested in a tiny subset of all the possible strings of text - those that correspond to valid
programs in whatever language you're writing.  In a **text** editor, you will spend the overwhelming majority of your time
with the text in your editor being invalid as you make edits to move between valid programs.

This is inefficient for the programmer, and causes lots of issues for software like Language Servers which have to cope as best they can with these invalid states.

To be fair, editors like Vim and Emacs do better than most by providing shortcuts to do common text manipulations which is a step in the right direction.
However, when you're writing code, you're thinking about the **code** not the text and the ideal editor is one that thinks the same way you do.

The idea behind Sapling is that instead of using keyboard shortcuts to perform common edits on text (for example in Vim, `d3W` deletes forwards 3 whitespace-delimited words), we use keyboard shortcuts to directly edit the syntax tree of the code you're editing, which gets converted to text only when needed.
Instead of selecting substrings of text, we select nodes in the syntax tree (a node corresponds to any syntactic part of the code - from a single 'identifier' `variable_name` to an entire function definition `fn foo() { let variable_name = 3; }`).
And once we have a selection, we can perform actions directly on the tree to make the edits we desire (for example, typing `d3c` for `[d]elete 3 [c]hildren` when selecting a function definition would delete the next 3 function definitions, without having to select the exact text areas they correspond to).

## Pros of AST-based editing
- Because the editor already knows the syntactic structure of your program, the following are **much** easier to implement for every language supported by Sapling:
  - Syntax highlighting
  - Code folding
  - Auto-formatting of code (in fact, this is nearly automatic and *not* implementing it is hard)
- Syntax trees will potentially have lots of duplication (how many times does the identifier `i` appear in codebases?), so ASTs of a program could potentially be stored in much less space than the equivalent text, as well as being fast to edit (though probably not to render).
- It will hopefully be **FAST** to edit code

## Cons of AST-based editing (otherwise known as 'extra fun challenges')
Because the editor *has* to hold a valid program, the following things that other editors take for granted are hard to implement:
- Just opening a file - opening a syntactically correct file is essentially the same as writing a compiler-esque parser for every language you want to load
  (not an easy task but there's plenty of literature/libraries already existing for this).
  However, I think [tree-sitter](https://github.com/tree-sitter/tree-sitter) has already solved this problem.
