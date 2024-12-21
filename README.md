# 🧠🦀 BrainCrab
A Brainfuck CLI. Toolchain for a programming language that compiles down to Brainfuck.

## Features
- Run BrainCrab (.bc) files
- Compile BrainCrab files to Brainfuck (.bf)
- Run .bf files
- A Brainfuck repl

## TODO
Here are my upcoming tasks.
- Refine repl, perhaps use crossterm or even ratatui?
- Improve Cli code
- Compiler flags
  - debug could keep comments or even code inside the .bf file
  - optimized could do the address optimization and others
- Better compiler errors by storing program pointers at parse time
- Language documentation

### Braincrab Language TODO
- Types? (u8, bool, structs, arrays...)
  - tuples/arrays
  - structs
- Array functionality:
  - init with strings
  - mutable arrays
  - dynamic indices
- Macros
- Make read work as an expression
- Comments
- Modules

## Bugs
- Filter parser should somehow prioritize it's errors from the inner parser