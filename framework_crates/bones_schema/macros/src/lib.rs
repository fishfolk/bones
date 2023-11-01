use proc_macro::TokenStream;
use proc_macro2::{Punct, Spacing, TokenStream as TokenStream2, TokenTree as TokenTree2};
use quote::{format_ident, quote, quote_spanned, spanned::Spanned};
use venial::{GenericBound, StructFields};

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
///
/// ## Example
///
/// ```ignore
/// /// This is a custom type data.
/// ///
/// /// While it must implement [`HasSchema`] it is fine to just make it opaque.
/// ///
/// /// In this case we want to store the name of the type in our custom type data.
/// #[derive(HasSchema, Clone, Default)]
/// /// struct TypeName(String);
///
/// /// In order to make [`TypeName`] derivable, we must implement [`FromType`] for it.
/// impl<T> FromType<T> for TypeName {
///     fn from_type() -> Self {
///         Self(std::any::type_name::<T>().to_string())
///     }
/// }
///
/// /// Finally we can derive our type data on other types that implement [`HasSchema`] by using the
/// /// `#[derive_type_data()]` attribute with one or more type datas to derive.
/// #[derive(HasSchema, Debug, Default, Clone)]
/// #[derive_type_data(TypeName)]
/// #[repr(C)]
/// struct MyStruct {
///     x: f32,
///     y: f32,
/// }
///
/// /// It is also possible to add type data that may or may not implement [`FromType`] by passing in an
/// /// expression for the type data into a `type_data` attribute.
/// #[derive(HasSchema, Clone, Default, Debug)]
/// #[type_data(TypeName("CustomName".into()))]
/// #[repr(C)]
/// struct MyOtherStruct;
/// ```
///
/// ## Known Limitations
///
/// Currently it isn't possible to construct a struct that contains itself. For example, this will
/// not work:
///
/// ```ignore
/// #[derive(HasSchema)]
/// struct Data {
///     others: Vec<Data>,
/// }
/// ```
///
/// If this is a problem for your use-case, please open an issue.
#[proc_macro_derive(
    HasSchema,
    attributes(schema, derive_type_data, type_data, schema_module)
)]
pub fn derive_has_schema(input: TokenStream) -> TokenStream {
    let input = venial::parse_declaration(input.into()).unwrap();
    let name = input.name().expect("Type must have a name");

    // Get the schema module, reading optionally from the `schema_module` attribute, so that we can
    // set the module to `crate` when we want to use it within the `bones_schema` crate itself.
    let schema_mod = input
        .attributes()
        .iter()
        .find_map(|attr| {
            (attr.path.len() == 1 && attr.path[0].to_string() == "schema_module").then(|| {
                attr.value
                    .get_value_tokens()
                    .iter()
                    .cloned()
                    .collect::<TokenStream2>()
            })
        })
        .unwrap_or_else(|| quote!(bones_schema));

    // Get the type datas that have been added and derived
    let derive_type_data_flags = get_flags_for_attr(&input, "derive_type_data");
    let type_datas = {
        let add_derive_type_datas = derive_type_data_flags.into_iter().map(|ty| {
            let ty = format_ident!("{ty}");
            quote! {
                tds.insert(<#ty as #schema_mod::FromType<#name>>::from_type()).unwrap();
            }
        });
        let add_type_datas = input
            .attributes()
            .iter()
            .filter(|x| x.path.len() == 1 && x.path[0].to_string() == "type_data")
            .map(|x| x.get_value_tokens())
            .map(|x| x.iter().cloned().collect::<TokenStream2>());

        quote! {
            {
                let tds = #schema_mod::alloc::TypeDatas::default();
                #(#add_derive_type_datas),*
                #(
                    tds.insert(#add_type_datas).unwrap();
                ),*
                tds
            }
        }
    };

    // Collect repr tags
    let mut repr_flags = get_flags_for_attr(&input, "repr");
    repr_flags.iter_mut().for_each(|x| *x = x.to_lowercase());
    let repr_c = repr_flags.iter().any(|x| x == "c");
    let primitive_repr = repr_flags.iter().find_map(|x| match x.as_ref() {
        "u8" => Some(quote!(U8)),
        "u16" => Some(quote!(U16)),
        "u32" => Some(quote!(U32)),
        _ => None,
    });

    // Collect schema flags
    let schema_flags = get_flags_for_attr(&input, "schema");
    let no_clone = schema_flags.iter().any(|x| x.as_str() == "no_clone");
    let no_default = schema_flags.iter().any(|x| x.as_str() == "no_default");
    let is_opaque = schema_flags.iter().any(|x| x.as_str() == "opaque")
        || !(repr_c || primitive_repr.is_some());

    // Get the clone and default functions based on the flags
    let clone_fn = if no_clone {
        quote!(None)
    } else {
        quote!(Some(<Self as #schema_mod::raw_fns::RawClone>::raw_clone))
    };
    let default_fn = if no_default {
        quote!(None)
    } else {
        quote!(Some(<Self as #schema_mod::raw_fns::RawDefault>::raw_default))
    };

    // Get the schema kind
    let schema_kind = (|| {
        if is_opaque {
            return quote! {
                {
                    let layout = ::std::alloc::Layout::new::<Self>();
                    #schema_mod::SchemaKind::Primitive(#schema_mod::Primitive::Opaque {
                        size: layout.size(),
                        align: layout.align(),
                    })
                }
            };
        }

        // Helper to parse struct fields from structs or enum variants
        let parse_struct_fields = |fields: &StructFields| {
            match fields {
                venial::StructFields::Tuple(tuple) => tuple
                    .fields
                    .iter()
                    .map(|(field, _)| {
                        let ty = &field.ty;
                        quote_spanned! {field.ty.__span() =>
                            #schema_mod::StructFieldInfo {
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
                                #schema_mod::StructFieldInfo {
                                    name: Some(stringify!(#name).into()),
                                    schema: {
                                        let layout = ::std::alloc::Layout::new::<#ty>();
                                        #schema_mod::registry::SCHEMA_REGISTRY.register(#schema_mod::SchemaData {
                                            name: stringify!(#ty).into(),
                                            full_name: concat!(module_path!(), "::", stringify!(#ty)).into(),
                                            kind: #schema_mod::SchemaKind::Primitive(#schema_mod::Primitive::Opaque {
                                                size: layout.size(),
                                                align: layout.align(),
                                            }),
                                            type_id: Some(std::any::TypeId::of::<#ty>()),
                                            type_data: #type_datas,
                                            clone_fn: #clone_fn,
                                            default_fn: #default_fn,
                                            eq_fn: None,
                                            hash_fn: None,
                                            drop_fn: Some(<Self as #schema_mod::raw_fns::RawDrop>::raw_drop),
                                        })
                                    },
                                }
                            }
                        } else {
                            quote_spanned! {field.ty.__span() =>
                                #schema_mod::StructFieldInfo {
                                    name: Some(stringify!(#name).into()),
                                    schema: <#ty as #schema_mod::HasSchema>::schema(),
                                }
                            }
                        }
                    })
                    .collect::<Vec<_>>(),
                venial::StructFields::Unit => Vec::new(),
            }
        };

        // Match on the the type we are deriving on and return its SchemaData
        match &input {
            venial::Declaration::Struct(s) => {
                let fields = parse_struct_fields(&s.fields);

                quote! {
                    #schema_mod::SchemaKind::Struct(#schema_mod::StructSchemaInfo {
                        fields: vec![
                            #(#fields),*
                        ]
                    })
                }
            }
            venial::Declaration::Enum(e) => {
                let Some(tag_type) = primitive_repr else {
                    throw!(
                        e,
                        "Enums deriving HasSchema with a `#[repr(C)]` annotation \
                        must also specify an enum tag type like `#[repr(C, u8)]` where \
                        `u8` could be either `u16` or `u32` if you need more than 256 enum \
                        variants."
                    );
                };
                let mut variants = Vec::new();

                for v in e.variants.items() {
                    let name = v.name.to_string();
                    let variant_schema_name = format!("{}::{}", e.name, name);
                    let fields = parse_struct_fields(&v.contents);
                    variants.push(quote! {
                        #schema_mod::VariantInfo {
                            name: #name.into(),
                            schema: {
                                static S: ::std::sync::OnceLock<&'static #schema_mod::Schema> = ::std::sync::OnceLock::new();
                                S.get_or_init(|| {
                                    #schema_mod::registry::SCHEMA_REGISTRY.register(#schema_mod::SchemaData {
                                        name: #variant_schema_name.into(),
                                        full_name: concat!(module_path!(), "::", #variant_schema_name).into(),
                                        type_id: None,
                                        kind: #schema_mod::SchemaKind::Struct(#schema_mod::StructSchemaInfo {
                                            fields: vec![
                                                #(#fields),*
                                            ]
                                        }),
                                        type_data: Default::default(),
                                        default_fn: None,
                                        clone_fn: None,
                                        eq_fn: None,
                                        hash_fn: None,
                                        drop_fn: None,
                                    })
                                })
                            }
                        }
                    })
                }

                quote! {
                    #schema_mod::SchemaKind::Enum(#schema_mod::EnumSchemaInfo {
                        tag_type: #schema_mod::EnumTagType::#tag_type,
                        variants: vec![#(#variants),*],
                    })
                }
            }
            _ => {
                throw!(
                    input,
                    "You may only derive HasSchema for structs and enums."
                );
            }
        }
    })();

    let schema_register = quote! {
        #schema_mod::registry::SCHEMA_REGISTRY.register(#schema_mod::SchemaData {
            name: stringify!(#name).into(),
            full_name: concat!(module_path!(), "::", stringify!(#name)).into(),
            type_id: Some(::std::any::TypeId::of::<Self>()),
            kind: #schema_kind,
            type_data: #type_datas,
            default_fn: #default_fn,
            clone_fn: #clone_fn,
            eq_fn: None,
            hash_fn: None,
            drop_fn: Some(<Self as #schema_mod::raw_fns::RawDrop>::raw_drop),
        })
    };

    if let Some(generic_params) = input.generic_params() {
        let mut sync_send_generic_params = generic_params.clone();
        for (param, _) in sync_send_generic_params.params.iter_mut() {
            let clone_bound = if !no_clone { quote!(+ Clone) } else { quote!() };
            param.bound = Some(GenericBound {
                tk_colon: Punct::new(':', Spacing::Joint),
                tokens: quote!(HasSchema #clone_bound ).into_iter().collect(),
            });
        }
        quote! {
            unsafe impl #sync_send_generic_params #schema_mod::HasSchema for #name #generic_params {
                fn schema() -> &'static #schema_mod::Schema {
                    // TODO: use faster hashmap and rwlocks from bones_utils.
                    use ::std::sync::{OnceLock};
                    use ::std::any::TypeId;
                    use bones_utils::{HashMap, parking_lot::RwLock};
                    static S: OnceLock<RwLock<HashMap<TypeId, &'static Schema>>> = OnceLock::new();
                    let schema = {
                        S.get_or_init(Default::default)
                            .read()
                            .get(&TypeId::of::<Self>())
                            .copied()
                    };
                    schema.unwrap_or_else(|| {
                        let schema = #schema_register;

                        S.get_or_init(Default::default)
                            .write()
                            .insert(TypeId::of::<Self>(), schema);

                        schema
                    })

                }
            }
        }
    } else {
        quote! {
            unsafe impl #schema_mod::HasSchema for #name {
                fn schema() -> &'static #schema_mod::Schema {
                    static S: ::std::sync::OnceLock<&'static #schema_mod::Schema> = ::std::sync::OnceLock::new();
                    S.get_or_init(|| {
                        #schema_register
                    })
                }
            }
        }
    }
    .into()
}

//
// Helpers
//

/// Look for an attribute with the given name and get all of the comma-separated flags that are
/// in that attribute.
///
/// For example, with the given struct:
///
/// ```ignore
/// #[example(test)]
/// #[my_attr(hello, world)]
/// struct Hello;
/// ```
///
/// Calling `get_flags_for_attr("my_attr")` would return `vec!["hello", "world"]`.
fn get_flags_for_attr(input: &venial::Declaration, attr_name: &str) -> Vec<String> {
    let attrs = input
        .attributes()
        .iter()
        .filter(|attr| attr.path.len() == 1 && attr.path[0].to_string() == attr_name)
        .collect::<Vec<_>>();
    attrs
        .iter()
        .map(|attr| match &attr.value {
            venial::AttributeValue::Group(_, value) => {
                let mut flags = Vec::new();

                let mut current_flag = proc_macro2::TokenStream::new();
                for token in value {
                    match token {
                        TokenTree2::Punct(x) if x.as_char() == ',' => {
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
                // TODO: Improve macro error message span.
                panic!("Unsupported attribute format");
            }
            venial::AttributeValue::Empty => Vec::new(),
        })
        .fold(Vec::new(), |mut acc, item| {
            acc.extend(item);
            acc
        })
}
