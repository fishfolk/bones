use proc_macro::TokenStream;
use quote::{quote, quote_spanned, spanned::Spanned};

/// Helper macro to bail out of the macro with a compile error.
macro_rules! throw {
    ($hasSpan:expr, $err:literal) => {
        let span = $hasSpan.__span();
        return quote_spanned!(span =>
            compile_error!($err);
        ).into();
    };
}

/// Derive macro for deriving [`Deref`] on structs with one field.
#[proc_macro_derive(Deref, attributes(asset_id, asset))]
pub fn derive_deref(input: TokenStream) -> TokenStream {
    let input = venial::parse_declaration(input.into()).unwrap();

    if let Some(s) = input.as_struct() {
        let name = &s.name;
        let params = &s.generic_params;

        match &s.fields {
            venial::StructFields::Tuple(tuple) => {
                if tuple.fields.len() != 1 {
                    throw!(tuple, "May only derive Deref for structs with one field.");
                }

                let deref_type = &tuple.fields[0].0.ty;

                quote! {
                    impl #params ::std::ops::Deref for #name #params {
                        type Target = #deref_type;

                        fn deref(&self) -> &Self::Target {
                            &self.0
                        }
                    }
                }
                .into()
            }
            venial::StructFields::Named(named) => {
                if named.fields.len() != 1 {
                    throw!(named, "May only derive Deref for structs with one field.");
                }

                let deref_type = &named.fields[0].0.ty;
                let field_name = &named.fields[0].0.name;

                quote! {
                    impl #params ::std::ops::Deref for #name #params {
                        type Target = #deref_type;

                        fn deref(&self) -> &Self::Target {
                            &self.#field_name
                        }
                    }
                }
                .into()
            }
            venial::StructFields::Unit => {
                throw!(s, "Cannot derive Deref on anything but structs.");
            }
        }
    } else {
        throw!(input, "Cannot derive Deref on anything but structs.");
    }
}

/// Derive macro for deriving [`DerefMut`] on structs with one field.
#[proc_macro_derive(DerefMut, attributes(asset_id, asset))]
pub fn derive_deref_mut(input: TokenStream) -> TokenStream {
    let input = venial::parse_declaration(input.into()).unwrap();

    if let Some(s) = input.as_struct() {
        let name = &s.name;
        let params = &s.generic_params;

        match &s.fields {
            venial::StructFields::Tuple(tuple) => {
                if tuple.fields.len() != 1 {
                    throw!(
                        tuple,
                        "May only derive DerefMut for structs with one field."
                    );
                }

                quote! {
                    impl #params std::ops::DerefMut for #name #params {
                        fn deref_mut(&mut self) -> &mut Self::Target {
                            &mut self.0
                        }
                    }
                }
                .into()
            }
            venial::StructFields::Named(named) => {
                if named.fields.len() != 1 {
                    throw!(
                        named,
                        "May only derive DerefMut for structs with one field."
                    );
                }

                let field_name = &named.fields[0].0.name;

                quote! {
                    impl #params std::ops::DerefMut for #name #params {
                        fn deref_mut(&mut self) -> &mut Self::Target {
                            &mut self.#field_name
                        }
                    }
                }
                .into()
            }
            venial::StructFields::Unit => {
                throw!(s, "Cannot derive DerefMut on anything but structs.");
            }
        }
    } else {
        throw!(input, "Cannot derive DerefMut on anything but structs.");
    }
}
