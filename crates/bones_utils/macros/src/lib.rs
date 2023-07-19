use proc_macro::TokenStream;
use quote::{quote, quote_spanned, spanned::Spanned, ToTokens};

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
#[proc_macro_derive(Deref, attributes(deref))]
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
                let (deref_type, field_name) = if named.fields.is_empty() {
                    throw!(named, "May not derive Deref for struct without fields");
                } else if named.fields.len() > 1 {
                    let mut info = None;
                    for (field, _) in named.fields.iter() {
                        for attr in &field.attributes {
                            if attr.to_token_stream().to_string() == "#[deref]" {
                                if info.is_some() {
                                    throw!(attr, "Only one field may have the #[deref] attribute");
                                } else {
                                    info = Some((&field.ty, &field.name));
                                }
                            }
                        }
                    }

                    if let Some(info) = info {
                        info
                    } else {
                        throw!(
                            named,
                            "One field must be annotated with a #[deref] attribute"
                        );
                    }
                } else {
                    (&named.fields[0].0.ty, &named.fields[0].0.name)
                };

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
#[proc_macro_derive(DerefMut, attributes(deref))]
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
                let field_name = if named.fields.is_empty() {
                    throw!(named, "May not derive Deref for struct without fields");
                } else if named.fields.len() > 1 {
                    let mut info = None;
                    for (field, _) in named.fields.iter() {
                        for attr in &field.attributes {
                            if attr.to_token_stream().to_string() == "#[deref]" {
                                if info.is_some() {
                                    throw!(attr, "Only one field may have the #[deref] attribute");
                                } else {
                                    info = Some(&field.name);
                                }
                            }
                        }
                    }

                    if let Some(name) = info {
                        name
                    } else {
                        throw!(
                            named,
                            "One field must be annotated with a #[deref] attribute"
                        );
                    }
                } else {
                    &named.fields[0].0.name
                };

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
