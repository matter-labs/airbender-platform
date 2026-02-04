//! Procedural macros for Airbender guest programs.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, spanned::Spanned, ItemFn, ReturnType, Type};

#[proc_macro_attribute]
pub fn main(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    if !input.sig.inputs.is_empty() {
        return syn::Error::new(
            input.sig.inputs.span(),
            "airbender::main does not accept arguments",
        )
        .to_compile_error()
        .into();
    }
    if input.sig.asyncness.is_some() {
        return syn::Error::new(
            input.sig.asyncness.span(),
            "airbender::main cannot be async",
        )
        .to_compile_error()
        .into();
    }
    if let ReturnType::Type(_, ty) = &input.sig.output {
        if matches!(**ty, Type::Never(_)) {
            return syn::Error::new(
                ty.span(),
                "airbender::main must return a value implementing Commit (use () if needed)",
            )
            .to_compile_error()
            .into();
        }
    }

    let fn_name = &input.sig.ident;
    let wrapper_name = syn::Ident::new(&format!("__airbender_start_{fn_name}"), fn_name.span());

    let expanded = quote! {
        #input

        #[no_mangle]
        #[export_name = "_start_rust"]
        pub extern "C" fn #wrapper_name() -> ! {
            ::airbender::rt::start(|| {
                let output = #fn_name();
                ::airbender::guest::commit(output)
            })
        }
    };

    expanded.into()
}
