use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;
use syn::parse_macro_input;

mod resource;

#[proc_macro_error]
#[proc_macro_derive(Resource, attributes(actions, specifier))]
pub fn resource(input: TokenStream) -> TokenStream {
    resource::derive_resource(parse_macro_input!(input))
}