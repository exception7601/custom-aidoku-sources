use aidoku::{
	AidokuError, Result,
	alloc::{String, Vec, format},
	imports::js::WebView,
};

#[cfg(not(test))]
use aidoku::alloc::vec;

use serde::Deserialize;

#[cfg(not(test))]
use serde::Serialize;

use crate::debug_logs_enabled;

use super::manga::WebViewMangaDetails;

#[cfg(not(test))]
use crate::ACCEPT_LANGUAGE;

#[cfg(not(test))]
use super::{
	WEBVIEW_CHAPTER_CACHE_GLOBAL_KEY, WEBVIEW_DEVICE_PIXEL_RATIO, WEBVIEW_MANGA_CACHE_GLOBAL_KEY,
	WEBVIEW_MAX_TOUCH_POINTS, WEBVIEW_PLATFORM, WEBVIEW_USER_AGENT, WEBVIEW_VENDOR,
	WEBVIEW_VIEWPORT_HEIGHT, WEBVIEW_VIEWPORT_WIDTH,
};

#[cfg(not(test))]
const WEBVIEW_INSTRUMENTATION_SOURCE: &str = include_str!("../webview/instrumentation.js");

#[derive(Debug, Clone, Deserialize)]
pub(super) struct WebViewMangaCache {
	pub(super) manga: WebViewMangaDetails,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct WebViewChapterPagesCache {
	pub(super) pages: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub(super) struct WebViewDebugSnapshot {
	href: String,
	title: String,
	#[serde(rename = "readyState")]
	ready_state: String,
	#[serde(rename = "innerWidth")]
	inner_width: i32,
	#[serde(rename = "innerHeight")]
	inner_height: i32,
	#[serde(rename = "scrollHeight")]
	scroll_height: i32,
	#[serde(rename = "scrollY")]
	scroll_y: f64,
	#[serde(rename = "mangaStorageLength")]
	manga_storage_length: usize,
	#[serde(rename = "chapterStorageLength")]
	chapter_storage_length: usize,
	#[serde(default, rename = "events")]
	events: Vec<String>,
	#[serde(rename = "bodyTextPreview")]
	body_text_preview: String,
}

#[cfg(not(test))]
#[derive(Debug, Clone, Serialize)]
pub(super) struct WebViewInstrumentationConfig {
	width: i32,
	height: i32,
	dpr: f64,
	#[serde(rename = "userAgent")]
	user_agent: String,
	platform: String,
	vendor: String,
	#[serde(rename = "maxTouchPoints")]
	max_touch_points: i32,
	#[serde(rename = "languageHeader")]
	language_header: String,
	#[serde(rename = "siteHostHints")]
	site_host_hints: Vec<String>,
	#[serde(rename = "mangaPageUrlHints")]
	manga_page_url_hints: Vec<String>,
	#[serde(rename = "chapterPageUrlHints")]
	chapter_page_url_hints: Vec<String>,
	#[serde(rename = "mangaPayloadUrlHints")]
	manga_payload_url_hints: Vec<String>,
	#[serde(rename = "chapterButtonTextHints")]
	chapter_button_text_hints: Vec<String>,
	#[serde(rename = "chapterLinkUrlHints")]
	chapter_link_url_hints: Vec<String>,
	#[serde(rename = "reactRootSelectors")]
	react_root_selectors: Vec<String>,
	#[serde(rename = "chapterImageUrlHints")]
	chapter_image_url_hints: Vec<String>,
	#[serde(rename = "mangaCacheGlobalKey")]
	manga_cache_global_key: String,
	#[serde(rename = "mangaStorageKey")]
	manga_storage_key: String,
	#[serde(rename = "chapterCacheGlobalKey")]
	chapter_cache_global_key: String,
	#[serde(rename = "chapterStorageKey")]
	chapter_storage_key: String,
	debug: bool,
}

pub(super) fn shorten_for_log(value: &str, max_chars: usize) -> String {
	let mut output: String = value.chars().take(max_chars).collect();
	if value.chars().nth(max_chars).is_some() {
		output.push_str("...");
	}
	output
}

pub(crate) fn webview_manga_storage_key(slug: &str) -> String {
	format!(
		"nexustoons_manga_cache_v1:{}",
		slug.trim().trim_matches('/')
	)
}

pub(crate) fn webview_chapter_storage_key(slug: &str, chapter_id: &str) -> String {
	format!(
		"nexustoons_chapter_pages_cache_v1:{}:{}",
		slug.trim().trim_matches('/'),
		chapter_id.trim().trim_matches('/')
	)
}

#[cfg(not(test))]
pub(super) fn build_webview_instrumentation_config(
	manga_storage_key: &str,
	chapter_storage_key: &str,
) -> WebViewInstrumentationConfig {
	WebViewInstrumentationConfig {
		width: WEBVIEW_VIEWPORT_WIDTH,
		height: WEBVIEW_VIEWPORT_HEIGHT,
		dpr: WEBVIEW_DEVICE_PIXEL_RATIO,
		user_agent: String::from(WEBVIEW_USER_AGENT),
		platform: String::from(WEBVIEW_PLATFORM),
		vendor: String::from(WEBVIEW_VENDOR),
		max_touch_points: WEBVIEW_MAX_TOUCH_POINTS,
		language_header: String::from(ACCEPT_LANGUAGE),
		site_host_hints: vec![
			String::from("nexustoons.com"),
			String::from("img.nx-toons.xyz"),
		],
		manga_page_url_hints: vec![String::from("/manga/")],
		chapter_page_url_hints: vec![String::from("/ler/")],
		manga_payload_url_hints: vec![String::from("/api/manga/")],
		chapter_button_text_hints: vec![String::from("Capítulos"), String::from("Capitulos")],
		chapter_link_url_hints: vec![String::from("/ler/"), String::from("/r/")],
		react_root_selectors: vec![String::from("div.custom-scrollbar")],
		chapter_image_url_hints: vec![String::from("manga_pages")],
		manga_cache_global_key: String::from(WEBVIEW_MANGA_CACHE_GLOBAL_KEY),
		manga_storage_key: String::from(manga_storage_key),
		chapter_cache_global_key: String::from(WEBVIEW_CHAPTER_CACHE_GLOBAL_KEY),
		chapter_storage_key: String::from(chapter_storage_key),
		debug: debug_logs_enabled(),
	}
}

#[cfg(not(test))]
pub(super) fn build_webview_user_script(config: &WebViewInstrumentationConfig) -> Result<String> {
	let config_json = serialize_webview_instrumentation_config(config)?;
	Ok(format!(
		"{}\n;globalThis.__nexustoonsAidokuBoot && globalThis.__nexustoonsAidokuBoot({});",
		WEBVIEW_INSTRUMENTATION_SOURCE, config_json
	))
}

#[cfg(not(test))]
fn serialize_webview_instrumentation_config(
	config: &WebViewInstrumentationConfig,
) -> Result<String> {
	serde_json::to_string(config).map_err(|error| {
		AidokuError::Message(format!(
			"Failed to serialize WebView instrumentation config.\nError: {error}"
		))
	})
}

pub(super) fn fetch_webview_debug_snapshot(webview: &WebView) -> Result<WebViewDebugSnapshot> {
	let raw = webview.eval(
    "JSON.stringify({href: location.href, title: document.title || '', readyState: document.readyState || '', innerWidth: window.innerWidth || 0, innerHeight: window.innerHeight || 0, scrollHeight: document.documentElement.scrollHeight || document.body.scrollHeight || 0, scrollY: window.scrollY || 0, mangaStorageLength: (() => { const key = globalThis.__nexustoonsAidokuConfig && globalThis.__nexustoonsAidokuConfig.mangaStorageKey; const raw = globalThis.__nexustoonsAidokuTestAPI && globalThis.__nexustoonsAidokuTestAPI.readMangaCache ? globalThis.__nexustoonsAidokuTestAPI.readMangaCache() : (key ? sessionStorage.getItem(key) : '') || ''; return raw.length; })(), chapterStorageLength: (() => { const key = globalThis.__nexustoonsAidokuConfig && globalThis.__nexustoonsAidokuConfig.chapterStorageKey; const raw = globalThis.__nexustoonsAidokuTestAPI && globalThis.__nexustoonsAidokuTestAPI.readChapterPagesCache ? globalThis.__nexustoonsAidokuTestAPI.readChapterPagesCache() : (key ? sessionStorage.getItem(key) : '') || ''; return raw.length; })(), events: globalThis.__nexustoonsAidokuDebug && Array.isArray(globalThis.__nexustoonsAidokuDebug.events) ? globalThis.__nexustoonsAidokuDebug.events.slice(-30) : [], bodyTextPreview: document.body ? document.body.innerText.replace(/\\s+/g, ' ').trim().slice(0, 500) : ''})",
  )?;
	serde_json::from_str(&raw).map_err(|error| {
		AidokuError::Message(format!("Failed to parse WebView snapshot.\nError: {error}"))
	})
}

pub(super) fn force_webview_visible_layout(webview: &WebView) -> Result<()> {
	#[cfg(test)]
	{
		let _ = webview;
		Ok(())
	}

	#[cfg(not(test))]
	{
		let debug_enabled = debug_logs_enabled();
		let config = build_webview_instrumentation_config("", "");
		let config_json = serialize_webview_instrumentation_config(&config)?;

		if debug_enabled {
			let raw = webview.eval(&format!(
				r#"(() => {{
          try {{
            if (typeof globalThis.__nexustoonsAidokuApplyLayoutPatch !== 'function') {{
              {script}
            }}
            const result =
              typeof globalThis.__nexustoonsAidokuApplyLayoutPatch === 'function'
                ? globalThis.__nexustoonsAidokuApplyLayoutPatch({config_json})
                : {{ error: 'missing __nexustoonsAidokuApplyLayoutPatch' }};
            return JSON.stringify(result);
          }} catch (error) {{
            return JSON.stringify({{ error: String(error) }});
          }}
        }})()"#,
				script = WEBVIEW_INSTRUMENTATION_SOURCE,
				config_json = config_json,
			))?;
			source_log!("[nexustoons] webview layout patch result={raw}");
			return Ok(());
		}

		webview.eval(&format!(
			r#"(() => {{
        try {{
          if (typeof globalThis.__nexustoonsAidokuApplyLayoutPatch !== 'function') {{
            {script}
          }}
          if (typeof globalThis.__nexustoonsAidokuApplyLayoutPatch === 'function') {{
            globalThis.__nexustoonsAidokuApplyLayoutPatch({config_json});
          }}
          return 'ok';
        }} catch (error) {{
          return String(error);
        }}
      }})()"#,
			script = WEBVIEW_INSTRUMENTATION_SOURCE,
			config_json = config_json,
		))?;
		Ok(())
	}
}

pub(super) fn sync_webview_debug_state(webview: &WebView, label: &str) -> Result<()> {
	if !debug_logs_enabled() {
		return Ok(());
	}

	let snapshot = fetch_webview_debug_snapshot(webview)?;
	let events = if snapshot.events.is_empty() {
		String::from("none")
	} else {
		snapshot.events.join(" | ")
	};
	source_log!(
		"[nexustoons] {} href={} title={} ready_state={} size={}x{} scroll={}/{} manga_cache_len={} chapter_cache_len={} events={} body={} ",
		label,
		snapshot.href,
		snapshot.title,
		snapshot.ready_state,
		snapshot.inner_width,
		snapshot.inner_height,
		snapshot.scroll_y,
		snapshot.scroll_height,
		snapshot.manga_storage_length,
		snapshot.chapter_storage_length,
		shorten_for_log(&events, 300),
		shorten_for_log(&snapshot.body_text_preview, 220)
	);
	Ok(())
}
