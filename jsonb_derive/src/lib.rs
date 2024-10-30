use proc_macro::TokenStream;
use syn::DeriveInput;

// Entry point for our macro
#[proc_macro_derive(Jsonb)]
pub fn main(stream: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(stream).unwrap();
    let node = ast.ident;

    TokenStream::from(quote::quote! {
        impl #node {
            pub fn is_empty(&self) -> bool {
                *self == Self::default()
            }

            pub fn to_json(&self) -> sqlx::types::Json<Self> {
                sqlx::types::Json::from(self.clone())
            }
        }
    })
}