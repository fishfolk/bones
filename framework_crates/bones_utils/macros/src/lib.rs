use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, quote_spanned, spanned::Spanned};
use venial::StructFields;

/// Helper macro to bail out of the macro with a compile error.
macro_rules! throw {
    ($hasSpan:expr, $err:literal) => {
        let span = $hasSpan.__span();
        return quote_spanned!(span =>
            compile_error!($err);
        ).into();
    };
}

/// Returns whether or not the passed-in attribute is a simple attribute with no arguments with a
/// name that matches `name`.
fn is_simple_named_attr(attr: &venial::Attribute, name: &str) -> bool {
    attr.get_single_path_segment() == Some(&format_ident!("{name}"))
        && attr.get_value_tokens().is_empty()
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
                            if is_simple_named_attr(attr, "deref") {
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
                            if is_simple_named_attr(attr, "deref") {
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

#[proc_macro_derive(DesyncHash, attributes(desync_hash_module))]
pub fn derive_desync_hash(input: TokenStream) -> TokenStream {
    let input = venial::parse_declaration(input.into()).unwrap();
    let name = input.name().expect("Type must have a name");

    // Get the schema module, reading optionally from the `schema_module` attribute, so that we can
    // set the module to `crate` when we want to use it within the `bones_schema` crate itself.
    let desync_hash_module = input
        .attributes()
        .iter()
        .find_map(|attr| {
            (attr.path.len() == 1 && attr.path[0].to_string() == "desync_hash_module").then(|| {
                attr.value
                    .get_value_tokens()
                    .iter()
                    .cloned()
                    .collect::<TokenStream2>()
            })
        })
        .unwrap_or_else(|| quote!(bones_utils));

    // Helper to get hash invocations of struct fields
    let hash_struct_fields = |fields: &StructFields| match fields {
        venial::StructFields::Tuple(tuple) => tuple
            .fields
            .iter()
            .enumerate()
            .map(|(idx, (field, _))| {
                let ty = &field.ty;
                quote! {<#ty as #desync_hash_module::DesyncHash>::hash(&self.#idx, hasher);}
            })
            .collect::<Vec<_>>(),
        venial::StructFields::Named(named) => named
            .fields
            .iter()
            .map(|(field, _)| {
                let name = &field.name;
                let ty = &field.ty;

                quote! {<#ty as #desync_hash_module::DesyncHash>::hash(&self.#name, hasher);}
            })
            .collect::<Vec<_>>(),
        venial::StructFields::Unit => vec![],
    };

    // Get fields of enum variant
    let enum_variant_fields = |fields: &StructFields| match fields {
        venial::StructFields::Tuple(tuple) => {
            // Build identifiers for tuple as  as a,b,c...

            if tuple.fields.len() > 26 {
                panic!("DesyncHash derive macro does not support variants of tuples with more than 26 fields.");
            }
            let identifiers: Vec<_> = (0..tuple.fields.len())
                .map(|i| format_ident!("{}", (b'a' + i as u8) as char))
                .collect();

            // format as (a, b, c, ...)
            let tuple_fields = quote! {
                (#(#identifiers),*)
            };

            // generate invocations for each field in tuple using generated identifier
            let invocations = identifiers
                .iter()
                .map(|ident| {
                    quote! {#desync_hash_module::DesyncHash::hash(#ident, hasher);}
                })
                .collect::<Vec<_>>();
            (tuple_fields, invocations)
        }
        venial::StructFields::Named(named) => {
            let field_idents: Vec<_> = named.fields.iter().map(|f| &f.0.name).collect();

            // format list of fields as '{ fieldA, fieldB, ... }'
            let named_fields = quote! {
                {#(#field_idents),*}
            };

            let invocations = field_idents
                .iter()
                .map(|ident| {
                    quote! {#desync_hash_module::DesyncHash::hash(#ident, hasher);}
                })
                .collect::<Vec<_>>();
            (named_fields, invocations)
        }
        venial::StructFields::Unit => (quote! {}, vec![]),
    };

    let field_hash_invocations: Vec<_> = match &input {
        venial::Declaration::Struct(s) => hash_struct_fields(&s.fields),
        venial::Declaration::Enum(e) => {
            let mut variants = Vec::new();

            let enum_name = &e.name;
            for (idx, v) in e.variants.items().enumerate() {
                let variant_name = &v.name;
                let (variant_fields_string, invocations) = enum_variant_fields(&v.contents);
                variants.push(quote! {
                    #enum_name::#variant_name #variant_fields_string => {
                        // Hash index of variant to ensure that two variants are unique
                        hasher.write_usize(#idx);
                        #(
                            #invocations
                        )*
                    },
                });
            }

            vec![quote! {
                match self {
                    #(#variants)*
                }
            }]
        }
        venial::Declaration::Union(_) => {
            panic!("DesyncHash derive macro impl does not support Unions");
        }
        _ => vec![],
    };

    let combined_hash_invocations = field_hash_invocations.iter().fold(quote! {}, |acc, q| {
        quote! {
            #acc
            #q
        }
    });

    quote! {
        impl #desync_hash_module::DesyncHash for #name {
            fn hash(&self, hasher: &mut dyn std::hash::Hasher) {
                #combined_hash_invocations
            }
        }
    }
    .into()
}
