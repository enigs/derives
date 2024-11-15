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

    let paginated = format_ident!("{}Page", node);

    // Create main token stream
    let mut token = quote::quote!{};

    // Set variables
    let mut fields = vec![];
    let mut types = vec![];
    let mut names = vec![];
    let mut props = vec![];
    let mut setters = vec![];

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
                pub fn #fname(&self) -> Option<sqlx::types::Json<#t>> {
                    if let Some(data) = self.#field.clone().take() {
                        if !data.is_empty() {
                            return Some(sqlx::types::Json::from(data));
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

    for (field, attr) in extractors::setter_fields(&ast.data) {
        let field_name = format_ident!("set_{}", field);

        setters.push(quote::quote!{
            pub fn #field_name(&self, value: #attr) -> Self {
                let mut data = self.clone();

                data.#field = nulls::new(value.clone());

                data
            }
        });

        if field.to_string().as_str() == "id" {
            let field_name = format_ident!("set_insert_{}", field);

            setters.push(quote::quote!{
                pub fn #field_name<T: ToString>(&self, value: T) -> Self {
                    let mut data = self.clone();
                    let id = data.id().unwrap_or_default();

                    if id.is_empty() {
                        data.#field = nulls::new(value.to_string());
                    }

                    data
                }
            });
        }
    }

    let mut null_to_undefined = vec![];
    let mut field_to_undefined = vec![];
    let mut cloned_fields = vec![];
    if let Data::Struct(s) = &mut ast.data {
        for f in s.fields.iter_mut() {
            let field_type = f.ty.clone();

            if let Some(field_name) = f.ident.clone() {
                let cloned_field = format_ident!("clone_{}", field_name);
                cloned_fields.push(quote::quote!{
                    pub fn #cloned_field(&self, value: &#field_type) -> Self {
                        let mut data = self.clone();

                        data.#field_name = value.clone();

                        data
                    }
                });

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

    // Create list
    let node_list = format_ident!("{}List", node.clone());

    // Extend alternative functionalities for base struct
    token.extend(quote::quote!{

        #[derive(Default, Debug, Clone, PartialEq)]
        #[derive(Deserialize, Serialize)]
        #[serde(rename_all = "camelCase")]
        pub struct #node_list(Vec<#node>);

        impl #node_list {
            pub fn is_empty(&self) -> bool {
                *self == Self::default()
            }

            pub fn push(&mut self, value: #node) {
                self.0.push(value);
            }

            pub fn parse(rows: sqlx::Result<Vec<sqlx::postgres::PgRow>>) -> Self {
                let mut list = Vec::new();

                if let Ok(rows) = rows {
                    for row in rows {
                        list.push(parsers::parse(&row));
                    }
                }

                #node_list(list)
            }
        }

        impl actix_web::Responder for #node_list {
            type Body = actix_web::body::BoxBody;

            fn respond_to(self, _req: &actix_web::HttpRequest) -> actix_web::HttpResponse {
                actix_web::HttpResponse::Ok().json(serde_json::json!({
                    "code": 200,
                    "data": self
                }))
            }
        }

        #[derive(Default, Debug, Clone, PartialEq)]
        #[derive(Deserialize, Serialize)]
        #[serde(rename_all = "camelCase")]
        pub struct #paginated {
            #[serde(skip_serializing_if = "Option::is_none")]
            pub page: Option<i64>,
            #[serde(skip_serializing_if = "Option::is_none")]
            pub per_page: Option<i64>,
            #[serde(skip_serializing_if = "Option::is_none")]
            pub filtered_count: Option<i64>,
            #[serde(skip_serializing_if = "Option::is_none")]
            pub total_count: Option<i64>,
            #[serde(skip_serializing_if = "Option::is_none")]
            pub search: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            pub filters: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            pub orders: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            pub records: Option<#node_list>,
        }

        impl actix_web::Responder for #paginated {
            type Body = actix_web::body::BoxBody;

            fn respond_to(self, _req: &actix_web::HttpRequest) -> actix_web::HttpResponse {
                actix_web::HttpResponse::Ok().json(serde_json::json!({
                    "code": 200,
                    "data": self
                }))
            }
        }

        impl #paginated {
            pub fn request(&self) -> Self {
                let mut data = self.clone();
                data.filtered_count = None;
                data.total_count = None;

                if data.page.clone().unwrap_or(0) < 1 {
                    data.page = Some(1);
                }

                if data.per_page.clone().unwrap_or(0) < 1 {
                    data.per_page = Some(10);
                }

                data
            }

            pub fn search(&self) -> String {
                self.search.clone().unwrap_or_default()
            }

            pub fn filters(&self) -> (Vec<String>, Vec<serde_json::Value>, usize) {
                let data = self.clone();
                let mut filter = vec![];

                if let Some(f) = data.filters.clone() {
                    if let Ok(f) = serde_json::from_str::<Vec<crate::Filter>>(&f) {
                        filter = f;
                    }
                }

                let mut conds = vec![];
                let mut vals =  vec![];
                let mut idx = 0;

                for (index, value) in filter.clone().into_iter().enumerate() {
                    idx = index + 1;
                    let mut col = value.cols.clone().unwrap_or_default();
                    let op = value.ops.clone().unwrap_or_default();
                    let val = value.vals.clone().unwrap_or(serde_json::Value::Null);

                    if let Ok(re) = regex::Regex::new(r"[^a-zA-Z0-9._]") {
                        col = re.replace_all(&col, "").to_string();
                    }

                    match op {
                        crate::FilterOps::Gt => {
                            conds.push(format!("{} > ${}", col, idx));
                            vals.push(value.vals.unwrap_or(serde_json::Value::Null));
                        },
                        crate::FilterOps::Lt => {
                            conds.push(format!("{} < ${}", col, idx));
                            vals.push(value.vals.unwrap_or(serde_json::Value::Null));
                        },
                        crate::FilterOps::Like => {
                            let v = match val.clone() {
                                serde_json::Value::Null => "%NULL%".to_string(),
                                serde_json::Value::Bool(d) => format!("%{}%", d),
                                serde_json::Value::Number(d) => format!("%{}%", d),
                                serde_json::Value::String(d) => format!("%{}%", d),
                                serde_json::Value::Array(d) => format!("%{}%", serde_json::json!(d).to_string()),
                                serde_json::Value::Object(d) => format!("%{}%", serde_json::json!(d).to_string()),
                            };

                            conds.push(format!("{} LIKE ${}", col, idx));
                            vals.push(serde_json::Value::String(v));
                        },
                        crate::FilterOps::LikeLeft => {
                            let v = match val.clone() {
                                serde_json::Value::Null => "%NULL".to_string(),
                                serde_json::Value::Bool(d) => format!("%{}", d),
                                serde_json::Value::Number(d) => format!("%{}", d),
                                serde_json::Value::String(d) => format!("%{}", d),
                                serde_json::Value::Array(d) => format!("%{}", serde_json::json!(d).to_string()),
                                serde_json::Value::Object(d) => format!("%{}", serde_json::json!(d).to_string()),
                            };

                            conds.push(format!("{} LIKE ${}", col, idx));
                            vals.push(serde_json::Value::String(v));
                        },
                        crate::FilterOps::LikeRight => {
                            let v = match val.clone() {
                                serde_json::Value::Null => "NULL%".to_string(),
                                serde_json::Value::Bool(d) => format!("{}%", d),
                                serde_json::Value::Number(d) => format!("{}%", d),
                                serde_json::Value::String(d) => format!("{}%", d),
                                serde_json::Value::Array(d) => format!("{}%", serde_json::json!(d).to_string()),
                                serde_json::Value::Object(d) => format!("{}%", serde_json::json!(d).to_string()),
                            };

                            conds.push(format!("{} LIKE ${}", col, idx));
                            vals.push(serde_json::Value::String(v));
                        },
                        crate::FilterOps::Eq => {
                            conds.push(format!("{} = ${}", col, idx));
                            vals.push(value.vals.unwrap_or(serde_json::Value::Null));
                        },
                        _ => {}
                    };

                }

                (conds, vals, idx)
            }

            pub fn limit(&self) -> (i64, i64, i64) {
                let mut page = self.page.unwrap_or(1);
                if page < 1 {
                    page = 1;
                }

                let mut per_page = self.per_page.unwrap_or(5);
                if per_page < 1 {
                    per_page = 5;
                }

                let max_page = (self.filtered_count.unwrap_or(0) + per_page - 1) / per_page;
                if page > max_page {
                    page = max_page;
                }

                let offset = (page - 1) * per_page;

                (page as i64, per_page as i64, offset as i64)
            }

            pub fn orders<T: ToString>(&self, default_order:T) -> String {
                let data = self.clone();
                let order = default_order.to_string();
                let mut ovec = vec![];

                if let Some(o) = data.orders {
                    if let Ok(o) = serde_json::from_str::<Vec<crate::Order>>(&o) {
                        for item in o {
                            let mut col = item.cols.clone().unwrap_or_default();
                            if let Ok(re) = regex::Regex::new(r"[^a-zA-Z0-9._]") {
                                col = re.replace_all(&col, "").to_string();
                            }

                            match item.ops.clone().unwrap_or_default() {
                                crate::OrderOps::Asc => ovec.push(format!("{} ASC", col)),
                                crate::OrderOps::Desc => ovec.push(format!("{} DESC", col)),
                            }
                        }
                    }
                }

                if ovec.is_empty() {
                    format!("ORDER BY {}", order)
                } else {
                    format!("ORDER BY {}", ovec.join(", "))
                }
            }

            pub fn response(&self) -> Self {
                let mut data = self.clone();

                data.search = None;
                data.filters = None;
                data.orders = None;

                data
            }
        }

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
                Ok(actix_web::HttpResponse::Ok().json(serde_json::json!({
                    "code": 200,
                    "data": self
                })))
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

            #(#setters)*

             #(#cloned_fields)*
        }

        impl actix_web::Responder for #node {
            type Body = actix_web::body::BoxBody;

            fn respond_to(self, _req: &actix_web::HttpRequest) -> actix_web::HttpResponse {
                actix_web::HttpResponse::Ok().json(serde_json::json!({
                    "code": 200,
                    "data": self
                }))
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
    });

    // Set parsers
    token.extend(quote::quote! {
        pub mod parsers {
            use nulls::Null;

            use crate::#node;

            pub fn parse(row: &sqlx::postgres::PgRow) -> #node {
                #node::parse(row)
            }

            pub fn result(row: sqlx::Result<sqlx::postgres::PgRow>) -> errors::Result<#node> {
                let result = row.map_err(errors::query)?;
                let row = parse(&result);

                match !row.is_empty() {
                    true => Ok(row),
                    false => Err(errors::str_to("table row not found"))
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