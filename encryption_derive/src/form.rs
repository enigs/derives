use proc_macro2::TokenStream;
use syn::{Ident, Type};

pub fn stream(
    node: &Ident,
    node_form: &Ident,
    members: (&Vec<Ident>, &Vec<Type>, &Vec<TokenStream>),
    conversions: (&Vec<TokenStream>, &Vec<TokenStream>),
    sanitizers: &Vec<TokenStream>
) -> TokenStream {
    let fields = members.0;
    let types = members.1;
    let derives = members.2;

    let conversions_to = conversions.0;
    let conversions_from = conversions.1;

    quote::quote! {
        #[derive(Debug, Clone, Default, PartialEq)]
        #[derive(Serialize, Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct #node_form {
            #(
                #derives
                pub #fields: Null<#types>,
            )*
        }

        // Form implementations
        impl #node_form {
            pub fn is_empty(&self) -> bool {
                *self == Self::default()
            }

            pub fn to<T: From<Self>>(&self) -> T {
                T::from(self.clone())
            }

            pub fn to_json(&self) -> sqlx::types::Json<Self> {
                sqlx::types::Json::from(self.clone())
            }

            pub fn sanitize(&self) -> Self {
                let mut data = self.clone();

                #(#sanitizers)*

                data
            }
        }

        impl From<#node> for #node_form {
            fn from(value: #node) -> Self {
                let mut data = Self::default();

                #(#conversions_to)*

                data
            }
        }

        impl From<#node_form> for #node {
            fn from(value: #node_form) -> Self {
                let mut data = Self::default();

                #(#conversions_from)*

                data
            }
        }
    }
}