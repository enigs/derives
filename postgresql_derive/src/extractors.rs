use std::collections::HashMap;
use deluxe::extract_attributes;
use syn::{DeriveInput, Data, Ident, Type};

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
