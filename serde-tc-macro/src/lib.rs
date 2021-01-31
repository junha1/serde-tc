#[macro_use]
extern crate quote;

mod dispatcher;
mod helper;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;

#[proc_macro_attribute]
pub fn tc_forward(args: TokenStream, input: TokenStream) -> TokenStream {
    unimplemented!()
}
