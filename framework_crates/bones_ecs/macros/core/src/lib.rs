use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{parse2, punctuated::Punctuated, Fields, GenericParam, Index, ItemStruct, Token};

macro_rules! err {
    ($target:expr, $message:expr) => {
        return Err(::syn::Error::new(
            ::syn::spanned::Spanned::span(&$target),
            $message,
        ))
    };
}

pub fn generate_system_param_impl(input: TokenStream) -> TokenStream {
    match _generate_system_param_impl(input) {
        Ok(output) => output,
        Err(err) => err.to_compile_error(),
    }
}

fn _generate_system_param_impl(input: TokenStream) -> syn::Result<TokenStream> {
    let item_struct: ItemStruct = parse2(input)?;

    let Some(GenericParam::Lifetime(lifetime)) =
        get_single_punctuated(&item_struct.generics.params)
    else {
        err!(
            item_struct,
            "struct must have a single generic lifetime parameter"
        );
    };

    let ident = &item_struct.ident;

    let fields = match &item_struct.fields {
        Fields::Unit => err!(item_struct, "unit structs are not supported"),
        Fields::Unnamed(_) => err!(item_struct, "structs with unnamed fields are not supported"),
        Fields::Named(fields) => fields,
    };

    let state_types: Punctuated<TokenStream, Token![,]> =
        Punctuated::from_iter(fields.named.iter().map(|field| {
            let ty = &field.ty;
            quote! { <#ty as ::bones_framework::prelude::SystemParam>::State }
        }));

    let get_state_items: Punctuated<TokenStream, Token![,]> =
        Punctuated::from_iter(fields.named.iter().map(|field| {
            let ty = &field.ty;
            quote! { <#ty as ::bones_framework::prelude::SystemParam>::get_state(world) }
        }));

    let borrow_param_fields: Punctuated<TokenStream, Token![,]> = fields
        .named
        .iter()
        .enumerate()
        .map(|(index, field)| {
            let ident = field.ident.as_ref().unwrap();
            let ty = &field.ty;
            let index = Index { index: index as u32, span: Span::call_site() };
            quote! {
                #ident: <#ty as ::bones_framework::prelude::SystemParam>::borrow(world, &mut state.#index)
            }
        })
        .collect();

    Ok(quote! {
        impl<#lifetime> ::bones_framework::prelude::SystemParam for #ident<#lifetime> {
            type State = ( #state_types );
            type Param<'p> = #ident<'p>;
            fn get_state(world: &::bones_framework::prelude::World) -> Self::State {
                ( #get_state_items )
            }
            fn borrow<'s>(
                world: &'s ::bones_framework::prelude::World,
                state: &'s mut Self::State,
            ) -> Self::Param<'s> {
                Self::Param { #borrow_param_fields }
            }
        }
    })
}

fn get_single_punctuated<T, P>(punctuated: &Punctuated<T, P>) -> Option<&T> {
    match punctuated.first() {
        single @ Some(_) if punctuated.len() == 1 => single,
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use quote::quote;

    use super::*;

    fn assert_tokens_eq(expected: TokenStream, actual: TokenStream) {
        let expected = expected.to_string();
        let actual = actual.to_string();
        assert_eq!(expected, actual);
    }

    #[test]
    fn correct_system_param_impl() {
        let expected = quote! {
            impl<'a> ::bones_framework::prelude::SystemParam for MySystemParam<'a> {
                type State = (
                    <Commands<'a> as ::bones_framework::prelude::SystemParam>::State,
                    <ResMut<'a, Entities> as ::bones_framework::prelude::SystemParam>::State
                );
                type Param<'p> = MySystemParam<'p>;
                fn get_state(world: &::bones_framework::prelude::World) -> Self::State {
                    (
                        <Commands<'a> as ::bones_framework::prelude::SystemParam>::get_state(world),
                        <ResMut<'a, Entities> as ::bones_framework::prelude::SystemParam>::get_state(world)
                    )
                }
                fn borrow<'s>(
                    world: &'s ::bones_framework::prelude::World,
                    state: &'s mut Self::State,
                ) -> Self::Param<'s> {
                    Self::Param {
                        commands: <Commands<'a> as ::bones_framework::prelude::SystemParam>::borrow(world, &mut state.0),
                        entities: <ResMut<'a, Entities> as ::bones_framework::prelude::SystemParam>::borrow(world, &mut state.1)
                    }
                }
            }
        };
        let input = quote! {
            struct MySystemParam<'a> {
                commands: Commands<'a>,
                entities: ResMut<'a, Entities>,
            }
        };
        let actual = generate_system_param_impl(input);
        assert_tokens_eq(expected, actual);
    }
}
