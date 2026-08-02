use aidoku::{
	AidokuError, Result,
	alloc::String,
	imports::{js::WebView, net::Request, std::sleep},
	prelude::*,
};

#[cfg(not(test))]
use aidoku::imports::js::WebViewUserScript;

#[cfg(not(test))]
use crate::generate_session;
use crate::{ACCEPT_LANGUAGE, ApiChapterDetails};

use super::{
	WEBVIEW_CHAPTER_CACHE_GLOBAL_KEY, WEBVIEW_CHAPTER_LOAD_ATTEMPTS,
	WEBVIEW_CHAPTER_LOAD_DELAY_SECONDS, WEBVIEW_USER_AGENT,
	webview_support::{
		WebViewChapterCache, build_webview_cookie_header, fetch_webview_debug_snapshot,
		force_webview_visible_layout, log_webview_debug_snapshot, shorten_for_log,
		update_webview_cookie_cache,
	},
};

#[cfg(not(test))]
use super::webview_support::{
	build_webview_instrumentation_config, build_webview_user_script, cookie_value_from_header,
};

pub(crate) fn webview_chapter_storage_key(manga_id: &str, chapter_id: &str) -> String {
	format!("toonlivre_chapter_cache_v1:{manga_id}:{chapter_id}")
}

pub(crate) fn parse_webview_chapter_cache(
	value: &str,
	storage_key: &str,
) -> Result<ApiChapterDetails> {
	let cache: WebViewChapterCache = serde_json::from_str(value).map_err(|error| {
		AidokuError::Message(format!(
			"Failed to parse chapter cache from WebView.\nKey: {storage_key}\nError: {error}"
		))
	})?;

	if cache.chapter.pages.is_empty() {
		bail!("WebView chapter cache does not contain pages.\nKey: {storage_key}");
	}

	Ok(cache.chapter)
}

pub(super) fn fetch_chapter_via_webview(
	chapter_url: &str,
	manga_id: &str,
	chapter_id: &str,
) -> Result<ApiChapterDetails> {
	source_log!(
		"[toonlivre] fetch_chapter_via_webview start url={} manga_id={} chapter_id={}",
		chapter_url,
		manga_id,
		chapter_id
	);

	let storage_key = webview_chapter_storage_key(manga_id, chapter_id);
	let cookie_header = build_webview_cookie_header();
	let webview = WebView::new();

	install_webview_user_script(
		&webview,
		chapter_url,
		manga_id,
		chapter_id,
		&storage_key,
		&cookie_header,
	)?;

	source_log!(
		"[toonlivre] webview cookie header prepared size={} value={}",
		cookie_header.len(),
		shorten_for_log(&cookie_header, 240)
	);

	load_chapter_page(&webview, chapter_url, &cookie_header)?;
	force_webview_visible_layout(&webview)?;
	source_log!("[toonlivre] webview visibility/layout forced");
	log_current_snapshot(&webview, "webview after load")?;
	wait_for_chapter_cache(&webview, chapter_url, manga_id, chapter_id, &storage_key)
}

fn read_cached_webview_chapter(
	webview: &WebView,
	chapter_url: &str,
	storage_key: &str,
) -> Result<Option<ApiChapterDetails>> {
	let script = format!(
		"window[{global_key:?}] || sessionStorage.getItem({storage_key:?}) || \"\"",
		global_key = WEBVIEW_CHAPTER_CACHE_GLOBAL_KEY,
		storage_key = storage_key,
	);
	let raw = webview.eval(&script).map_err(|error| {
		AidokuError::Message(format!(
			"WebView chapter eval failed.\nURL: {chapter_url}\nKey: {storage_key}\nError: {error:?}"
		))
	})?;
	let value = raw.trim();
	if value.is_empty() || value == "null" || value == "undefined" {
		return Ok(None);
	}

	parse_webview_chapter_cache(value, storage_key).map(Some)
}

fn load_chapter_page(webview: &WebView, chapter_url: &str, cookie_header: &str) -> Result<()> {
	let request = Request::get(chapter_url)?
		.header(
			"Accept",
			"text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
		)
		.header("User-Agent", WEBVIEW_USER_AGENT)
		.header("Accept-Language", ACCEPT_LANGUAGE)
		.header("Referer", crate::BASE_URL)
		.header("Cookie", cookie_header);

	source_log!("[toonlivre] Loading chapter URL: {}", chapter_url);
	webview.load_blocking(request).map_err(|error| {
		AidokuError::Message(format!(
			"WebView chapter load failed.\nURL: {chapter_url}\nError: {error:?}"
		))
	})?;
	Ok(())
}

fn wait_for_chapter_cache(
	webview: &WebView,
	chapter_url: &str,
	manga_id: &str,
	chapter_id: &str,
	storage_key: &str,
) -> Result<ApiChapterDetails> {
	for attempt in 1..=WEBVIEW_CHAPTER_LOAD_ATTEMPTS {
		if let Some(chapter) = read_cached_webview_chapter(webview, chapter_url, storage_key)? {
			source_log!(
				"[toonlivre] webview chapter cache ready attempt={} key={} pages={}",
				attempt,
				storage_key,
				chapter.pages.len()
			);
			return Ok(chapter);
		}
		log_current_snapshot(webview, &format!("webview wait attempt={attempt}"))?;
		if attempt < WEBVIEW_CHAPTER_LOAD_ATTEMPTS {
			sleep(WEBVIEW_CHAPTER_LOAD_DELAY_SECONDS);
		}
	}

	bail!(
		"WebView chapter cache not populated. URL: {chapter_url} manga_id={manga_id} chapter_id={chapter_id}"
	)
}

fn log_current_snapshot(webview: &WebView, label: &str) -> Result<()> {
	let snapshot = fetch_webview_debug_snapshot(webview)?;
	update_webview_cookie_cache(&snapshot.cookie);
	log_webview_debug_snapshot(label, &snapshot);
	Ok(())
}

#[cfg(not(test))]
fn install_webview_user_script(
	webview: &WebView,
	chapter_url: &str,
	manga_id: &str,
	chapter_id: &str,
	storage_key: &str,
	cookie_header: &str,
) -> Result<()> {
	let chapter_number_hint = String::from(chapter_url.rsplit('/').next().unwrap_or(chapter_id));
	let toon_v_cookie =
		cookie_value_from_header(cookie_header, "toon_v").unwrap_or_else(generate_session);
	let config = build_webview_instrumentation_config(
		storage_key,
		manga_id,
		chapter_id,
		&chapter_number_hint,
		&toon_v_cookie,
	);
	let user_script = WebViewUserScript {
		source: build_webview_user_script(&config)?,
		at_document_end: false,
		for_main_frame_only: true,
	};

	if let Err(_error) = webview.add_user_script(user_script) {
		source_log!(
			"[toonlivre] Failed to add visibility patch user script: {:?}",
			_error
		);
	}
	Ok(())
}

#[cfg(test)]
fn install_webview_user_script(
	_webview: &WebView,
	_chapter_url: &str,
	_manga_id: &str,
	_chapter_id: &str,
	_storage_key: &str,
	_cookie_header: &str,
) -> Result<()> {
	Ok(())
}
