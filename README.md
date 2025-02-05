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
- Better memory management
  - Mark edited memory as dirty
  - Zeroing memory makes it clean
  - Have a special function to explicitly mark memory as zero
  - Do not clean up when freeing
  - When allocating memory, check if it's dirty, and zero if so.

### Braincrab Language TODO
- Types? (u8, bool, structs, arrays...)
  - tuples/arrays
  - structs
- Array functionality:
  - init with strings
  - foreach on mutable arrays
- Macros
- Make read work as an expression
- Comments
- Modules

### New ABF TODO
- Document the concept somewhere
- Use ABFState in both `ABFOptimizer` and `ABFCompiler`
- Rewrite `BrainCrabCompiler`
  - number_testing.bc before: 4kb

## Bugs
- Filter parser should somehow prioritize it's errors from the inner parser