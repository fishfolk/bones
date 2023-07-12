use proc_macro::TokenStream;
use quote::{format_ident, quote, quote_spanned, spanned::Spanned};

/// Helper macro to bail out of the macro with a compile error.
macro_rules! throw {
    ($hasSpan:expr, $err:literal) => {
        let span = $hasSpan.__span();
        return quote_spanned!(span =>
            compile_error!($err);
        ).into();
    };
}

/// Derive macro for the `HasTypeRegistration` trait.
#[proc_macro_derive(HasTypeRegistration)]
pub fn derive_has_type_registration(input: TokenStream) -> TokenStream {
    let input = venial::parse_declaration(input.into()).unwrap();
    let item_ident = &input.name().unwrap();

    let module_ident = format_ident!(
        "{}_derive_has_type_registration",
        item_ident.to_string().to_lowercase()
    );

    // Parse the struct
    let Some(_in_struct) = input.as_struct() else {
        throw!(item_ident.span(), "You may only derive HasTypeRegistration on structs");
    };

    quote! {
        mod #module_ident {
            use super::#item_ident;

            impl ::bones_reflect::registry::HasTypeRegistration for #item_ident {
                fn get_type_registration() -> ::bones_reflect::registry::TypeRegistration {
                    let mut registration = ::bones_reflect::registry::TypeRegistration::of::<Self>();
                    registration
                }
            }
        }
    }
    .into()
}

/// Derive macro for the `HasSchema` trait.
#[proc_macro_derive(HasSchema, attributes(schema))]
pub fn derive_has_schema(input: TokenStream) -> TokenStream {
    let input = venial::parse_declaration(input.into()).unwrap();
    let name = input.name().expect("Type must have a name");
    let schema_mod = quote!(::bones_reflect::schema);

    if !input.attributes().iter().any(|attr| {
        &attr.path[0].to_string() == "repr" && &attr.value.get_value_tokens()[0].to_string() == "C"
    }) {
        throw!(
            input.name(),
            "You must have a `#[repr(C)]` annotation on your struct to derive `HasSchema`"
        );
    }

    let schema_kind = match input {
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
                                schema: <#ty as #schema_mod::HasSchema>::schema().clone(),
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
                                        #schema_mod::Schema {
                                            kind: #schema_mod::SchemaKind::Primitive(#schema_mod::Primitive::Opaque {
                                                size: layout.size(),
                                                align: layout.align(),
                                            }),
                                            type_id: Some(std::any::TypeId::of::<#ty>()),
                                            type_data: Default::default(),
                                        }
                                    },
                                }
                            }
                        } else {
                            quote_spanned! {field.ty.__span() =>
                                #schema_mod::StructField {
                                    name: Some(stringify!(#name).to_owned()),
                                    schema: <#ty as #schema_mod::HasSchema>::schema().clone(),
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
                #schema_mod::SchemaKind::Struct(#schema_mod::StructSchema {
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
        unsafe impl #schema_mod::HasSchema for #name {
            fn schema() -> &'static #schema_mod::Schema {
                static S: ::std::sync::OnceLock<#schema_mod::Schema> = ::std::sync::OnceLock::new();
                S.get_or_init(|| {
                    #schema_mod::Schema {
                        type_id: Some(std::any::TypeId::of::<Self>()),
                        kind: #schema_kind,
                        type_data: Default::default(),
                    }
                })
            }
        }
    }
    .into()
}
