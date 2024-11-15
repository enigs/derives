use std::collections::HashMap;
use deluxe::extract_attributes;
use regex::Regex;
use syn::{DeriveInput, Data, Fields, Ident, Type, parse_str};

// Set Attr struct
#[derive(deluxe::ExtractAttributes)]
#[deluxe(attributes(psql))]
#[allow(dead_code)]
pub struct Attrs {
    pub types: Type,
    pub props: Option<Type>
}

// Extract attributes
pub fn extract_attrs(ast: &mut DeriveInput) -> deluxe::Result<HashMap<Ident, Attrs>> {
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

#[derive(deluxe::ExtractAttributes)]
#[deluxe(attributes(props))]
#[allow(dead_code)]
pub struct PropsAttrs{
    pub types: Type
}

// Extract props
pub fn extract_props(ast: &mut DeriveInput) -> deluxe::Result<HashMap<Ident, PropsAttrs>> {
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

// Create a syn::Type to String conversion
pub fn type_to_string(input: &Type) -> String {
    format!("{}", quote::quote! { #input })
        .replace(" ", "")
}

// Retrieve all available field
pub fn setter_fields(data: &Data) -> Vec<(Ident, Type)> {
    let mut fields = vec![];

    if let Data::Struct(s) = data.clone() {
        if let Fields::Named(f) = s.fields {
            for field in f.named.iter() {
                if let Some(ident) = field.ident.clone() {
                    fields.push((ident, type_from_wrapper(type_to_string(&field.ty))));
                }
            }
        }
    }

    fields
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