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
pub struct MacroArgsRaw {
    pub serde_format: Option<syn::Path>,
    pub camel_case: Option<()>,
    pub async_methods: Option<()>,
    pub encoder: Option<()>,
    pub dispatcher: Option<()>,
    pub tuple: Option<()>,
    pub dict: Option<()>,
}

#[derive(Debug)]
pub struct MacroArgs {
    pub serde_format: syn::Path,
    pub camel_case: bool,
    pub async_methods: bool,
    pub encoder: bool,
    pub dispatcher: bool,
    pub tuple: bool,
    pub dict: bool,
}

impl MacroArgsRaw {
    pub fn update(&mut self, ts: TokenStream2) -> syn::parse::Result<()> {
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
            } else if arg == quote::format_ident!("encoder") {
                if self.encoder.replace(()).is_some() {
                    Err(syn::parse::Error::new_spanned(ts, "Duplicated arguments"))
                } else {
                    Ok(())
                }
            } else if arg == quote::format_ident!("dispatcher") {
                if self.dispatcher.replace(()).is_some() {
                    Err(syn::parse::Error::new_spanned(ts, "Duplicated arguments"))
                } else {
                    Ok(())
                }
            } else if arg == quote::format_ident!("tuple") {
                if self.tuple.replace(()).is_some() {
                    Err(syn::parse::Error::new_spanned(ts, "Duplicated arguments"))
                } else {
                    Ok(())
                }
            } else if arg == quote::format_ident!("dict") {
                if self.dict.replace(()).is_some() {
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

    pub fn fill_default_values(self) -> MacroArgs {
        MacroArgs {
            serde_format: self
                .serde_format
                .unwrap_or_else(|| syn::parse2(quote! {serde_json}).unwrap()),
            camel_case: self.camel_case.map(|_| true).unwrap_or(false),
            async_methods: self.async_methods.map(|_| true).unwrap_or(false),
            dispatcher: self.dispatcher.map(|_| true).unwrap_or(false),
            encoder: self.encoder.map(|_| true).unwrap_or(false),
            tuple: self.tuple.map(|_| true).unwrap_or(false),
            dict: self.dict.map(|_| true).unwrap_or(false),
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
