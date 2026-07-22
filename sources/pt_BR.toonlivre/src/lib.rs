#![no_std]
#[allow(unused_imports)]
use aidoku::{DeepLinkHandler, Home, ImageRequestProvider, Source, prelude::*};

#[cfg(any(test, debug_assertions))]
macro_rules! source_log {
	($($arg:tt)*) => {
		println!($($arg)*)
	};
}

#[cfg(not(any(test, debug_assertions)))]
macro_rules! source_log {
	($($arg:tt)*) => {};
}

pub(crate) const BASE_URL: &str = "https://toonlivre.net";
pub(crate) const ACCEPT_LANGUAGE: &str = "pt-BR,pt;q=0.9";

mod api;
mod manifest;
mod source;
mod utils;

pub(crate) use api::*;
pub(crate) use manifest::*;
pub(crate) use source::ToonLivre;
pub(crate) use utils::*;

register_source!(ToonLivre, DeepLinkHandler, Home, ImageRequestProvider);

#[cfg(test)]
mod tests;
