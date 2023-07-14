use proc_macro2::TokenStream;
use syn::{parse_macro_input, ItemFn};

mod error;
mod parse;

#[proc_macro_attribute]
pub fn range_kernel(attr: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let attr = parse_macro_input!(attr as RangeAttributes);
    let item = parse_macro_input!(item as RangeFn);
    let result = emit_range_kernel(attr, item);
    into_token_stream(result)
}

type TokenResult = Result<TokenStream, TokenStream>;

fn into_token_stream(result: TokenResult) -> proc_macro::TokenStream {
    match result {
        Ok(output) => proc_macro::TokenStream::from(output),
        Err(error) => proc_macro::TokenStream::from(error),
    }
}

struct RangeAttributes;
struct RangeFn(syn::ItemFn);
struct CudaCrate {
    name: String,
}

fn emit_range_kernel(attr: RangeAttributes, item: RangeFn) -> TokenResult {
    todo!("got here")
}
