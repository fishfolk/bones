use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, quote_spanned};

/// Derive macro for the `HasTypeRegistration` trait.
#[proc_macro_derive(HasTypeRegistration)]
pub fn has_type_registration(input: TokenStream) -> TokenStream {
    let input = syn::parse(input).unwrap();
    impl_has_type_registration(&input).into()
}

fn impl_has_type_registration(input: &syn::DeriveInput) -> TokenStream2 {
    let item_ident = &input.ident;

    let module_ident = format_ident!(
        "{}_derive_has_type_registration",
        item_ident.to_string().to_lowercase()
    );

    // Parse the struct
    let in_struct = match &input.data {
        syn::Data::Struct(s) => s,
        syn::Data::Enum(_) | syn::Data::Union(_) => {
            return quote_spanned! { input.ident.span() =>
                compile_error!("You may only derive HasTypeRegistration on structs");
            };
        }
    };

    quote! {
        mod #module_ident {
            use super::#item_ident;

            impl ::bones_reflectish::HasTypeRegistration for #item_ident {
                fn layout() -> std::alloc::Layout {
                    std::alloc::Layout::new::<#item_ident>()
                }

                fn drop_fn() -> Option<unsafe extern "C" fn(*mut u8)> {
                    #item_ident::drop_fn()
                }

                fn clone_fn() -> unsafe extern "C" fn(*const u8, *mut u8) {
                    #item_ident::clone_fn()
                }

                fn type_name(&self) -> &str {
                    #item_ident::type_name(self)
                }
            }
        }
    }
}
