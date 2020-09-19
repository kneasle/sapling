# Sapling
![Sapling logo](https://raw.githubusercontent.com/kneasle/sapling/master/sapling.gif)
A highly experimental code editor where you write code not text.

This project is an experiment to determine if an editor like this is actually useful.  If it is, then the sapling might grow into a tree.

## But why?
When writing code with any text editor, you are usually only interested in a tiny subset of valid strings of text - those that correspond to valid
programs in whatever language you're writing.  In a **text** editor, you will spend the overwhelming majority of your time
with the text in your editor being invalid as you make edits to move between valid programs.

This is incredibly inefficient.

To be fair, editors like Vim and Emacs do better than most by providing shortcuts to do common text manipulations which is a step in the right direction.

## Pros
- Because the editor already knows the syntactic structure of your program, you get the following essentially for free:
  - Syntax highlighting
  - Code folding
