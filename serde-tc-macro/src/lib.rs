#[macro_use]
extern crate quote;

mod args;
mod dispatcher;
mod encoder;
mod fallible;
mod helper;
mod stub;

use args::MacroArgsRaw;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;

#[proc_macro_attribute]
pub fn serde_tc(args: TokenStream, input: TokenStream) -> TokenStream {
    match expand(TokenStream2::from(args), TokenStream2::from(input)) {
        Ok(x) => TokenStream::from(x),
        Err(x) => TokenStream::from(x),
    }
}

#[proc_macro_attribute]
pub fn serde_tc_debug(args: TokenStream, input: TokenStream) -> TokenStream {
    match expand(TokenStream2::from(args), TokenStream2::from(input)) {
        Ok(x) => println!("{}", x),
        Err(x) => println!("{}", x),
    }
    TokenStream::new()
}

fn expand(args: TokenStream2, input: TokenStream2) -> Result<TokenStream2, TokenStream2> {
    let args: MacroArgsRaw = syn::parse2(args).map_err(|e| e.to_compile_error())?;
    let args = args.fill_default_values();

    let source_trait = match syn::parse2::<syn::ItemTrait>(input.clone()) {
        Ok(x) => x,
        Err(_) => {
            return Err(
                syn::Error::new_spanned(input, "You can use #[serde_tc] only on a trait")
                    .to_compile_error(),
            )
        }
    };

    let dispatcher = dispatcher::generate_dispatcher(&source_trait, &args)?;
    let encoder = encoder::generate_encoder(&source_trait, &args)?;
    let fallible = fallible::generate_fallible_trait(&source_trait, &args)?;
    let stub = if args.stub {
        stub::generate_stub(
            &source_trait,
            &syn::parse2(fallible.clone()).unwrap(),
            &args,
        )?
    } else {
        quote! {}
    };

    if args.async_methods {
        Ok(quote! {
            #[async_trait::async_trait]
            #source_trait
            #[async_trait::async_trait]
            #fallible
            #dispatcher
            #encoder
            #stub
        })
    } else {
        Ok(quote! {
            #source_trait
            #[async_trait::async_trait]
            #fallible
            #dispatcher
            #encoder
            #stub
        })
    }
}
