use syn::{Data, Fields, Ident, Type};

// Retrieve all available field
pub fn all_fields(data: &Data) -> Vec<(Ident, Type)> {
    let mut fields = vec![];

    if let Data::Struct(s) = data.clone() {
        if let Fields::Named(f) = s.fields {
            for field in f.named.iter() {
                if let Some(ident) = field.ident.clone() {
                    fields.push((ident, field.ty.clone()));
                }
            }
        }
    }

    fields
}