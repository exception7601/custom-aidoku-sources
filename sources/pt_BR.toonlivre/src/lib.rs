#![no_std]
#[allow(unused_imports)]
use aidoku::{DeepLinkHandler, Home, ImageRequestProvider, Source, prelude::*};

pub(crate) const BASE_URL: &str = "https://toonlivre.net";
pub(crate) const ACCEPT_LANGUAGE: &str = "en-US,en;q=0.9,pt;q=0.8";

mod api;
mod source;
mod utils;

pub(crate) use api::*;
pub(crate) use source::ToonLivre;
pub(crate) use utils::*;

register_source!(ToonLivre, DeepLinkHandler, Home, ImageRequestProvider);

#[cfg(test)]
mod tests;
