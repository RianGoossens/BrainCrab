# 🧠🦀 BrainCrab
A Brainfuck CLI. My long term goal is to turn this into a toolchain for a programming language that compiles down to Brainfuck.

## Features
- Run .bf files
- A Brainfuck repl

## TODO
Here are my upcoming tasks.
- Refine repl, perhaps use crossterm or even ratatui?

### Braincrab Language TODO
- Easier way to check if a value is the same as an address, and if it's temp
- Edge case for assigning to self (nop)
- Multiplication
- Division
- Modulo
- eq, neq, lt, gt, le, ge
- Logical And
- Logical Or
- Logical Not
- Expressions
- Types? (u8, bool, structs, arrays...)
- Rethink values and expressions
  - expressions combine other expressions
  - leaf values are the simplest expressions:
    - const: compile time known value which does not need an address
    - owned: address which is owned and will be released when out of scope 
      - These can do their own cleanup like temps currently do
      - Non-temp ones might need to be reference counted or something like that
    - borrowed: address which is borrowed and will not be released when out of scope
