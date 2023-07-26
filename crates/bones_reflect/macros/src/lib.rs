use proc_macro::TokenStream;
use proc_macro2::TokenTree;
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

/// Derive macro for the `HasSchema` trait.
#[proc_macro_derive(HasSchema, attributes(schema, type_datas))]
pub fn derive_has_schema(input: TokenStream) -> TokenStream {
    let input = venial::parse_declaration(input.into()).unwrap();
    let name = input.name().expect("Type must have a name");
    let schema_mod = quote!(::bones_reflect);

    let get_flags_for_attr = |attr_name: &str| {
        let attr = input
            .attributes()
            .iter()
            .find(|attr| attr.path.len() == 1 && attr.path[0].to_string() == attr_name);
        attr.map(|attr| match &attr.value {
            venial::AttributeValue::Group(_, value) => {
                let mut flags = Vec::new();

                let mut current_flag = proc_macro2::TokenStream::new();
                for token in value {
                    match token {
                        TokenTree::Punct(x) if x.as_char() == ',' => {
                            flags.push(current_flag.to_string());
                            current_flag = Default::default();
                        }
                        x => current_flag.extend(std::iter::once(x.clone())),
                    }
                }
                flags.push(current_flag.to_string());

                flags
            }
            venial::AttributeValue::Equals(_, _) => {
                // TODO: Better error message span.
                panic!("Unsupported attribute format");
            }
            venial::AttributeValue::Empty => Vec::new(),
        })
        .unwrap_or_default()
    };

    let type_datas_flags = get_flags_for_attr("type_datas");
    let type_datas = {
        let add_type_datas = type_datas_flags.into_iter().map(|ty| {
            let ty = format_ident!("{ty}");
            quote! {
                let id = <#ty as #schema_mod::TypeData>::TYPE_DATA_ID;
                tds.0.insert(id, #schema_mod::ptr::SchemaBox::new(<#ty as #schema_mod::FromType<#name>>::from_type()));
            }
        });

        quote! {
            {
                let mut tds = #schema_mod::TypeDatas::default();
                #(#add_type_datas),*
                tds
            }
        }
    };

    let schema_flags = get_flags_for_attr("schema");

    let is_opaque = schema_flags.iter().any(|x| x.as_str() == "opaque");
    let no_clone = schema_flags.iter().any(|x| x.as_str() == "no_clone");
    let no_default = schema_flags.iter().any(|x| x.as_str() == "no_default");

    let clone_fn = if no_clone {
        quote!(None)
    } else {
        quote!(Some(<Self as #schema_mod::RawClone>::raw_clone))
    };
    let default_fn = if no_default {
        quote!(None)
    } else {
        quote!(Some(<Self as #schema_mod::RawDefault>::raw_default))
    };

    if is_opaque {
        return quote! {
            unsafe impl #schema_mod::HasSchema for #name {
                fn schema() -> &'static #schema_mod::Schema {
                    static S: ::std::sync::OnceLock<&'static #schema_mod::Schema> = ::std::sync::OnceLock::new();
                    S.get_or_init(|| {
                        let layout = std::alloc::Layout::new::<Self>();
                        #schema_mod::registry::SCHEMA_REGISTRY.register(#schema_mod::Schema {
                            id: None,
                            type_id: Some(std::any::TypeId::of::<Self>()),
                            clone_fn: #clone_fn,
                            default_fn: #default_fn,
                            drop_fn: Some(<Self as #schema_mod::RawDrop>::raw_drop),
                            kind: #schema_mod::SchemaKind::Primitive(#schema_mod::Primitive::Opaque {
                                size: layout.size(),
                                align: layout.align(),
                            }),
                            type_data: #type_datas,
                        }).1
                    })
                }
            }
        }
        .into();
    }

    if !input
        .attributes()
        .iter()
        .any(|attr| quote!(#attr).to_string() == "#[repr(C)]")
    {
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
                                    name: Some(stringify!(#name).into()),
                                    schema: {
                                        let layout = ::std::alloc::Layout::new::<#ty>();
                                        #schema_mod::Schema {
                                            id: None,
                                            kind: #schema_mod::SchemaKind::Primitive(#schema_mod::Primitive::Opaque {
                                                size: layout.size(),
                                                align: layout.align(),
                                            }),
                                            type_id: Some(std::any::TypeId::of::<#ty>()),
                                            type_data: #type_datas,
                                            clone_fn: #clone_fn,
                                            default_fn: #default_fn,
                                            drop_fn: Some(<Self as #schema_mod::RawDrop>::raw_drop),
                                        }
                                    },
                                }
                            }
                        } else {
                            quote_spanned! {field.ty.__span() =>
                                #schema_mod::StructField {
                                    name: Some(stringify!(#name).into()),
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
                static S: ::std::sync::OnceLock<&'static #schema_mod::Schema> = ::std::sync::OnceLock::new();
                S.get_or_init(|| {
                    #schema_mod::registry::SCHEMA_REGISTRY.register(#schema_mod::Schema {
                        id: None,
                        type_id: Some(std::any::TypeId::of::<Self>()),
                        kind: #schema_kind,
                        type_data: #type_datas,
                        default_fn: #default_fn,
                        clone_fn: #clone_fn,
                        drop_fn: Some(<Self as #schema_mod::RawDrop>::raw_drop),
                    }).1
                })
            }
        }
    }
    .into()
}
