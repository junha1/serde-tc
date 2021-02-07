use crate::args::MacroArgs;
use heck::CamelCase;
use proc_macro2::{Span, TokenStream as TokenStream2};

pub(super) fn generate_dispatcher(
    source_trait: &syn::ItemTrait,
    args: &MacroArgs,
) -> Result<TokenStream2, TokenStream2> {
    let trait_ident = source_trait.ident.clone();
    let serde_format = args.serde_format.clone();

    let mut if_else_clauses_tuple = TokenStream2::new();
    let mut if_else_clauses_dict = TokenStream2::new();

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

        // Dict case
        let mut stmt_deserialize_dict = quote! {};

        // Tuple case
        let mut let_pattern = syn::PatTuple {
            attrs: Vec::new(),
            paren_token: syn::token::Paren(Span::call_site()),
            elems: syn::punctuated::Punctuated::new(),
        };
        // Annotation for the tuple
        let mut type_annotation = syn::TypeTuple {
            paren_token: syn::token::Paren(Span::call_site()),
            elems: syn::punctuated::Punctuated::new(),
        };
        // Applying arguments
        let mut args_applying: syn::punctuated::Punctuated<syn::Expr, syn::token::Comma> =
            syn::punctuated::Punctuated::new();

        for (j, arg_source) in method.sig.inputs.iter().skip(1).enumerate() {
            let the_iden = quote::format_ident!("a{}", j + 1);
            let (arg_type, arg_name) = match arg_source {
                syn::FnArg::Typed(syn::PatType {
                    attrs: _,
                    pat: name,
                    colon_token: _,
                    ty: t,
                }) => (&**t, name),
                _ => panic!("Method has a paramter pattern that is not supported"),
            };
            let mut arg_name = match *arg_name.clone() {
                syn::Pat::Ident(name) => name.ident,
                _ => panic!("Method has a paramter pattern that is not supported"),
            };
            if args.camel_case {
                arg_name = quote::format_ident!("{}", arg_name.to_string().to_camel_case());
            }
            let arg_type = if let Some(unrefed_type) = crate::helper::is_ref(arg_type)
                .map_err(|e| syn::Error::new_spanned(arg_source, &e).to_compile_error())?
            {
                unrefed_type
            } else {
                arg_type.clone()
            };
            type_annotation.elems.push(arg_type.clone());

            // Dict case
            let arg_name_lit =
                syn::LitStr::new(&arg_name.to_string(), proc_macro2::Span::call_site());
            stmt_deserialize_dict.extend(quote! {
                let #the_iden: #arg_type = arguments.get(#arg_name_lit).ok_or(format!("argument not found: `{}`", #arg_name))?;
            });

            // Tuple case
            let_pattern.elems.push(syn::Pat::Ident(syn::PatIdent {
                attrs: Vec::new(),
                by_ref: None,
                mutability: None,
                ident: the_iden,
                subpat: None,
            }));
            let_pattern
                .elems
                .push_punct(syn::token::Comma(Span::call_site()));

            type_annotation
                .elems
                .push_punct(syn::token::Comma(Span::call_site()));

            let arg_ident = quote::format_ident!("a{}", j + 1);
            let the_arg = if crate::helper::is_ref(&arg_type)
                .map_err(|e| syn::Error::new_spanned(arg_source, &e).to_compile_error())?
                .is_some()
            {
                quote! {
                    &#arg_ident
                }
            } else {
                quote! {
                    #arg_ident
                }
            };
            args_applying.push(syn::parse2(the_arg).unwrap());
        }
        let stmt_deserialize_tuple = quote! {
            let #let_pattern: #type_annotation = #serde_format::from_str(arguments)?;
        };
        let mut method_name = method.sig.ident.clone();
        if args.camel_case {
            method_name = quote::format_ident!("{}", method_name.to_string().to_camel_case());
        }
        let method_name_lit =
            syn::LitStr::new(&method_name.to_string(), proc_macro2::Span::call_site());

        let stmt_call = if args.async_methods {
            quote! {
                let result = self.#method_name(#args_applying).await;
            }
        } else {
            quote! {
                let result = self.#method_name(#args_applying);
            }
        };

        let the_return = quote! {
            return Ok(#serde_format::to_string(&result).unwrap());
        };

        if_else_clauses_tuple.extend(quote! {
            if method == #method_name_lit {
                #stmt_deserialize_tuple
                #stmt_call
                #the_return
            }
        });
        if_else_clauses_dict.extend(quote! {
            if method == #method_name_lit {
                #stmt_deserialize_dict
                #stmt_call
                #the_return
            }
        });
    }

    let (trait_name_tuple, trait_name_dict) = if args.async_methods {
        (
            quote! {serde_tc::DispatchStringTuple},
            quote! {serde_tc::DispatchStringDict},
        )
    } else {
        (
            quote! {serde_tc::DispatchStringTupleAsync},
            quote! {serde_tc::DispatchStringDictAsync},
        )
    };

    Ok(quote! {
        impl #trait_name_tuple for dyn #trait_ident {
            type Error = #serde_format::Error;
            fn dispatch(&self, method: &str, arguments: &str) -> Result<String, Self::Error> {
                #if_else_clauses_tuple
            }
        }
        impl #trait_name_dict for dyn #trait_ident {
            type Error = #serde_format::Error;
            type Poly = #serde_format::Value;
            fn dispatch(&self, method: &str, arguments: &HashMap<String, Self::Poly>) -> Result<String, DictError<Self::Error>> {
                #if_else_clauses_dict
            }
        }
    })
}
