use std::fmt::Display;

use proc_macro2::{TokenStream, Ident};
use syn::Generics;

use crate::TokenResult;

pub(super) enum Error<'a> {
    _UnexpectedAttribute(&'a Ident),
    _UnexpectedGenerics(&'a Generics),
}

impl From<Error<'_>> for TokenStream {
    fn from(value: Error<'_>) -> Self {
        todo!()
    }
}

impl Display for Error<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

pub(super) trait NaivelyTokenize: Sized + ToString {
    fn naively_tokenize(self) -> TokenStream {
        let s = self.to_string();
        quote::quote! { #s }
    }
}

impl NaivelyTokenize for std::io::Error {}
impl NaivelyTokenize for serde_json::Error {}
