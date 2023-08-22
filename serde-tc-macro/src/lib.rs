#[macro_use]
extern crate quote;

mod args;
mod augment;
mod dispatcher;
mod encoder;
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
pub fn serde_tc_full(_args: TokenStream, input: TokenStream) -> TokenStream {
    match expand(
        quote! {dispatcher, encoder, dict, tuple, async_methods, fallible = eyre::Error, caller_speicifed = hdk_common::crypto::PublicKey, stub},
        TokenStream2::from(input),
    ) {
        Ok(x) => TokenStream::from(x),
        Err(x) => TokenStream::from(x),
    }
}

#[proc_macro_attribute]
pub fn serde_tc_debug(_args: TokenStream, input: TokenStream) -> TokenStream {
    match expand(
        quote! {dispatcher, encoder, dict, tuple, async_methods, fallible = eyre::Error, caller_speicifed = hdk_common::crypto::PublicKey, stub},
        TokenStream2::from(input),
    ) {
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

    if let (Some(key_type), Some(error_type)) =
        (args.caller_speicifed.clone(), args.fallible.clone())
    {
        let server_trait = augment::generate_caller_specified_trait(&source_trait, &key_type)?;
        let client_trait = augment::generate_fallible_trait(&source_trait, &error_type)?;

        let dispatcher1 = dispatcher::generate_dispatcher(&source_trait, &args)?;
        let dispatcher2 = dispatcher::generate_dispatcher(&server_trait, &args)?;
        let encoder = encoder::generate_encoder(&source_trait, &args)?;

        let stub = if args.stub {
            stub::generate_stub(
                &source_trait,
                &syn::parse2(quote! {#client_trait}).unwrap(),
                &args,
            )?
        } else {
            quote! {}
        };
        let trait_ident = source_trait.ident.clone();
        let trait_ident_server = server_trait.ident.clone();
        if args.async_methods {
            Ok(quote! {
                #[async_trait::async_trait]
                #source_trait
                #[async_trait::async_trait]
                #server_trait
                #[async_trait::async_trait]
                #client_trait
                #dispatcher1
                #dispatcher2
                #encoder
                #stub
                impl HttpInterface for dyn #trait_ident {}
                impl HttpInterface for dyn #trait_ident_server {}
            })
        } else {
            Ok(quote! {
                #source_trait
                #server_trait
                #client_trait
                #dispatcher1
                #dispatcher2
                #encoder
                #stub
            })
        }
    } else {
        return Err(syn::Error::new_spanned(
            input,
            "You must specify both `caller_specified` and `fallible`. Otherwise, currently unimplemented.",
        )
        .to_compile_error());
    }
}
