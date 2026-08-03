#![no_std]
#[allow(unused_imports)]
use aidoku::{
	DeepLinkHandler, Home, ImageRequestProvider, Source, alloc::String,
	imports::defaults::defaults_get, prelude::*,
};

use core::cell::RefCell;

const ENABLE_DEBUG_LOGS_KEY: &str = "enable_debug_logs";

struct DebugLogsFlagCache(RefCell<Option<bool>>);

unsafe impl Sync for DebugLogsFlagCache {}

static DEBUG_LOGS_FLAG_CACHE: DebugLogsFlagCache = DebugLogsFlagCache(RefCell::new(None));

#[cfg(any(test, debug_assertions))]
macro_rules! source_log {
  ($($arg:tt)*) => {
    ::aidoku::prelude::println!($($arg)*)
  };
}

#[cfg(not(any(test, debug_assertions)))]
macro_rules! source_log {
  ($($arg:tt)*) => {
    if $crate::debug_logs_enabled() {
      ::aidoku::prelude::println!($($arg)*)
    }
  };
}

pub(crate) fn debug_logs_enabled() -> bool {
	if cfg!(any(test, debug_assertions)) {
		return true;
	}

	if let Some(enabled) = *DEBUG_LOGS_FLAG_CACHE.0.borrow() {
		return enabled;
	}

	let enabled = defaults_get::<i32>(ENABLE_DEBUG_LOGS_KEY).is_some_and(|value| value == 1)
		|| defaults_get::<bool>(ENABLE_DEBUG_LOGS_KEY).unwrap_or(false)
		|| defaults_get::<String>(ENABLE_DEBUG_LOGS_KEY)
			.map(|value| {
				let trimmed = value.trim();
				trimmed == "1"
					|| trimmed.eq_ignore_ascii_case("true")
					|| trimmed.eq_ignore_ascii_case("yes")
					|| trimmed.eq_ignore_ascii_case("on")
			})
			.unwrap_or(false);
	*DEBUG_LOGS_FLAG_CACHE.0.borrow_mut() = Some(enabled);
	enabled
}

pub(crate) const BASE_URL: &str = "https://nexustoons.com";
pub(crate) const ACCEPT_LANGUAGE: &str = "pt-BR,pt;q=0.9";

mod api;
mod source;
mod utils;

pub(crate) use api::*;
#[allow(unused_imports)]
pub(crate) use source::{
	NexusToons, map_list_response, parse_webview_chapter_pages_cache, parse_webview_manga_cache,
	webview_chapter_storage_key, webview_manga_storage_key,
};
pub(crate) use utils::*;

register_source!(NexusToons, Home, DeepLinkHandler, ImageRequestProvider);

#[cfg(test)]
mod tests;
