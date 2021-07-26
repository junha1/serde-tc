use crate::args::MacroArgs;
use proc_macro2::TokenStream as TokenStream2;

pub(super) fn generate_fallible_trait(
    source_trait: &syn::ItemTrait,
    args: &MacroArgs,
) -> Result<TokenStream2, TokenStream2> {
    if let Some(error_type) = args.fallible.clone() {
        let mut source_trait = source_trait.clone();
        source_trait.ident = format_ident!("{}Fallible", source_trait.ident);
        for item in source_trait.items.iter_mut() {
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

            match method.sig.output.clone() {
                syn::ReturnType::Default => {
                    let ok_type: syn::Type = syn::parse2(quote! {()}).unwrap();
                    method.sig.output =
                        syn::parse2(quote! {-> Result<#ok_type, #error_type>}).unwrap();
                }
                syn::ReturnType::Type(_, ok_type) => {
                    method.sig.output =
                        syn::parse2(quote! {-> Result<#ok_type, #error_type>}).unwrap();
                }
            };
        }
        Ok(quote! {
            #source_trait
        })
    } else {
        Ok(quote! {})
    }
}
