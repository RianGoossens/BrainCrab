# 🧠🦀 BrainCrab
A Brainfuck CLI. My long term goal is to turn this into a toolchain for a programming language that compiles down to Brainfuck.

## Features
- Run .bf files
- A Brainfuck repl

## TODO
Here are my upcoming tasks.
- Refine repl, perhaps use crossterm or even ratatui?

### Braincrab Language TODO
- Rewrite examples in main to actual examples
- Multiplication
- Division
- Modulo
- Types? (u8, bool, structs, arrays...)
- Macros
- Const vs mut
- Make Write work with expressions
- Make read work as an expression
- Named constant optimization
- Comments
- Compiler flags
  - debug could keep comments or even code inside the .bf file
  - optimized could do the address optimization and others
- Improve Cli code

#### Parser Todo
- macros?