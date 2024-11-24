# 🧠🦀 BrainCrab
A Brainfuck CLI. My long term goal is to turn this into a toolchain for a programming language that compiles down to Brainfuck.

## Features
- Run .bf files
- A Brainfuck repl

## TODO
Here are my upcoming tasks.
- Refine repl, perhaps use crossterm or even ratatui?

### Braincrab Language TODO
- Multiplication
- Division
- Modulo
- Types? (u8, bool, structs, arrays...)
- Macros
- Start reusing cli

#### Parser Todo
- Parse literal characters
- Parse variable names
- Parse assignments
- Parse add-assignment
- Parse sub-assignment
- Parse expressions
  - variables
  - unaries
  - binaries
  - order of operations!
  - parens