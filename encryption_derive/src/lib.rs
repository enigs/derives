mod cipher;
mod error;
mod extractors;
mod form;
mod parsers;
mod response;

use deluxe::Result;
use proc_macro2::TokenStream;
use quote::format_ident;
use syn::{parse_str, DeriveInput, Type};

// Start of derive and field attribute derives
#[proc_macro_derive(Encryption, attributes(encryption))]
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

    // Set all variables for response
    let mut fields = vec![];
    let mut types = vec![];
    let mut error_types = vec![];
    let mut derives = vec![];
    let mut props = vec![];
    let mut ciphered_fields = vec![];
    let mut conversions_to_response = vec![];
    let mut conversions_to_form = vec![];
    let mut conversions_from_form = vec![];

    // Parse all fields
    for (original_field, original_type) in parsers::all_fields(&ast.data) {
        fields.push(original_field.clone());
        derives.push(quote::quote! {
            #[serde(skip_serializing_if = "nulls::Null::is_undefined")]
        });

        let key = original_field.to_string();
        let attribute = "types";
        let defaults = original_type;
        let converted_type = extractors::get_dynamic_type(&key, attribute, defaults, &ast);
        let converted_type_string = extractors::type_to_string(&converted_type);

        types.push(converted_type.clone());

        if let Ok(error_defaults) = parse_str::<Type>("String") {
            let error_attribute = "errors";
            error_types.push(extractors::get_dynamic_type(&key, error_attribute, error_defaults.clone(), &ast));
        }

        let is_ciphered_fields = extractors::is_attributed_type(key, attribute, &ast);
        if is_ciphered_fields {
            ciphered_fields.push(original_field.clone());

            if converted_type_string.as_str() == "i32" {
                conversions_to_response.push(quote::quote! {
                    data.#original_field =  match value.#original_field.clone() {
                        nulls::Null::Undefined => nulls::Null::Undefined,
                        nulls::Null::Null => nulls::Null::Null,
                        nulls::Null::Value(data) => nulls::new(data.to_i32().unwrap_or(0))
                    };
                });

                conversions_to_form.push(quote::quote! {
                    data.#original_field =  match value.#original_field.clone() {
                        nulls::Null::Undefined => nulls::Null::Undefined,
                        nulls::Null::Null => nulls::Null::Null,
                        nulls::Null::Value(data) => nulls::new(data.to_i32().unwrap_or(0))
                    };
                });

                conversions_from_form.push(quote::quote! {
                    data.#original_field =  match value.#original_field.clone().take().unwrap_or(0) > 0 {
                        true => nulls::new(ciphers::new(value.#original_field.clone().take().unwrap_or(0))),
                        false => nulls::Null::Undefined
                    };
                });

                props.push(quote::quote! {
                    pub fn #original_field(&self) -> #converted_type {
                        self.decrypt()
                            .unwrap_or_default()
                            .#original_field
                            .take()
                            .unwrap_or_default()
                            .to_i32()
                            .unwrap_or(0)
                    }
                });
            }

            if converted_type_string.as_str() == "String" {
                conversions_to_response.push(quote::quote! {
                    data.#original_field =  match value.#original_field.clone() {
                        nulls::Null::Undefined => nulls::Null::Undefined,
                        nulls::Null::Null => nulls::Null::Null,
                        nulls::Null::Value(data) => match data.is_empty() {
                            true => nulls::Null::Undefined,
                            false => nulls::Null::Value(data.to_string())
                        }
                    };
                });

                conversions_to_form.push(quote::quote! {
                    data.#original_field =  match value.#original_field.clone() {
                        nulls::Null::Undefined => nulls::Null::Undefined,
                        nulls::Null::Null => nulls::Null::Null,
                        nulls::Null::Value(data) => match data.is_empty() {
                            true => nulls::Null::Undefined,
                            false => nulls::Null::Value(data.to_string())
                        }
                    };
                });

                conversions_from_form.push(quote::quote! {
                    data.#original_field =  match !value.#original_field.clone().take().unwrap_or_default().is_empty() {
                        true => nulls::Null::Value(ciphers::new(value.#original_field.clone().take().unwrap_or_default())),
                        false => nulls::Null::Undefined
                    };
                });

                props.push(quote::quote! {
                    pub fn #original_field(&self) -> #converted_type {
                        self.decrypt()
                            .unwrap_or_default()
                            .#original_field
                            .take()
                            .unwrap_or_default()
                            .to_string()
                    }
                });
            }
        } else {
            conversions_to_response.push(quote::quote!{
                 data.#original_field = value.#original_field.clone();
            });

            conversions_to_form.push(quote::quote!{
                 data.#original_field = value.#original_field.clone();
            });

            conversions_from_form.push(quote::quote! {
                data.#original_field =  value.#original_field.clone();
            });

            props.push(quote::quote! {
                pub fn #original_field(&self) -> #converted_type {
                    self.#original_field
                        .clone()
                        .take()
                        .unwrap_or_default()
                }
            });
        }
    }

    // Attach all important implementations for parent node struct
    token.extend(cipher::stream(node, &ciphered_fields, &props));

    // Stream token for response
    let node_response = format_ident!("{}Response", ast.ident);
    token.extend(response::stream(
        node, &node_response, &fields, &types,
        &derives, &conversions_to_response
    ));

    // Stream token for form
    let node_form = format_ident!("{}Form", ast.ident);
    let sanitizers = extractors::get_sanitize(&ast);
    let members = (&fields, &types, &derives);
    let conversions = (&conversions_to_form, &conversions_from_form);
    token.extend(form::stream(
        node, &node_form, members,
        conversions, &sanitizers
    ));

    // Stream token for error
    let node_error = format_ident!("{}Error", ast.ident);
    token.extend(error::stream(
        &node_error, &node_form, &fields,
        &error_types, &derives
    ));

    // Return the new token
    Ok(token)
}