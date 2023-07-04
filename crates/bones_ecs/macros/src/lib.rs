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

/// Derive macro for the `HasSchema` trait.
#[proc_macro_derive(HasSchema, attributes(schema))]
pub fn derive_has_schema(input: TokenStream) -> TokenStream {
    let input = venial::parse_declaration(input.into()).unwrap();
    let name = input.name().expect("Type must have a name");
    let schema_mod = quote!(::bones_ecs::schema);

    if !input.attributes().iter().any(|attr| {
        &attr.path[0].to_string() == "repr" && &attr.value.get_value_tokens()[0].to_string() == "C"
    }) {
        throw!(
            input.name(),
            "You must have a `#[repr(C)]` annotation on your struct to derive `HasSchema`"
        );
    }

    let schema = match input {
        venial::Declaration::Struct(s) => {
            let fields = match s.fields {
                venial::StructFields::Tuple(tuple) => tuple
                    .fields
                    .iter()
                    .map(|(field, _)| {
                        let ty = &field.ty;
                        quote_spanned! {field.ty.__span() =>
                            #schema_mod::StructField {
                                name: None,
                                schema: <#ty as #schema_mod::HasSchema>::schema(),
                            }
                        }
                    })
                    .collect::<Vec<_>>(),
                venial::StructFields::Named(named) => named
                    .fields
                    .iter()
                    .map(|(field, _)| {
                        let name = &field.name;
                        let ty = &field.ty;
                        let opaque = field.attributes.iter().any(|attr| {
                            &attr.path[0].to_string() == "schema"
                                && &attr.value.get_value_tokens()[0].to_string() == "opaque"
                        });

                        if opaque {
                            quote_spanned! {field.ty.__span() =>
                                #schema_mod::StructField {
                                    name: Some(stringify!(#name).to_owned()),
                                    schema: {
                                        let layout = ::std::alloc::Layout::new::<#ty>();
                                        #schema_mod::Schema::Primitive(#schema_mod::Primitive::Opaque {
                                            size: layout.size(),
                                            align: layout.align(),
                                        })
                                    },
                                }
                            }
                        } else {
                            quote_spanned! {field.ty.__span() =>
                                #schema_mod::StructField {
                                    name: Some(stringify!(#name).to_owned()),
                                    schema: <#ty as #schema_mod::HasSchema>::schema(),
                                }
                            }
                        }
                    })
                    .collect::<Vec<_>>(),
                venial::StructFields::Unit => {
                    throw!(s.name, "Cannot derive HasSchema for unit structs.");
                }
            };

            quote! {
                #schema_mod::Schema::Struct(#schema_mod::StructSchema {
                    fields: vec![
                        #(#fields),*
                    ]
                })
            }
        }
        venial::Declaration::Enum(_) => todo!(),
        _ => {
            throw!(
                input,
                "You may only derive HasSchema for structs and enums."
            );
        }
    };

    quote! {
        impl #schema_mod::HasSchema for #name {
            fn schema() -> #schema_mod::Schema {
                #schema
            }
        }
    }
    .into()
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
