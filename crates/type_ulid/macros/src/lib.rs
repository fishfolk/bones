use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;

/// Derive macro for the `TypeUlid` trait.
///
/// # Example
///
/// ```ignore
/// #[derive(TypeUlid)]
/// #[ulid = "01GNDEY1ZEC7BGREZNG2JNTPRP"]
/// struct MyStruct;
/// ```
#[proc_macro_derive(TypeUlid, attributes(ulid))]
pub fn type_ulid(input: TokenStream) -> TokenStream {
    let input = syn::parse(input).unwrap();

    impl_type_ulid(&input).into()
}

fn impl_type_ulid(input: &syn::DeriveInput) -> TokenStream2 {
    let item_ident = &input.ident;

    // Check for `#[ulid = "ulid"]` attribute
    let mut ulid = None;
    for attr in &input.attrs {
        let Ok(syn::Meta::NameValue(name_value)) = attr.parse_meta() else {
            continue;
        };

        if name_value
            .path
            .get_ident()
            .map(|i| i != "ulid")
            .unwrap_or(true)
        {
            continue;
        }

        let syn::Lit::Str(lit_str) = name_value.lit else {
            continue;
        };

        match ulid::Ulid::from_string(&lit_str.value()) {
            Ok(id) => ulid = Some(id),
            Err(e) => {
                let msg = e.to_string();
                return quote_spanned! { attr.span() =>
                    compile_error!(concat!("Could not parse ULID: ", #msg));
                };
            }
        }
    }

    let Some(ulid) = ulid else {
        return quote! {
            compile_error!("You must specify a `ulid` attribute");
        };
    };

    // Add impl block
    let id = ulid.0;
    quote! {
        impl ::type_ulid::TypeUlid for #item_ident {
            fn ulid() -> ::type_ulid::Ulid {
                ::type_ulid::Ulid(#id)
            }
        }
    }
}
