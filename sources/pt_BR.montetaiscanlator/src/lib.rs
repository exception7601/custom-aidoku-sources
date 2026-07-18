#![no_std]
#[allow(unused_imports)]
use aidoku::{DeepLinkHandler, ImageRequestProvider, Source, prelude::*};

#[cfg(test)]
macro_rules! source_log {
	($($arg:tt)*) => {
		println!($($arg)*)
	};
}

#[cfg(not(test))]
macro_rules! source_log {
	($($arg:tt)*) => {};
}

pub(crate) const BASE_URL: &str = "https://montetaiscanlator.xyz";

mod scraper;
mod source;
mod utils;

pub(crate) use scraper::*;
pub(crate) use source::MonteTaiScanlator;
#[cfg(test)]
pub(crate) use source::update_manga_from_document;
pub(crate) use utils::*;

register_source!(MonteTaiScanlator, DeepLinkHandler, ImageRequestProvider);

#[cfg(test)]
mod tests;
