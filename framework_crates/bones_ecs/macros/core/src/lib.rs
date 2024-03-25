use proc_macro2::TokenStream;
use syn::{Fields, ItemStruct};

pub fn generate_system_param_impl(input: TokenStream) -> TokenStream {
    match _generate_system_param_impl(input) {
        Ok(output) => output,
        Err(err) => err.to_compile_error(),
    }
}

macro_rules! err {
    ($target:expr, $message:expr) => {
        return Err(::syn::Error::new(
            ::syn::spanned::Spanned::span(&$target),
            $message,
        ))
    };
}

fn _generate_system_param_impl(input: TokenStream) -> syn::Result<TokenStream> {
    let item_struct: ItemStruct = syn::parse2(input)?;

    match item_struct.fields {
        Fields::Unit => err!(item_struct, "unit structs are not supported"),
        Fields::Unnamed(_) => err!(item_struct, "structs with unnamed fields are not supported"),
        Fields::Named(_) => {}
    }

    Ok(TokenStream::default())
}
