use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, quote_spanned};
use syn::{parse_quote, spanned::Spanned};

/// Derive macro for the `BonesBevyAsset` trait.
#[proc_macro_derive(BonesBevyAsset, attributes(asset_id, asset))]
pub fn bones_bevy_asset(input: TokenStream) -> TokenStream {
    let input = syn::parse(input).unwrap();

    impl_bones_bevy_asset(&input).into()
}

fn impl_bones_bevy_asset(input: &syn::DeriveInput) -> TokenStream2 {
    let deserialize_only: syn::Attribute = parse_quote! {
        #[asset(deserialize_only)]
    };
    let item_ident = &input.ident;

    let mut asset_id = None;
    for attr in &input.attrs {
        let Ok(syn::Meta::NameValue(name_value)) = attr.parse_meta() else {
            continue;
        };

        if name_value
            .path
            .get_ident()
            .map(|i| i != "asset_id")
            .unwrap_or(true)
        {
            continue;
        }

        let syn::Lit::Str(lit_str) = name_value.lit else {
            continue;
        };

        asset_id = Some(lit_str.value());
    }

    let Some(asset_id) = asset_id else {
        return quote! {
            compile_error!("You must specify a `asset_id` attribute");
        };
    };

    let module_ident = format_ident!(
        "{}_derive_bevy_asset",
        item_ident.to_string().to_lowercase()
    );

    // Parse the struct
    let in_struct = match &input.data {
        syn::Data::Struct(s) => s,
        syn::Data::Enum(_) | syn::Data::Union(_) => {
            return quote_spanned! { input.ident.span() =>
                compile_error!("You may only derive HasLoadProgress on structs");
            };
        }
    };

    let mut field_loads = Vec::new();
    'field: for field in &in_struct.fields {
        // Skip this field if it has `#[has_load_progress(none)]`
        for attr in &field.attrs {
            if attr.path == parse_quote!(asset) {
                if attr != &deserialize_only {
                    field_loads.push(quote_spanned! { attr.span() =>
                        compile_error!("Attribute must be `#[asset(deserialize_only)]` if specified");
                    });
                }
                continue 'field;
            }
        }
        let field_ident = field.ident.as_ref().expect("Field identifier missing");
        field_loads.push(quote_spanned! { field_ident.span() =>
            ::bones_bevy_asset::BonesBevyAssetLoad::load(
                &mut meta.#field_ident,
                load_context,
                &mut dependencies
            );
        });
    }

    quote! {
        mod #module_ident {
            use ::type_ulid::TypeUlid;
            use ::bevy::asset::AddAsset;
            use super::#item_ident;

            // Make sure `TypeUlid` is implemented
            trait RequiredBounds: type_ulid::TypeUlid + for<'de> ::serde::Deserialize<'de> {}
            impl RequiredBounds for #item_ident {}

            impl ::bevy::reflect::TypeUuid for #item_ident {
                const TYPE_UUID: bevy::reflect::Uuid = bevy::reflect::Uuid::from_u128(Self::ULID.0);
            }

            struct AssetLoader;
            impl ::bevy::asset::AssetLoader for AssetLoader {
                fn load<'a>(
                    &'a self,
                    bytes: &'a [u8],
                    load_context: &'a mut bevy::asset::LoadContext,
                ) -> bevy::utils::BoxedFuture<'a, Result<(), bevy::asset::Error>> {
                    Box::pin(async move {
                        let mut dependencies = Vec::new();
                        let mut meta: #item_ident =
                            if load_context.path().extension() == Some(std::ffi::OsStr::new("json")) {
                                ::bones_bevy_asset::_private::serde_json::from_slice(bytes)?
                            } else {
                                ::bones_bevy_asset::_private::serde_yaml::from_slice(bytes)?
                            };

                        #(#field_loads)*

                        load_context.set_default_asset(
                            bevy::asset::LoadedAsset::new(meta)
                                .with_dependencies(dependencies)
                        );

                        Ok(())
                    })
                }

                fn extensions(&self) -> &[&str] {
                    &[
                        concat!(#asset_id, ".json"),
                        concat!(#asset_id, ".yaml"),
                        concat!(#asset_id, ".yml"),
                    ]
                }
            }

            impl ::bones_bevy_asset::BonesBevyAsset for #item_ident {
                fn install_asset(app: &mut ::bevy::app::App) {
                    app
                        .add_asset::<Self>()
                        .add_asset_loader(AssetLoader);
                }
            }
        }
    }
}

/// Derive macro for the `BonesBevyAssetLoad` trait.
#[proc_macro_derive(BonesBevyAssetLoad, attributes(asset))]
pub fn bones_bevy_asset_load(input: TokenStream) -> TokenStream {
    let input = syn::parse(input).unwrap();

    impl_bones_bevy_asset_load(&input).into()
}

fn impl_bones_bevy_asset_load(input: &syn::DeriveInput) -> TokenStream2 {
    let deserialize_only: syn::Attribute = parse_quote! {
        #[asset(deserialize_only)]
    };
    let item_ident = &input.ident;

    // Parse the struct
    let mut field_loads = Vec::new();
    match &input.data {
        syn::Data::Struct(s) => {
            'field: for field in &s.fields {
                // Skip this field if it has `#[has_load_progress(none)]`
                for attr in &field.attrs {
                    if attr.path == parse_quote!(asset) {
                        if attr != &deserialize_only {
                            field_loads.push(quote_spanned! { attr.span() =>
                        compile_error!("Attribute must be `#[asset(deserialize_only)]` if specified");
                    });
                        }
                        continue 'field;
                    }
                }
                let field_ident = field.ident.as_ref().expect("Field identifier missing");
                field_loads.push(quote_spanned! { field_ident.span() =>
                    ::bones_bevy_asset::BonesBevyAssetLoad::load(
                        &mut self.#field_ident,
                        load_context,
                        dependencies
                    );
                });
            }
        }
        syn::Data::Enum(e) => {
            let mut patterns = Vec::new();
            for variant in &e.variants {
                let variant_ident = &variant.ident;
                match &variant.fields {
                    syn::Fields::Named(fields) => {
                        let ids = fields
                            .named
                            .iter()
                            .map(|x| x.ident.as_ref().expect("Field without ident"))
                            .collect::<Vec<_>>();
                        let loads = ids.iter().map(|id| {
                            quote! {
                                ::bones_bevy_asset::BonesBevyAssetLoad::load(#id, load_context, dependencies);
                            }
                        });

                        patterns.push(quote! {
                            Self::#variant_ident { #(#ids,)* } => {
                                #(#loads)*
                            }
                        });
                    }
                    syn::Fields::Unnamed(fields) => {
                        let ids = fields
                            .unnamed
                            .iter()
                            .enumerate()
                            .map(|(i, _)| format_ident!("field_{}", i))
                            .collect::<Vec<_>>();
                        let loads = ids.iter().map(|id| {
                            quote! {
                                ::bones_bevy_asset::BonesBevyAssetLoad::load(#id, load_context, dependencies);
                            }
                        });

                        patterns.push(quote! {
                            Self::#variant_ident(#(#ids)*) => {
                                #(#loads)*
                            }
                        });
                    }
                    syn::Fields::Unit => patterns.push(quote! {
                        Self::#variant_ident => (),
                    }),
                }
            }

            field_loads.push(quote! {
                match self {
                    #(#patterns)*
                }
            });
        }
        syn::Data::Union(_) => {
            return quote_spanned! { input.ident.span() =>
                compile_error!("Deriving not supported on unions");
            };
        }
    };

    quote! {
        impl ::bones_bevy_asset::BonesBevyAssetLoad for #item_ident {
            fn load(
                &mut self,
                load_context: &mut bevy::asset::LoadContext,
                dependencies: &mut Vec<bevy::asset::AssetPath<'static>>,
            ) {
                #(#field_loads)*
            }
        }
    }
}
