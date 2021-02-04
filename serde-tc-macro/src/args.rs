use proc_macro2::TokenStream as TokenStream2;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::Token;

struct SingleArg<T: Parse> {
    pub arg_name: syn::Ident,
    pub arg_value: T,
}

impl<T: Parse> Parse for SingleArg<T> {
    fn parse(input: ParseStream) -> syn::parse::Result<Self> {
        let arg_name = input.parse()?;
        input.parse::<Token![=]>()?;
        let arg_value = input.parse()?;
        Ok(Self {
            arg_name,
            arg_value,
        })
    }
}

#[derive(Default)]
struct MacroArgsRaw {
    pub serde_format: Option<syn::Path>,
    pub camel_case: Option<()>,
    pub async_methods: Option<()>,
}

pub struct MacroArgs {
    pub serde_format: syn::Path,
    pub camel_case: bool,
    pub async_methods: bool,
}

impl MacroArgsRaw {
    fn update(&mut self, ts: TokenStream2) -> syn::parse::Result<()> {
        if let Ok(arg) = syn::parse2::<syn::Ident>(ts.clone()) {
            return if arg == quote::format_ident!("camel_case") {
                if self.camel_case.replace(()).is_some() {
                    Err(syn::parse::Error::new_spanned(ts, "Duplicated arguments"))
                } else {
                    Ok(())
                }
            } else if arg == quote::format_ident!("async_methods") {
                if self.async_methods.replace(()).is_some() {
                    Err(syn::parse::Error::new_spanned(ts, "Duplicated arguments"))
                } else {
                    Ok(())
                }
            } else {
                Err(syn::parse::Error::new_spanned(ts, "Unsupported argument"))
            };
        }

        let arg: SingleArg<TokenStream2> = syn::parse2(ts.clone())?;
        if arg.arg_name == quote::format_ident!("serde_format") {
            let value = syn::parse2(arg.arg_value)?;
            if self.serde_format.replace(value).is_some() {
                Err(syn::parse::Error::new_spanned(ts, "Duplicated arguments"))
            } else {
                Ok(())
            }
        } else {
            Err(syn::parse::Error::new_spanned(ts, "Unsupported argument"))
        }
    }

    fn fill_default_values(self) -> MacroArgs {
        MacroArgs {
            serde_format: self.serde_format.unwrap_or_else(|| {
                syn::parse2(quote! {serde_json}).unwrap()
            }),
            camel_case: self.camel_case.map(|_| true).unwrap_or(false),
            async_methods: self.async_methods.map(|_| true).unwrap_or(false),
        }
    }
}

impl Parse for MacroArgsRaw {
    fn parse(input: ParseStream) -> syn::parse::Result<Self> {
        let mut result = MacroArgsRaw::default();
        let args = Punctuated::<syn::Expr, Token![,]>::parse_terminated(input)?;
        for arg in args {
            result.update(quote! {#arg})?;
        }
        Ok(result)
    }
}

pub fn get_args(args: TokenStream2) -> Result<MacroArgs, TokenStream2> {
    let args: MacroArgsRaw = syn::parse2(args).map_err(|e| e.to_compile_error())?;
    Ok(args.fill_default_values())
}