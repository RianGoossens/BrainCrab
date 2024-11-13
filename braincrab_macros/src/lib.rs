use braincrab_core::parse_bf;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, LitStr};

#[proc_macro]
pub fn bf(input: TokenStream) -> TokenStream {
    // Parse the input as a string literal
    let input = parse_macro_input!(input as LitStr);
    let brainfuck_code = input.value();

    let compiled_program = parse_bf(&brainfuck_code).expect("Not a valid Brainfuck program");

    // Generate the tokens for returning an instance of `BFProgram`
    let output = quote! {
        #compiled_program
    };

    output.into()
}
