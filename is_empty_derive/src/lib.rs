use proc_macro::TokenStream;
use syn::DeriveInput;

// Entry point for our macro
#[proc_macro_derive(IsEmpty)]
pub fn main(stream: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(stream).unwrap();
    let node = ast.ident;

    TokenStream::from(quote::quote! {
        impl #node {
            pub fn is_empty(&self) -> bool {
                *self == Self::default()
            }
        }
    })
}