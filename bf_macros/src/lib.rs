use bf_core::{parse_bf, BFProgram, BFTree};
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, LitStr};

fn bftree_to_tokens(tree: &BFTree, tokens: &mut TokenStream2) {
    let tokenized = match tree {
        BFTree::Move(offset) => quote! { BFTree::Move(#offset) },
        BFTree::Add(value) => quote! { BFTree::Add(#value) },
        BFTree::Write => quote! { BFTree::Write },
        BFTree::Read => quote! { BFTree::Read },
        BFTree::Loop(subtrees) => {
            let subtrees_tokens = subtrees.iter().map(|subtree| {
                let mut subtree_tokens = TokenStream2::new();
                bftree_to_tokens(subtree, &mut subtree_tokens);
                subtree_tokens
            });
            quote! { BFTree::Loop(vec![#(#subtrees_tokens),*]) }
        }
    };

    tokens.extend(tokenized);
}

fn bfprogram_to_tokens(program: &BFProgram, tokens: &mut TokenStream2) {
    let bftrees_tokens = program.0.iter().map(|tree| {
        let mut tree_tokens = TokenStream2::new();
        bftree_to_tokens(tree, &mut tree_tokens);
        tree_tokens
    });

    tokens.extend(quote! {
        BFProgram(vec![#(#bftrees_tokens),*])
    });
}

#[proc_macro]
pub fn bf(input: TokenStream) -> TokenStream {
    // Parse the input as a string literal
    let input = parse_macro_input!(input as LitStr);
    let brainfuck_code = input.value();

    let compiled_program = parse_bf(&brainfuck_code).expect("Not a valid Brainfuck program");

    // Generate the tokens for returning an instance of `BFProgram`
    let mut bfprogram_tokens = TokenStream2::new();
    bfprogram_to_tokens(&compiled_program, &mut bfprogram_tokens);
    let output = quote! {
        #bfprogram_tokens
    };

    output.into()
}
