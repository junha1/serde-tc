use proc_macro2::{Span, TokenStream as TokenStream2};

pub(super) fn generate_dispatcher(
    source_trait: &syn::ItemTrait,
) -> Result<TokenStream2, TokenStream2> {
    let trait_ident = source_trait.ident.clone();

    let mut if_else_clauses = TokenStream2::new();

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

            let arg_type = match arg_source {
                syn::FnArg::Typed(syn::PatType {
                    attrs: _,
                    pat: _,
                    colon_token: _,
                    ty: t,
                }) => &**t,
                _ => panic!(),
            };

            if let Some(unrefed_type) = crate::helper::is_ref(arg_type)
                .map_err(|e| syn::Error::new_spanned(arg_source, &e).to_compile_error())?
            {
                type_annotation.elems.push(unrefed_type);
            } else {
                type_annotation.elems.push(arg_type.clone());
            }

            type_annotation
                .elems
                .push_punct(syn::token::Comma(Span::call_site()));

            let arg_ident = quote::format_ident!("a{}", j + 1);
            let the_arg = if crate::helper::is_ref(arg_type)
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
        let stmt_deserialize = quote! {
            let #let_pattern: #type_annotation = serde_json::from_str(args)?;
        };
        let method_name = method.sig.ident.clone();
        let stmt_call = quote! {
            let result = self.#method_name(#args_applying);
        };
        let the_return = quote! {
            return serde_json::to_string(&result)?;
        };

        if_else_clauses.extend(quote! {
            if method == #id_ident.load(#env_path::ID_ORDERING) {
                #stmt_deserialize
                #stmt_call
                #the_return
            }
        });
    }

    Ok(quote! {
        impl serde_tc::CallByString for dyn #trait_ident {
            type Error = serde_json::Error;
            fn call_dict(&self, arguments: &str) -> Result<String, Self::Error> {
                #stmt_deserialize
                #stmt_call
                #the_return
            }
            fn call_tuple(&self, arguments: &str) -> Result<String, Self::Error> {

            }
        }
        impl serde_tc::CallByStringAsync for dyn #trait_ident {
            async fn call_dict(&self, arguments: &str) -> Result<String, Self::Error> {

            }
            async fn call_tuple(&self, arguments: &str) -> Result<String, Self::Error> {

            }
        }
    })
}
