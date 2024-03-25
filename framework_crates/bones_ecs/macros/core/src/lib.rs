use proc_macro2::TokenStream;
use syn::ItemStruct;

pub fn generate_system_param_impl(input: TokenStream) -> TokenStream {
    match _generate_system_param_impl(input) {
        Ok(output) => output,
        Err(err) => err.to_compile_error(),
    }
}

fn _generate_system_param_impl(input: TokenStream) -> syn::Result<TokenStream> {
    syn::parse2::<ItemStruct>(input)?;
    Ok(TokenStream::default())
}
