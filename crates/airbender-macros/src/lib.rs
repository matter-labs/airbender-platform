#![doc = include_str!("../README.md")]

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    spanned::Spanned,
    Error, ItemFn, ReturnType, Token, Type,
};

struct MainArgs {
    allocator_init: Option<syn::Path>,
}

impl Parse for MainArgs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        if input.is_empty() {
            return Ok(Self {
                allocator_init: None,
            });
        }

        let key: syn::Ident = input.parse()?;
        if key != "allocator_init" {
            return Err(Error::new(
                key.span(),
                "unsupported argument; expected `allocator_init = <path>`",
            ));
        }
        input.parse::<Token![=]>()?;
        let allocator_init = input.parse::<syn::Path>()?;

        if !input.is_empty() {
            return Err(Error::new(
                input.span(),
                "unexpected trailing tokens in `airbender::main` arguments",
            ));
        }

        Ok(Self {
            allocator_init: Some(allocator_init),
        })
    }
}

#[proc_macro_attribute]
pub fn main(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as MainArgs);
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

    let guest_entry = quote! {
        let output = #fn_name();
        ::airbender::guest::commit(output)
    };

    let start_call = if let Some(allocator_init) = args.allocator_init {
        quote! {
            ::airbender::rt::start_with_allocator_init(#allocator_init, || {
                #guest_entry
            })
        }
    } else {
        quote! {
            ::airbender::rt::start(|| {
                #guest_entry
            })
        }
    };

    let expanded = quote! {
        #input

        #[no_mangle]
        #[export_name = "_start_rust"]
        pub extern "C" fn #wrapper_name() -> ! {
            #start_call
        }
    };

    expanded.into()
}
