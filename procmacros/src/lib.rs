//! Procedural macros for Dipstick, a metric library for Rust.
//!
//! Please check the Dipstick crate for more documentation.

#![recursion_limit = "512"]

extern crate proc_macro;

#[macro_use]
extern crate syn;

#[macro_use]
extern crate quote;

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn timed(attrs: TokenStream, item: TokenStream) -> TokenStream {
    item
}
