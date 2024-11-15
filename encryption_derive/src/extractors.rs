use regex::Regex;
use std::collections::HashMap;
use deluxe::extract_attributes;
use proc_macro2::TokenStream;
use syn::{DeriveInput, Data, Ident, Type, parse_str, LitStr};

// Set Attr struct
#[derive(deluxe::ExtractAttributes)]
#[deluxe(attributes(encryption))]
#[allow(dead_code)]
struct Attrs {
    types: Option<Type>,
    sanitize: Option<LitStr>,
    errors: Option<Type>
}

// Extract attributes
fn extract_attrs(ast: &mut DeriveInput) -> deluxe::Result<HashMap<Ident, Attrs>> {
    let mut attrs = HashMap::new();
    if let Data::Struct(s) = &mut ast.data {
        for field in s.fields.iter_mut() {
            if let Ok(attr) = extract_attributes(field) {
                attrs.insert(field.ident.as_ref().unwrap().clone(), attr);
            }
        }
    }

    Ok(attrs)
}

// Extract type from wrapper
pub fn type_from_wrapper<T>(input: T) -> Type
    where T: ToString
{
    let input = input.to_string();

    let re = Regex::new(r"^[^<]*<(.+)>$").unwrap();
    if let Some(captures) = re.captures(&input) {
        if let Some(captured) = captures.get(1) {
            if let Ok(ty) = parse_str::<Type>(captured.as_str()) {
                return ty;
            }
        }
    } else if let Ok(ty) = parse_str::<Type>(&input) {
        return ty;
    }

    panic!("Invalid type string");
}

// Create a syn::Type to String conversion
pub fn type_to_string(input: &Type) -> String {
    format!("{}", quote::quote! { #input })
        .replace(" ", "")
}

// Extract type from map
#[allow(dead_code)]
pub fn get_type<T>(key: T, map: &HashMap<String, Type>) -> Option<Type>
    where T: ToString
{
    let key = key.to_string();

    for (k, v) in map.clone() {
        if k == key {
            return Some(v);
        }
    }

    None
}

// Get type whether it was attributed or not
pub fn get_dynamic_type<T, U>(key: T, attribute: U, defaults: Type, derive_input: &DeriveInput) -> Type
    where T: ToString,
          U: ToString
{
    let key = key.to_string();
    let attribute = attribute.to_string();

    if let Ok(extracted) = extract_attrs(&mut derive_input.clone()) {
        for (field, attrs) in extracted {
            let col = field.to_string();
            if key == col {
                let types = attrs.types.clone();
                let errors = attrs.errors.clone();

                if attribute.as_str() == "types" {
                    if let Some(types) = types {
                        return types;
                    }
                }

                if attribute.as_str() == "errors" {
                    if let Some(errors) = errors {
                        return errors;
                    }
                }
            }
        }
    }

    type_from_wrapper(type_to_string(&defaults))
}

// Retrieve sanitize attribute pairs
pub fn get_sanitize(derive_input: &DeriveInput) -> Vec<TokenStream> {
    let mut sanitizers = vec![];

    if let Ok(extracted) = extract_attrs(&mut derive_input.clone()) {
        for (field, attrs) in extracted {
            if let Some(attr) = attrs.sanitize {
                if attr.value().as_str() == "trim" {
                    sanitizers.push(quote::quote! {
                        if let nulls::Null::Value(value) = data.#field.clone() {
                            if !value.is_empty() {
                                data.#field = nulls::new(value.to_string().trim().to_string());
                            }
                        }
                    });
                }

                if attr.value().as_str() == "trim_slash" {
                    sanitizers.push(quote::quote! {
                        if let nulls::Null::Value(value) = data.#field.clone() {
                            if !value.is_empty() {
                                data.#field = nulls::new(value
                                    .to_string()
                                    .trim()
                                    .trim_end_matches('/')
                                    .trim()
                                    .to_string());
                            }
                        }
                    });
                }
            }
        }
    }

    sanitizers
}

// Check if type is attributed
pub fn is_attributed_type<T, U>(key: T, attribute: U, derive_input: &DeriveInput) -> bool
    where T: ToString,
          U: ToString
{
    let key = key.to_string();
    let attribute = attribute.to_string();

    if let Ok(extracted) = extract_attrs(&mut derive_input.clone()) {
        for (field, attrs) in extracted {
            let col = field.to_string();
            if key == col {
                let types = attrs.types.clone();
                let errors = attrs.errors.clone();

                if attribute.as_str() == "types" && types.is_some() {
                    return true;
                }

                if attribute.as_str() == "errors" && errors.is_some() {
                    return true;
                }
            }
        }
    }

    false
}
