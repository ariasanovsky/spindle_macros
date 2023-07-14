use std::fmt::Display;

use proc_macro2::{TokenStream, Ident};
use syn::Generics;

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