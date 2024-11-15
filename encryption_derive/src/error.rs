use proc_macro2::TokenStream;
use syn::{Ident, Type};

pub fn stream(
    node: &Ident,
    node_form: &Ident,
    fields: &Vec<Ident>,
    types: &Vec<Type>,
    derives: &Vec<TokenStream>
) -> TokenStream {
    quote::quote! {
        #[derive(Debug, Clone, Default, PartialEq)]
        #[derive(Serialize, Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct #node {
            #(
                #derives
                pub #fields: nulls::Null<#types>,
            )*
        }

        impl std::fmt::Display for #node {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "{:#?}", self)
            }
        }

        // Set form node implementation
        impl #node_form {
            pub fn to_error(&self) -> #node {
                #node::default()
            }
        }

        // Error implementations
        impl #node {
            pub fn is_empty(&self) -> bool {
                *self == Self::default()
            }

            pub fn validate(&self) -> errors::Result<()> {
                if self.is_empty() {
                    return Ok(())
                }

                Err(errors::to(self))
            }

             pub fn as_response(&self) -> actix_web::Result<actix_web::HttpResponse> {
                Ok(actix_web::HttpResponse::Ok().json(serde_json::json!({
                    "code": 200,
                    "data": self
                })))
            }
        }

        impl From<#node> for actix_web::Result<actix_web::HttpResponse> {
            fn from(value: #node) -> actix_web::Result<actix_web::HttpResponse> {
                value.as_response()
            }
        }

        impl From<&#node> for actix_web::Result<actix_web::HttpResponse> {
            fn from(value: &#node) -> actix_web::Result<actix_web::HttpResponse> {
                value.as_response()
            }
        }

         impl actix_web::Responder for #node {
            type Body = actix_web::body::BoxBody;

            fn respond_to(self, _req: &actix_web::HttpRequest) -> actix_web::HttpResponse {
                actix_web::HttpResponse::Ok().json(self)
            }
        }
    }
}