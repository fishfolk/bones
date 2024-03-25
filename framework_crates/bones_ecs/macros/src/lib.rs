use proc_macro::TokenStream;

#[proc_macro_derive(SystemParam)]
pub fn system_param_derive_macro(input: TokenStream) -> TokenStream {
    let input = input.into();
    bones_ecs_macros_core::generate_system_param_impl(input).into()
}
