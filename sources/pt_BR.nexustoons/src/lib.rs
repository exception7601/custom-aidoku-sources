#![no_std]
#[allow(unused_imports)]
use aidoku::{DeepLinkHandler, Home, ImageRequestProvider, Source, prelude::*};

#[cfg(any(test, debug_assertions))]
macro_rules! source_log {
	($($arg:tt)*) => {
		::aidoku::prelude::println!($($arg)*)
	};
}

#[cfg(not(any(test, debug_assertions)))]
macro_rules! source_log {
	($($arg:tt)*) => {};
}

pub(crate) const BASE_URL: &str = "https://nexustoons.com";
pub(crate) const ACCEPT_LANGUAGE: &str = "pt-BR,pt;q=0.9";

mod api;
mod source;
mod utils;

pub(crate) use api::*;
pub(crate) use source::{NexusToons, map_list_response};
pub(crate) use utils::*;

register_source!(NexusToons, Home, DeepLinkHandler, ImageRequestProvider);

#[cfg(test)]
mod tests;
