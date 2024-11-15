use proc_macro2::TokenStream;
use syn::{Ident, Type};

pub fn stream(
    node: &Ident,
    node_response: &Ident,
    fields: &Vec<Ident>,
    types: &Vec<Type>,
    derives: &Vec<TokenStream>,
    conversions: &Vec<TokenStream>
) -> TokenStream {
    quote::quote! {
        // Create response struct
        #[derive(Default, Debug, Clone, PartialEq)]
        #[derive(Serialize, Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct #node_response {
            #(
                #derives
                pub #fields: nulls::Null<#types>,
            )*
        }

        impl actix_web::Responder for #node {
            type Body = actix_web::body::BoxBody;

            fn respond_to(self, _req: &actix_web::HttpRequest) -> actix_web::HttpResponse {
                actix_web::HttpResponse::Ok().json(serde_json::json!({
                    "code": 200,
                    "data": self.to::<#node_response>()
                }))
            }
        }

        impl #node {
            pub fn as_response(&self) -> actix_web::Result<actix_web::HttpResponse> {
                self.to::<#node_response>().as_response()
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

        // Response implementations
        impl #node_response {
            pub fn is_empty(&self) -> bool {
                *self == Self::default()
            }

            pub fn to<T: From<Self>>(&self) -> T {
                T::from(self.clone())
            }

            pub fn to_json(&self) -> sqlx::types::Json<Self> {
                sqlx::types::Json::from(self.clone())
            }

             pub fn as_response(&self) -> actix_web::Result<actix_web::HttpResponse> {
                Ok(actix_web::HttpResponse::Ok().json(serde_json::json!({
                    "code": 200,
                    "data": self
                })))
            }
        }

        impl From<#node> for #node_response {
            fn from(value: #node) -> #node_response {
                let mut data = #node_response::default();

                #(#conversions)*

                data
            }
        }

        impl From<#node_response> for actix_web::Result<actix_web::HttpResponse> {
            fn from(value: #node_response) -> actix_web::Result<actix_web::HttpResponse> {
                value.as_response()
            }
        }

        impl From<&#node_response> for actix_web::Result<actix_web::HttpResponse> {
            fn from(value: &#node_response) -> actix_web::Result<actix_web::HttpResponse> {
                value.as_response()
            }
        }

        impl actix_web::Responder for #node_response {
            type Body = actix_web::body::BoxBody;

            fn respond_to(self, _req: &actix_web::HttpRequest) -> actix_web::HttpResponse {
                actix_web::HttpResponse::Ok().json(self)
            }
        }
    }
}