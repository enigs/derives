use change_case::snake_case;
use proc_macro::TokenStream;
use quote::format_ident;
use syn::{parse_macro_input, Data, DeriveInput};

#[proc_macro_derive(Enums)]
pub fn derive_enum_iter(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    let ident = &ast.ident;
    let variants = match &ast.data {
        Data::Enum(data) => data.variants.iter()
            .map(|variant| &variant.ident)
            .collect::<Vec<_>>(),
        _ => panic!("This derive is only applicable under enum types.")
    };

    let default_variant = match &ast.data {
        Data::Enum(data) => {
            data.variants
                .iter()
                .find_map(|variant| {
                    variant.attrs.iter().find_map(|attr| {
                        if attr.path().segments.len() == 1 && attr.path().segments[0].ident == "default" {
                            Some(&variant.ident)
                        } else {
                            None
                        }
                    })
                })
        },
        _ => panic!("This derive is only applicable under enum types.")
    };


    let mut token = quote::quote!{};
    let mut default_from_conversion = quote::quote!{};

    let mut checkers = vec![];
    let mut variant_name_snake_lower = vec![];
    let mut variant_name_snake_upper = vec![];

    for variant in variants.clone() {
        let function_name = format_ident!("is_{}", snake_case(&variant.to_string()));
        checkers.push(quote::quote! {
            pub fn #function_name(&self) -> bool {
                *self == Self::#variant
            }
        });

        if let Some(default_variant) = default_variant {
            default_from_conversion.extend(quote::quote! {
                _ => Self::#default_variant,
            });
        };

        variant_name_snake_lower.push(snake_case(&variant.to_string()));
        variant_name_snake_upper.push(snake_case(&variant.to_string()).to_uppercase());
    }

    token.extend(quote::quote! {
        impl #ident {
            #(#checkers)*
        }

        // Conversions
        // ____________________________________________
        impl From<String> for #ident {
            fn from(value: String) -> Self {
                match value.to_lowercase().as_str() {
                    #(#variant_name_snake_lower => Self::#variants,)*
                    #default_from_conversion
                }
            }
        }

        impl From<&String> for #ident {
            fn from(value: &String) -> Self {
                Self::from(value.to_string())
            }
        }

        impl From<&str> for #ident {
            fn from(value: &str) -> Self {
                Self::from(value.to_string())
            }
        }

        impl From<Option<String>> for #ident {
            fn from(value: Option<String>) -> Self {
                match value {
                    Some(value) => Self::from(value),
                    None => Self::None,
                }
            }
        }

        // Display
        // ____________________________________________
        impl std::fmt::Display for #ident {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                match self {
                    #(#ident::#variants => write!(f, #variant_name_snake_upper),)*
                }
            }
        }

        // Deserialize
        // ____________________________________________
        impl<'de> serde::de::Deserialize<'de> for #ident {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where
                    D: serde::Deserializer<'de>,
            {
                let variant = String::deserialize(deserializer)?;

                match variant.to_lowercase().as_str() {
                    #( #variant_name_snake_lower => Ok(Self::#variants), )*
                    _ => Err(serde::de::Error::unknown_variant(
                        &variant,
                        &[  #( #variant_name_snake_upper, )* ],
                    )),
                }
            }
        }

        // Serialize
        // ____________________________________________
        impl serde::Serialize for #ident {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
                let variant_str = match self {
                    #( Self::#variants => #variant_name_snake_upper, )*
                };

                serializer.serialize_str(variant_str)
            }
        }
    });

    token.into()
}