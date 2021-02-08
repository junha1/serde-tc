use crate::args::MacroArgs;
use heck::{CamelCase, SnakeCase};
use proc_macro2::TokenStream as TokenStream2;

pub(super) fn generate_encoder(
    source_trait: &syn::ItemTrait,
    args: &MacroArgs,
) -> Result<TokenStream2, TokenStream2> {
    let mut functions_dict = TokenStream2::new();
    let mut functions_tuple = TokenStream2::new();
    let serde_format = args.serde_format.clone();

    for item in source_trait.items.iter() {
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

        let mut args_in_tuple: syn::ExprTuple = syn::parse2(quote! {()}).unwrap();
        let mut args_in_dict = quote! {let mut dict: std::collections::HashMap<String, #serde_format::Value> = Default::default();};
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
            let arg_name_for_lit = if args.camel_case {
                quote::format_ident!("{}", arg_name.to_string().to_camel_case())
            } else {
                arg_name.clone()
            };
            let arg_name_lit = syn::LitStr::new(
                &arg_name_for_lit.to_string(),
                proc_macro2::Span::call_site(),
            );

            args_in_dict.extend(quote! {
                dict.insert(#arg_name_lit.to_owned(), #serde_format::to_value(&#arg_name).unwrap());
            });
            args_in_tuple
                .elems
                .push(syn::parse2(quote! {#arg_name}).unwrap());
        }

        let mut the_fn: syn::ItemFn = syn::parse2(quote! {pub fn f() -> String {}}).unwrap();
        the_fn.sig.ident = method.sig.ident.clone();

        // remove &self
        let inputs = method.sig.inputs.iter().cloned().skip(1).collect();
        the_fn.sig.inputs = inputs;

        the_fn.block = syn::parse2(quote! {{
            #args_in_dict
            #serde_format::to_string(&dict).unwrap()
        }})
        .unwrap();
        functions_dict.extend(quote! {#the_fn});

        the_fn.block = syn::parse2(quote! {{
            #serde_format::to_string(&#args_in_tuple).unwrap()
        }})
        .unwrap();
        functions_tuple.extend(quote! {#the_fn});
    }

    let mut modules = quote! {};
    if args.dict {
        let module_name = quote::format_ident!(
            "{}_encoder_dict",
            source_trait.ident.to_string().to_snake_case()
        );
        modules.extend(quote! {
            #[allow(unused_mut)]
            pub mod #module_name {
                #functions_dict
            }
        });
    }
    if args.tuple {
        let module_name = quote::format_ident!(
            "{}_encoder_tuple",
            source_trait.ident.to_string().to_snake_case()
        );
        modules.extend(quote! {
            pub mod #module_name {
                #functions_tuple
            }
        });
    }
    Ok(modules)
}
