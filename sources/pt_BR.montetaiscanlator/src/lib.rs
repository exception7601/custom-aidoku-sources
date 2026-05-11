#![no_std]
#[allow(unused_imports)]
use aidoku::{DeepLinkHandler, ImageRequestProvider, Source, prelude::*};

pub(crate) const BASE_URL: &str = "https://montetaiscanlator.xyz";

mod scraper;
mod source;
mod utils;

pub(crate) use scraper::*;
pub(crate) use source::{MonteTaiScanlator, update_manga_from_document};
pub(crate) use utils::*;

register_source!(MonteTaiScanlator, DeepLinkHandler, ImageRequestProvider);

#[cfg(test)]
mod tests;
