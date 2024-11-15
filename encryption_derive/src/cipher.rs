use proc_macro2::TokenStream;
use syn::Ident;

pub fn stream(
    node: &Ident,
    fields: &Vec<Ident>,
    props: &Vec<TokenStream>
) -> TokenStream {
    quote::quote! {
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

            pub fn mutate(&mut self, form: &Self) -> &mut Self {
                #(
                    self.#fields = form.#fields.clone();
                )*

                self
            }

            pub fn encrypt(&self) -> Result<Self, errors::Error> {
                let mut data = self.clone();

                #(
                    if let Some(cipher) = self.#fields.clone().take() {
                        if !cipher.is_empty() {
                            data.#fields = nulls::new(cipher.encrypt()?);
                        }
                    }
                )*

                Ok(data)
            }

            pub fn decrypt(&self) -> Result<Self, errors::Error> {
                let mut data = self.clone();

                #(
                    if let Some(cipher) = self.#fields.clone().take() {
                        if !cipher.is_empty() {
                            data.#fields = nulls::new(cipher.decrypt()?);
                        }
                    }
                )*

                Ok(data)
            }

            #(#props)*
        }
    }
}