mod extractors;

use deluxe::Result;
use proc_macro2::TokenStream;
use quote::format_ident;
use syn::{Data, DeriveInput};

// Start of derive and field attribute derives
#[proc_macro_derive(PostgreSQL, attributes(psql, props))]
pub fn main(stream: proc_macro::TokenStream) -> proc_macro::TokenStream {
    derive(stream.into()).unwrap().into()
}

// Start of derive and token processing
fn derive(stream: TokenStream) -> Result<TokenStream> {
    // Parse token stream
    let mut ast: DeriveInput = syn::parse2(stream)?;
    let node = &ast.ident.clone();
    let prefix = stringcase::snake_case(&node.clone().to_string()).to_lowercase();

    // Create main token stream
    let mut token = quote::quote!{};

    // Set variables
    let mut fields = vec![];
    let mut types = vec![];
    let mut names = vec![];
    let mut props = vec![];

    let mut all_fields = vec![];
    let mut aliased_values = vec![];
    let mut plain_values = vec![];
    let mut renamed_values = vec![];
    let mut tabled_values = vec![];

    let mut jsons = vec![];

    for (field, attr) in extractors::extract_attrs(&mut ast)? {
        let plain_field = format_ident!("{}", field.to_string().to_uppercase());
        let aliased_value = format!("{}.{} AS {}_{}", prefix.clone(), field, prefix.clone(), field);
        let plain_value = field.to_string().to_lowercase();
        let renamed_value = format!("{}_{}", prefix.clone(), field);
        let tabled_value = format!("{}.{}", prefix.clone(), field);

        all_fields.push(plain_field);
        aliased_values.push(aliased_value);
        plain_values.push(plain_value);
        renamed_values.push(renamed_value.clone());
        tabled_values.push(tabled_value);

        fields.push(field.clone());
        types.push(attr.types.clone());
        names.push(renamed_value);

        let t = match attr.props.clone() {
            Some(t) => t,
            None => attr.types.clone(),
        };

        let is_json = extractors::type_to_string(&attr.types).contains("Json");
        if is_json {
            let fname = format_ident!("{}_json", field.to_string().to_lowercase());
            jsons.push(quote::quote! {
                pub fn #fname(&self) -> Option<Json<#t>> {
                    if let Some(data) = self.#field.clone().take() {
                        if !data.is_empty() {
                            return Some(Json::from(data));
                        }
                    }

                    None
                }
            });
        }

        props.push(quote::quote! {
            pub fn #field(&self) -> Option<#t> {
                self.#field.clone().take()
            }
        });
    }

    for (field, attr) in extractors::extract_props(&mut ast)? {
        let ty = attr.types;

        props.push(quote::quote! {
            pub fn #field(&self) -> Option<#ty> {
                self.#field.clone().take()
            }
        });
    }

    let mut null_to_undefined = vec![];
    let mut field_to_undefined = vec![];
    if let Data::Struct(s) = &mut ast.data {
        for f in s.fields.iter_mut() {
            if let Some(field_name) = f.ident.clone() {
                let field_type = extractors::type_to_string(&f.ty.clone());
                if field_type.contains("Null") {
                    null_to_undefined.push(field_name.clone());
                    field_to_undefined.push(format_ident!("{}_to_undefined", field_name));
                }
            }
        }
    }

    // Combine fields as String value
    let all_aliased_values = aliased_values.join(",");
    let all_plain_values = plain_values.join(",");
    let all_renamed_values = renamed_values.join(",");
    let all_tabled_values = tabled_values.join(",");

    // Extend alternative functionalities for base struct
    token.extend(quote::quote!{
        impl #node {
            pub fn is_empty(&self) -> bool {
                *self == Self::default()
            }

            pub fn to<T: From<Self>>(&self) -> T {
                T::from(self.clone())
            }

            pub fn to_json(&self) -> sqlx::types::Json<Self> {
                sqlx::types::Json::from(self.clone())
            }

            pub fn parse(row: &sqlx::postgres::PgRow) -> Self {
                use sqlx::Row;

                let mut data = Self::default();

                #(
                    data.#fields = Null::from(row.try_get::<#types, &str>(#names));
                )*

                data
            }

            pub fn as_response(&self) -> actix_web::Result<actix_web::HttpResponse> {
                let response = responses::as_response(self);

                Ok(actix_web::HttpResponse::Ok().json(response))
            }

            pub fn nulls_to_undefined(&self) -> Self {
                let mut data = self.clone();

                #(
                    if let Null::Null = data.#null_to_undefined {
                        data.#null_to_undefined = Null::Undefined;
                    }
                )*

                data
            }

            #(
                pub fn #field_to_undefined(&self) -> Self {
                    let mut data = self.clone();

                    data.#null_to_undefined = Null::Undefined;

                    data
                }
            )*

            #(#props)*

            #(#jsons)*
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
    });

    // Set parsers
    token.extend(quote::quote! {
        pub mod parsers {
            use nulls::Null;

            use crate::#node;

            pub fn parse(row: &sqlx::postgres::PgRow) -> #node {
                #node::parse(row)
            }

            pub fn result(row: sqlx::Result<sqlx::postgres::PgRow>) -> actix_web::Result<#node> {
                let result = row.map_err(errors::query)?;
                let row = parse(&result);

                match !row.is_empty() {
                    true => Ok(row),
                    false => Err(errors::Errors::to("table row not found"))
                }
            }

            pub fn relational(row: &sqlx::postgres::PgRow) -> Null<#node> {
                let row = parse(row);

                match row.is_empty() {
                    true => Null::Undefined,
                    false => Null::Value(row)
                }
            }
        }
    });

    // Extend token alias
    token.extend(quote::quote!{
        pub mod alias {
            pub const ALL: &'static str = #all_aliased_values;

            #(
                pub const #all_fields: &'static str = #aliased_values;
            )*
        }
    });

    // Extend token plain
    token.extend(quote::quote!{
        pub mod plain {
            pub const ALL: &'static str = #all_plain_values;

            #(
                pub const #all_fields: &'static str = #plain_values;
            )*
        }
    });

    // Extend token renamed
    token.extend(quote::quote!{
        pub mod renamed {
            pub const ALL: &'static str = #all_renamed_values;

            #(
                pub const #all_fields: &'static str = #renamed_values;
            )*
        }
    });

    // Extend token table
    token.extend(quote::quote!{
        pub mod tables {
            pub const ALL: &'static str = #all_tabled_values;

            #(
                pub const #all_fields: &'static str = #tabled_values;
            )*
        }
    });


    // Return the new token
    Ok(token)
}