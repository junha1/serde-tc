use core::panic;

use crate::args::MacroArgs;
use heck::SnakeCase;
use proc_macro2::{Span, TokenStream as TokenStream2};

pub(super) fn generate_stub(
    source_trait: &syn::ItemTrait,
    source_fallable_trait: &syn::ItemTrait,
    args: &MacroArgs,
) -> Result<TokenStream2, TokenStream2> {
    let error_type = args.fallible.clone().ok_or_else(|| {
        syn::Error::new(Span::call_site(), "You must set fallible to use stub").to_compile_error()
    })?;
    let struct_name = quote::format_ident!("{}Stub", source_trait.ident.to_string());
    let serde_format = args.serde_format.clone();
    let trait_ident = source_fallable_trait.ident.clone();

    let mut trait_impl: syn::ItemImpl = syn::parse2(quote! {
        impl #trait_ident for #struct_name {
        }
    })
    .unwrap();

    for item in source_fallable_trait.items.iter() {
        let method = match item {
            syn::TraitItem::Method(x) => x,
            non_method => {
                return Err(syn::Error::new_spanned(
                    non_method,
                    "Service trait must have only methods",
                )
                .to_compile_error())
            }
        };

        let lit_method_name = syn::LitStr::new(&format!("{}", method.sig.ident), Span::call_site());
        let encoder_module_name = quote::format_ident!(
            "{}_encoder_dict",
            source_trait.ident.to_string().to_snake_case()
        );

        let mut args = syn::punctuated::Punctuated::<syn::Expr, syn::token::Comma>::new();
        for arg_source in method.sig.inputs.iter().skip(1) {
            let arg_name = match arg_source {
                syn::FnArg::Typed(syn::PatType {
                    attrs: _,
                    pat: name,
                    colon_token: _,
                    ty: _,
                }) => name,
                _ => panic!("Method has a paramter pattern that is not supported"),
            };
            let arg_name = match *arg_name.clone() {
                syn::Pat::Ident(name) => name.ident,
                _ => panic!("Method has a paramter pattern that is not supported"),
            };
            args.push(syn::parse2(quote! {#arg_name}).unwrap());
        }

        let method_ident = method.sig.ident.clone();

        trait_impl.items.push(syn::ImplItem::Method(syn::ImplItemMethod {
            attrs: Vec::new(),
            vis: syn::Visibility::Inherited,
            defaultness: None,
            sig: method.sig.clone(),
            block: syn::parse2(quote! {
                {
                    let msg = self.call.call(#lit_method_name, #encoder_module_name:: #method_ident (#args)).await?;
                    Ok(#serde_format::from_str(&msg)?)
                }
            }).unwrap(),
        }));
    }

    Ok(quote! {
        pub struct #struct_name {
            call: Box<dyn StubCall<Error = #error_type>>
        }

        impl Stub for #struct_name {
            type ClientTrait = dyn #trait_ident;
            fn new<T: StubCall>(sc: T) -> Self {
                Self {call: Box::new(sc)}
            }
            fn as_remote_object(this: Arc<Self>) -> Arc<Self::ClientTrait> {
                this as Arc<Self::ClientTrait>
            }
        }

        #[async_trait::async_trait]
        #trait_impl
    })
}
