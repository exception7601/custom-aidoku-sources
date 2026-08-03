use aidoku::{
	AidokuError, Result,
	alloc::{String, Vec},
	imports::{js::WebView, net::Request, std::sleep},
	prelude::*,
};

#[cfg(not(test))]
use aidoku::imports::js::WebViewUserScript;

use crate::ACCEPT_LANGUAGE;

use super::{
	WEBVIEW_CHAPTER_CACHE_GLOBAL_KEY, WEBVIEW_LOAD_ATTEMPTS, WEBVIEW_LOAD_DELAY_SECONDS,
	WEBVIEW_MANGA_CACHE_GLOBAL_KEY, WEBVIEW_USER_AGENT,
	manga::{WebViewMangaDetails, chapter_sort_key_from_webview},
	webview_support::{
		WebViewChapterPagesCache, WebViewMangaCache, force_webview_visible_layout, shorten_for_log,
		sync_webview_debug_state, webview_chapter_storage_key, webview_manga_storage_key,
	},
};

#[cfg(not(test))]
use super::webview_support::{build_webview_instrumentation_config, build_webview_user_script};

pub(crate) fn parse_webview_manga_cache(
	value: &str,
	storage_key: &str,
) -> Result<WebViewMangaDetails> {
	let cache: WebViewMangaCache = serde_json::from_str(value).map_err(|error| {
		AidokuError::Message(format!(
			"Failed to parse manga cache from WebView.\nKey: {storage_key}\nError: {error}"
		))
	})?;

	if cache.manga.title.trim().is_empty() {
		bail!("WebView manga cache does not contain title.\nKey: {storage_key}");
	}

	Ok(cache.manga)
}

pub(crate) fn parse_webview_chapter_pages_cache(
	value: &str,
	storage_key: &str,
) -> Result<Vec<String>> {
	let cache: WebViewChapterPagesCache = serde_json::from_str(value).map_err(|error| {
		AidokuError::Message(format!(
			"Failed to parse chapter pages cache from WebView.\nKey: {storage_key}\nError: {error}"
		))
	})?;

	if cache.pages.is_empty() {
		bail!("WebView chapter pages cache is empty.\nKey: {storage_key}");
	}

	Ok(cache.pages)
}

pub(super) fn fetch_manga_via_webview(slug: &str) -> Result<WebViewMangaDetails> {
	source_log!("[nexustoons] fetch_manga_via_webview start slug={slug}");

	let storage_key = webview_manga_storage_key(slug);
	let webview = WebView::new();
	install_webview_user_script(&webview, &storage_key, "")?;

	let url = crate::manga_url_from_slug(slug);
	load_page(&webview, &url)?;
	force_webview_visible_layout(&webview)?;
	poke_manga_capture(&webview)?;
	sync_webview_debug_state(&webview, "webview manga after load")?;

	let mut details = wait_for_manga_cache(&webview, &url, &storage_key)?;
	details.chapters.sort_by(chapter_sort_key_from_webview);
	details.chapters.reverse();
	source_log!(
		"[nexustoons] fetch_manga_via_webview done title={} chapters={} cover={:?}",
		details.title,
		details.chapters.len(),
		details.cover_url
	);
	Ok(details)
}

pub(super) fn fetch_chapter_pages_via_webview(
	chapter_url: &str,
	slug: &str,
	chapter_id: &str,
) -> Result<Vec<String>> {
	source_log!(
		"[nexustoons] fetch_chapter_pages_via_webview start url={} slug={} chapter_id={}",
		chapter_url,
		slug,
		chapter_id
	);

	let storage_key = webview_chapter_storage_key(slug, chapter_id);
	let webview = WebView::new();
	install_webview_user_script(&webview, "", &storage_key)?;

	load_page(&webview, chapter_url)?;
	force_webview_visible_layout(&webview)?;
	poke_chapter_capture(&webview)?;
	sync_webview_debug_state(&webview, "webview chapter after load")?;

	let pages = wait_for_chapter_pages_cache(&webview, chapter_url, &storage_key)?;
	source_log!(
		"[nexustoons] fetch_chapter_pages_via_webview done pages={} url={}",
		pages.len(),
		chapter_url
	);
	Ok(pages)
}

fn load_page(webview: &WebView, url: &str) -> Result<()> {
	let request = Request::get(url)?
		.header(
			"Accept",
			"text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
		)
		.header("Accept-Language", ACCEPT_LANGUAGE)
		.header("User-Agent", WEBVIEW_USER_AGENT)
		.header("Referer", crate::BASE_URL);

	source_log!("[nexustoons] Loading WebView URL: {url}");
	webview.load_blocking(request).map_err(|error| {
		AidokuError::Message(format!(
			"WebView load failed.\nURL: {url}\nError: {error:?}"
		))
	})?;
	Ok(())
}

fn read_cached_webview_manga(
	webview: &WebView,
	manga_url: &str,
	storage_key: &str,
) -> Result<Option<WebViewMangaDetails>> {
	let script = format!(
		"window[{global_key:?}] || sessionStorage.getItem({storage_key:?}) || ''",
		global_key = WEBVIEW_MANGA_CACHE_GLOBAL_KEY,
		storage_key = storage_key,
	);
	let raw = webview.eval(&script).map_err(|error| {
		AidokuError::Message(format!(
			"WebView manga eval failed.\nURL: {manga_url}\nKey: {storage_key}\nError: {error:?}"
		))
	})?;
	let value = raw.trim();
	if value.is_empty() || value == "null" || value == "undefined" {
		return Ok(None);
	}

	parse_webview_manga_cache(value, storage_key).map(Some)
}

fn read_cached_webview_chapter_pages(
	webview: &WebView,
	chapter_url: &str,
	storage_key: &str,
) -> Result<Option<Vec<String>>> {
	let script = format!(
		"window[{global_key:?}] || sessionStorage.getItem({storage_key:?}) || ''",
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

	parse_webview_chapter_pages_cache(value, storage_key).map(Some)
}

fn wait_for_manga_cache(
	webview: &WebView,
	manga_url: &str,
	storage_key: &str,
) -> Result<WebViewMangaDetails> {
	for attempt in 1..=WEBVIEW_LOAD_ATTEMPTS {
		poke_manga_capture(webview)?;
		if let Some(details) = read_cached_webview_manga(webview, manga_url, storage_key)? {
			source_log!(
				"[nexustoons] webview manga cache ready attempt={} key={} chapters={} title={}",
				attempt,
				storage_key,
				details.chapters.len(),
				details.title
			);
			return Ok(details);
		}
		sync_webview_debug_state(webview, &format!("webview manga wait attempt={attempt}"))?;
		if attempt < WEBVIEW_LOAD_ATTEMPTS {
			sleep(WEBVIEW_LOAD_DELAY_SECONDS);
		}
	}

	bail!("WebView manga cache not populated. URL: {manga_url} key={storage_key}")
}

fn wait_for_chapter_pages_cache(
	webview: &WebView,
	chapter_url: &str,
	storage_key: &str,
) -> Result<Vec<String>> {
	for attempt in 1..=WEBVIEW_LOAD_ATTEMPTS {
		poke_chapter_capture(webview)?;
		if let Some(pages) = read_cached_webview_chapter_pages(webview, chapter_url, storage_key)? {
			source_log!(
				"[nexustoons] webview chapter pages cache ready attempt={} key={} pages={}",
				attempt,
				storage_key,
				pages.len()
			);
			return Ok(pages);
		}
		sync_webview_debug_state(webview, &format!("webview chapter wait attempt={attempt}"))?;
		if attempt < WEBVIEW_LOAD_ATTEMPTS {
			sleep(WEBVIEW_LOAD_DELAY_SECONDS);
		}
	}

	bail!("WebView chapter pages cache not populated. URL: {chapter_url} key={storage_key}")
}

fn poke_manga_capture(webview: &WebView) -> Result<()> {
	let raw = webview.eval(
    "(() => { try { globalThis.__nexustoonsAidokuOpenChapterList && globalThis.__nexustoonsAidokuOpenChapterList(); globalThis.__nexustoonsAidokuCaptureMangaNow && globalThis.__nexustoonsAidokuCaptureMangaNow(); return 'ok'; } catch (error) { return String(error); } })()",
  )?;
	source_log!(
		"[nexustoons] poke_manga_capture result={}",
		shorten_for_log(raw.trim(), 160)
	);
	Ok(())
}

fn poke_chapter_capture(webview: &WebView) -> Result<()> {
	let raw = webview.eval(
    "(() => { try { globalThis.__nexustoonsAidokuCollectChapterPagesNow && globalThis.__nexustoonsAidokuCollectChapterPagesNow(); return 'ok'; } catch (error) { return String(error); } })()",
  )?;
	source_log!(
		"[nexustoons] poke_chapter_capture result={}",
		shorten_for_log(raw.trim(), 160)
	);
	Ok(())
}

#[cfg(not(test))]
fn install_webview_user_script(
	webview: &WebView,
	manga_storage_key: &str,
	chapter_storage_key: &str,
) -> Result<()> {
	let config = build_webview_instrumentation_config(manga_storage_key, chapter_storage_key);
	let user_script = WebViewUserScript {
		source: build_webview_user_script(&config)?,
		at_document_end: false,
		for_main_frame_only: true,
	};

	if let Err(error) = webview.add_user_script(user_script) {
		source_log!(
			"[nexustoons] Failed to add WebView user script: {:?}",
			error
		);
	}
	Ok(())
}

#[cfg(test)]
fn install_webview_user_script(
	_webview: &WebView,
	_manga_storage_key: &str,
	_chapter_storage_key: &str,
) -> Result<()> {
	Ok(())
}
