use deluxe::Result;
use quote::format_ident;
use proc_macro2::{Ident, TokenStream};
use syn::{Data, DeriveInput, LitBool, LitStr, Type};

// Set Attr struct
#[derive(deluxe::ExtractAttributes)]
#[deluxe(attributes(form))]
struct Attrs {
    pub refs: Option<Ident>,
    pub sanitize: Option<LitStr>,
    pub error: Option<Type>,
    pub skip_refs: Option<LitBool>
}

// Start of derive and field attribute derives
#[proc_macro_derive(Form, attributes(form, reference))]
pub fn main(stream: proc_macro::TokenStream) -> proc_macro::TokenStream {
    derive(stream.into()).unwrap().into()
}

// Start of derive and token processing
fn derive(stream: TokenStream) -> Result<TokenStream> {
    // Parse token stream
    let ast: DeriveInput = syn::parse2(stream)?;
    let node = &ast.ident.clone();

    // Create main token stream
    let mut token = quote::quote!{};

    // Create error & response node
    let node_error = format_ident!("{}Error", node.to_string().replace("Form", ""));

    // Retrieve node reference
    let Attrs { refs, .. } = deluxe::extract_attributes(&mut ast.clone())?;
    let node_reference = refs;

    let mut sanitizers = vec![];
    let mut fields = vec![];
    let mut ref_fields = vec![];
    let mut error_derives = vec![];
    let mut error_fields = vec![];
    let mut error_types = vec![];
    let mut cloned_fields = vec![];

    if let Data::Struct(s) = &mut ast.data.clone() {
        for f in s.fields.iter_mut() {
            let field_type = f.ty.clone();

            if let Ok(attrs) = deluxe::extract_attributes::<syn::Field, Attrs>(f) {
                let field = f.ident.as_ref().unwrap().clone();

                // Save fields
                fields.push(field.clone());

                if !(attrs.skip_refs.is_some() && attrs.skip_refs.clone().unwrap().value) {
                    ref_fields.push(field.clone());
                }

                // Set sanitizers
                if let Some(attr) = attrs.sanitize {
                    match attr.value().as_str() {
                        "lowercase" => sanitizers.push(quote::quote! {
                            if let Null::Value(value) = data.#field.clone() {
                                if !value.is_empty() {
                                    data.#field = Null::Value(value.to_string().trim().to_lowercase().to_string());
                                }
                            }
                        }),
                        "normalize_name" => sanitizers.push(quote::quote! {
                            if let Null::Value(value) = data.#field.clone() {
                                let value = value.trim();

                                if !value.is_empty() {
                                    let mut name_vector = Vec::new();
                                    let name_split = value.split(' ');

                                    for row in name_split {
                                        let item = titlecase::titlecase(row);

                                        match item.as_str() {
                                            "." => name_vector.push("".to_string()),
                                            "Jr." => name_vector.push("Jr".to_string()),
                                            "Sr." => name_vector.push("Sr".to_string()),
                                            "I" => name_vector.push("I".to_string()),
                                            "Ii" => name_vector.push("II".to_string()),
                                            "Iii" => name_vector.push("III".to_string()),
                                            "Iv" => name_vector.push("IV".to_string()),
                                            "V" => name_vector.push("V".to_string()),
                                            "Vi" => name_vector.push("VI".to_string()),
                                            "Vii" => name_vector.push("VII".to_string()),
                                            "Viii" => name_vector.push("VIII".to_string()),
                                            "Ix" => name_vector.push("IX".to_string()),
                                            "X" => name_vector.push("X".to_string()),
                                            "Xi" => name_vector.push("XI".to_string()),
                                            "Xii" => name_vector.push("XII".to_string()),
                                            "Xiii" => name_vector.push("XIII".to_string()),
                                            "Xiv" => name_vector.push("XIV".to_string()),
                                            "Xv" => name_vector.push("XV".to_string()),
                                            "Xvi" => name_vector.push("XVI".to_string()),
                                            "Xvii" => name_vector.push("XVII".to_string()),
                                            "Xviii" => name_vector.push("XVIII".to_string()),
                                            "Xix" => name_vector.push("XIX".to_string()),
                                            "Xx" => name_vector.push("XX".to_string()),
                                            s => name_vector.push(s.to_string()),
                                        }
                                    }

                                    data.#field = Null::Value(name_vector.clone().join(" "));
                                }
                            }
                        }),
                        "trim" => sanitizers.push(quote::quote! {
                            if let Null::Value(value) = data.#field.clone() {
                                if !value.is_empty() {
                                    data.#field = Null::Value(value.to_string().trim().to_string());
                                }
                            }
                        }),
                        "trim_slash" => sanitizers.push(quote::quote! {
                            if let Null::Value(value) = data.#field.clone() {
                                if !value.is_empty() {
                                    data.#field = Null::Value(value
                                        .to_string()
                                        .trim()
                                        .trim_end_matches('/')
                                        .trim()
                                        .to_string());
                                }
                            }
                        }),
                        _ => {}
                    }
                }

                // Set errors
                error_fields.push(field.clone());
                error_types.push(match () {
                    _ if attrs.error.is_some() => attrs.error.unwrap(),
                    _ => field_type.clone()
                });

                error_derives.push(quote::quote! {
                    #[serde(skip_serializing_if = "Null::is_undefined")]
                });

                let cloned_field = format_ident!("clone_{}", field);
                cloned_fields.push(quote::quote!{
                    pub fn #cloned_field(&self, value: &#field_type) -> Self {
                        let mut data = self.clone();

                        data.#field = value.clone();

                        data
                    }
                });
            }
        }
    }

    token.extend(quote::quote! {
        impl #node {
            pub fn is_empty(&self) -> bool {
                *self == Self::default()
            }

            pub fn to<T: From<Self>>(&self) -> T {
                T::from(self.clone())
            }

            pub fn to_error(&self) -> #node_error {
                #node_error::default()
            }

            pub fn to_json(&self) -> sqlx::types::Json<Self> {
                sqlx::types::Json::from(self.clone())
            }

            pub fn sanitize(&self) -> Self {
                let mut data = self.clone();

                #(#sanitizers)*

                data
            }

            #(#cloned_fields)*
        }

        #[derive(Debug, Clone, Default, PartialEq)]
        #[derive(Serialize, Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct #node_error {
            #(
                #error_derives
                pub #error_fields: #error_types,
            )*
        }

        // Error implementations
        impl #node_error {
            pub fn is_empty(&self) -> bool {
                *self == Self::default()
            }

            pub fn validate(&self) -> errors::Result<()> {
                if self.is_empty() {
                    return Ok(())
                }

                Err(errors::to(self))
            }
        }

        impl actix_web::Responder for #node_error {
            type Body = actix_web::body::BoxBody;

            fn respond_to(self, _req: &actix_web::HttpRequest) -> actix_web::HttpResponse {
                actix_web::HttpResponse::Ok().json(self)
            }
        }
    });

    // Check if reference exists
    if let Some(refs) = node_reference {
        token.extend(quote::quote! {
            impl From<#node> for #refs {
                fn from(value: #node) -> Self {
                    let mut data = Self::default();

                    #(
                        data.#ref_fields = value.#ref_fields.clone();
                    )*

                    data
                }
            }

            impl From<#refs> for #node {
                fn from(value: #refs) -> Self {
                    let mut data = Self::default();

                    #(
                        data.#ref_fields = value.#ref_fields.clone();
                    )*

                    data
                }
            }
        });
    }

    // Return the new token
    Ok(token)
}