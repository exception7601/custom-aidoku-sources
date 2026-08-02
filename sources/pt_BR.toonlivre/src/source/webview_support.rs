use aidoku::{
	AidokuError, Result,
	alloc::{String, Vec},
	imports::js::WebView,
	prelude::*,
};

#[cfg(not(test))]
use aidoku::alloc::vec;

use core::cell::RefCell;

use serde::Deserialize;

#[cfg(not(test))]
use serde::Serialize;

use crate::{ApiChapterDetails, debug_logs_enabled, generate_session};

#[cfg(not(test))]
use crate::ACCEPT_LANGUAGE;

#[cfg(not(test))]
use super::WEBVIEW_CHAPTER_CACHE_GLOBAL_KEY;

#[cfg(not(test))]
use super::{
	WEBVIEW_DEVICE_PIXEL_RATIO, WEBVIEW_MAX_TOUCH_POINTS, WEBVIEW_PLATFORM, WEBVIEW_USER_AGENT,
	WEBVIEW_VENDOR, WEBVIEW_VIEWPORT_HEIGHT, WEBVIEW_VIEWPORT_WIDTH,
};

#[cfg(not(test))]
const WEBVIEW_INSTRUMENTATION_SOURCE: &str = include_str!("../webview/instrumentation.js");

#[derive(Debug, Clone, Deserialize)]
pub(super) struct WebViewChapterCache {
	pub(super) chapter: ApiChapterDetails,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct WebViewResourceSnapshot {
	name: String,
	#[serde(rename = "initiatorType")]
	initiator_type: String,
	#[serde(rename = "transferSize")]
	transfer_size: u64,
	#[serde(rename = "decodedBodySize")]
	decoded_body_size: u64,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub(super) struct WebViewDebugSnapshot {
	href: String,
	title: String,
	#[serde(rename = "readyState")]
	ready_state: String,
	pub(super) cookie: String,
	#[serde(rename = "userAgent")]
	user_agent: String,
	language: String,
	languages: Vec<String>,
	platform: String,
	vendor: String,
	#[serde(rename = "maxTouchPoints")]
	max_touch_points: i32,
	#[serde(rename = "referrer")]
	referrer: String,
	#[serde(rename = "visibilityState")]
	visibility_state: String,
	#[serde(rename = "innerWidth")]
	inner_width: i32,
	#[serde(rename = "innerHeight")]
	inner_height: i32,
	#[serde(rename = "outerWidth")]
	outer_width: i32,
	#[serde(rename = "outerHeight")]
	outer_height: i32,
	#[serde(rename = "screenWidth")]
	screen_width: i32,
	#[serde(rename = "screenHeight")]
	screen_height: i32,
	#[serde(rename = "devicePixelRatio")]
	device_pixel_ratio: f64,
	#[serde(rename = "scrollY")]
	scroll_y: f64,
	#[serde(rename = "scrollHeight")]
	scroll_height: i32,
	#[serde(rename = "sessionStorageKeys")]
	session_storage_keys: Vec<String>,
	#[serde(rename = "sessionStorageLength")]
	session_storage_length: usize,
	#[serde(default, rename = "workerUrls")]
	worker_urls: Vec<String>,
	#[serde(default, rename = "runtimeProofEvents")]
	runtime_proof_events: Vec<String>,
	#[serde(default, rename = "fingerprintProfile")]
	fingerprint_profile: String,
	resources: Vec<WebViewResourceSnapshot>,
	#[serde(rename = "bodyTextPreview")]
	body_text_preview: String,
	#[serde(rename = "bodyHtmlPreview")]
	body_html_preview: String,
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
	#[serde(rename = "cookieName")]
	cookie_name: String,
	#[serde(rename = "toonVCookie")]
	toon_v_cookie: String,
	#[serde(rename = "chapterCacheGlobalKey")]
	chapter_cache_global_key: String,
	#[serde(rename = "chapterStorageKey")]
	chapter_storage_key: String,
	#[serde(rename = "runtimeUrlHints")]
	runtime_url_hints: Vec<String>,
	#[serde(rename = "payloadUrlHints")]
	payload_url_hints: Vec<String>,
	#[serde(rename = "siteHostHints")]
	site_host_hints: Vec<String>,
	#[serde(rename = "targetMangaId")]
	target_manga_id: String,
	#[serde(rename = "targetChapterId")]
	target_chapter_id: String,
	#[serde(rename = "targetChapterNumber")]
	target_chapter_number: String,
	debug: bool,
}

struct WebViewCookieCache(RefCell<Option<String>>);

unsafe impl Sync for WebViewCookieCache {}

static WEBVIEW_COOKIE_CACHE: WebViewCookieCache = WebViewCookieCache(RefCell::new(None));

pub(super) fn shorten_for_log(value: &str, max_chars: usize) -> String {
	let mut output: String = value.chars().take(max_chars).collect();
	if value.chars().nth(max_chars).is_some() {
		output.push_str("...");
	}
	output
}

pub(super) fn fetch_webview_debug_snapshot(webview: &WebView) -> Result<WebViewDebugSnapshot> {
	let raw = webview.eval(
		"JSON.stringify({href: location.href, title: document.title, readyState: document.readyState, cookie: document.cookie || '', userAgent: navigator.userAgent, language: navigator.language || '', languages: navigator.languages || [], platform: navigator.platform || '', vendor: navigator.vendor || '', maxTouchPoints: navigator.maxTouchPoints || 0, referrer: document.referrer || '', visibilityState: document.visibilityState, innerWidth: window.innerWidth || 0, innerHeight: window.innerHeight || 0, outerWidth: window.outerWidth || 0, outerHeight: window.outerHeight || 0, screenWidth: window.screen ? window.screen.width || 0 : 0, screenHeight: window.screen ? window.screen.height || 0 : 0, devicePixelRatio: window.devicePixelRatio || 0, scrollY: window.scrollY, scrollHeight: document.documentElement.scrollHeight || document.body.scrollHeight || 0, sessionStorageKeys: Object.keys(sessionStorage), sessionStorageLength: sessionStorage.length, workerUrls: window.__toonlivreAidokuDebug && Array.isArray(window.__toonlivreAidokuDebug.workerUrls) ? window.__toonlivreAidokuDebug.workerUrls.slice(-12) : [], runtimeProofEvents: window.__toonlivreAidokuDebug && Array.isArray(window.__toonlivreAidokuDebug.events) ? window.__toonlivreAidokuDebug.events.slice(-30) : [], fingerprintProfile: window.__toonlivreAidokuDebug && typeof window.__toonlivreAidokuDebug.fingerprintProfile === 'string' ? window.__toonlivreAidokuDebug.fingerprintProfile : '', resources: performance.getEntriesByType('resource').slice(-30).map(resource => ({name: resource.name, initiatorType: resource.initiatorType, transferSize: resource.transferSize, decodedBodySize: resource.decodedBodySize})), bodyTextPreview: document.body ? document.body.innerText.replace(/\\s+/g, ' ').trim().slice(0, 500) : '', bodyHtmlPreview: document.body ? document.body.innerHTML.replace(/\\s+/g, ' ').trim().slice(0, 500) : ''})",
	)?;
	serde_json::from_str(&raw).map_err(|error| {
		AidokuError::Message(format!("Failed to parse WebView snapshot.\nError: {error}"))
	})
}

pub(super) fn update_webview_cookie_cache(cookie_header: &str) {
	let candidate = cookie_header.trim();
	if candidate.is_empty() {
		return;
	}

	let candidate_pairs = candidate
		.split(';')
		.filter(|part| !part.trim().is_empty())
		.count();
	let mut cache = WEBVIEW_COOKIE_CACHE.0.borrow_mut();
	let should_update = match cache.as_ref() {
		Some(existing) => {
			let existing_pairs = existing
				.split(';')
				.filter(|part| !part.trim().is_empty())
				.count();
			candidate_pairs > existing_pairs
				|| (candidate_pairs == existing_pairs && candidate.len() >= existing.len())
		}
		None => true,
	};
	if should_update {
		*cache = Some(String::from(candidate));
	}
}

pub(super) fn build_webview_cookie_header() -> String {
	if let Some(cookie) = webview_cookie_cache()
		&& cookie.contains("toon_v=")
	{
		return cookie;
	}

	let session = generate_session();
	match webview_cookie_cache() {
		Some(cookie) if !cookie.trim().is_empty() => {
			if cookie.contains("toon_v=") {
				cookie
			} else {
				format!("{cookie}; toon_v={session}")
			}
		}
		_ => format!("toon_v={session}"),
	}
}

#[cfg(not(test))]
pub(super) fn cookie_value_from_header(cookie_header: &str, name: &str) -> Option<String> {
	cookie_header.split(';').find_map(|part| {
		let trimmed = part.trim();
		let (key, value) = trimmed.split_once('=')?;
		if key.trim() == name {
			Some(String::from(value.trim()))
		} else {
			None
		}
	})
}

#[cfg(not(test))]
pub(super) fn build_webview_instrumentation_config(
	storage_key: &str,
	manga_id: &str,
	chapter_id: &str,
	chapter_number_hint: &str,
	toon_v_cookie: &str,
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
		cookie_name: String::from("toon_v"),
		toon_v_cookie: String::from(toon_v_cookie),
		chapter_cache_global_key: String::from(WEBVIEW_CHAPTER_CACHE_GLOBAL_KEY),
		chapter_storage_key: String::from(storage_key),
		runtime_url_hints: vec![String::from("/api/reader/")],
		payload_url_hints: vec![String::from("/api/")],
		site_host_hints: vec![String::from("toonlivre.net")],
		target_manga_id: String::from(manga_id),
		target_chapter_id: String::from(chapter_id),
		target_chapter_number: String::from(chapter_number_hint),
		debug: debug_logs_enabled(),
	}
}

#[cfg(not(test))]
pub(super) fn build_webview_user_script(config: &WebViewInstrumentationConfig) -> Result<String> {
	let config_json = serialize_webview_instrumentation_config(config)?;
	Ok(format!(
		"{script}\n;globalThis.__toonlivreAidokuBoot({config_json});",
		script = WEBVIEW_INSTRUMENTATION_SOURCE,
		config_json = config_json,
	))
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
		let config = build_webview_instrumentation_config("", "", "", "", "");
		let config_json = serialize_webview_instrumentation_config(&config)?;
		if debug_enabled {
			let raw = webview.eval(&format!(
				r#"(() => {{
					try {{
						if (typeof globalThis.__toonlivreAidokuApplyLayoutPatch !== 'function') {{
							{script}
						}}
						const result =
							typeof globalThis.__toonlivreAidokuApplyLayoutPatch === 'function'
								? globalThis.__toonlivreAidokuApplyLayoutPatch({config_json})
								: {{ error: 'missing __toonlivreAidokuApplyLayoutPatch' }};
						return JSON.stringify(result);
					}} catch (error) {{
						return JSON.stringify({{ error: String(error) }});
					}}
				}})()"#,
				script = WEBVIEW_INSTRUMENTATION_SOURCE,
				config_json = config_json,
			))?;
			source_log!("[toonlivre] webview layout patch result={raw}");
			return Ok(());
		}

		webview.eval(&format!(
			r#"(() => {{
				try {{
					if (typeof globalThis.__toonlivreAidokuApplyLayoutPatch !== 'function') {{
						{script}
					}}
					if (typeof globalThis.__toonlivreAidokuApplyLayoutPatch === 'function') {{
						globalThis.__toonlivreAidokuApplyLayoutPatch({config_json});
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

#[allow(dead_code)]
pub(super) fn sync_webview_debug_state(webview: &WebView, label: &str) -> Result<()> {
	if !debug_logs_enabled() {
		let cookie = webview.eval("document.cookie || ''").map_err(|error| {
			AidokuError::Message(format!("Failed to read WebView cookie.\nError: {error:?}"))
		})?;
		update_webview_cookie_cache(cookie.trim());
		return Ok(());
	}

	let snapshot = fetch_webview_debug_snapshot(webview)?;
	update_webview_cookie_cache(&snapshot.cookie);
	log_webview_debug_snapshot(label, &snapshot);
	Ok(())
}

#[allow(dead_code)]
pub(super) fn log_webview_debug_snapshot(_label: &str, _snapshot: &WebViewDebugSnapshot) {
	let runtime_proof_events = if _snapshot.runtime_proof_events.is_empty() {
		String::from("(none)")
	} else {
		_snapshot
			.runtime_proof_events
			.iter()
			.map(|value| shorten_for_log(value, 220))
			.collect::<Vec<_>>()
			.join(" | ")
	};
	let worker_urls = if _snapshot.worker_urls.is_empty() {
		String::from("(none)")
	} else {
		_snapshot
			.worker_urls
			.iter()
			.map(|value| shorten_for_log(value, 180))
			.collect::<Vec<_>>()
			.join(" | ")
	};

	source_log!(
		"[toonlivre] {} href={} title={} ready_state={} visibility_state={} size={}x{} outer={}x{} screen={}x{} dpr={} scroll_y={} scroll_height={} cookie={} user_agent={} language={} languages={} platform={} vendor={} max_touch_points={} referrer={} fingerprint_profile={} session_storage_keys={} session_storage_length={}",
		_label,
		_snapshot.href,
		_snapshot.title,
		_snapshot.ready_state,
		_snapshot.visibility_state,
		_snapshot.inner_width,
		_snapshot.inner_height,
		_snapshot.outer_width,
		_snapshot.outer_height,
		_snapshot.screen_width,
		_snapshot.screen_height,
		_snapshot.device_pixel_ratio,
		_snapshot.scroll_y,
		_snapshot.scroll_height,
		shorten_for_log(&_snapshot.cookie, 240),
		shorten_for_log(&_snapshot.user_agent, 180),
		shorten_for_log(&_snapshot.language, 40),
		_snapshot.languages.join("|"),
		shorten_for_log(&_snapshot.platform, 40),
		shorten_for_log(&_snapshot.vendor, 40),
		_snapshot.max_touch_points,
		shorten_for_log(&_snapshot.referrer, 80),
		shorten_for_log(&_snapshot.fingerprint_profile, 80),
		_snapshot.session_storage_keys.join("|"),
		_snapshot.session_storage_length,
	);
	source_log!(
		"[toonlivre] {} runtime_proof_events={}",
		_label,
		runtime_proof_events
	);
	source_log!("[toonlivre] {} worker_urls={}", _label, worker_urls);
	source_log!(
		"[toonlivre] {} resources={}",
		_label,
		format_webview_resources(_snapshot)
	);
	source_log!(
		"[toonlivre] {} body_text_preview={}",
		_label,
		shorten_for_log(&_snapshot.body_text_preview, 500),
	);
	source_log!(
		"[toonlivre] {} body_html_preview={}",
		_label,
		shorten_for_log(&_snapshot.body_html_preview, 500),
	);
}

fn webview_cookie_cache() -> Option<String> {
	WEBVIEW_COOKIE_CACHE.0.borrow().clone()
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

#[allow(dead_code)]
fn format_webview_resources(snapshot: &WebViewDebugSnapshot) -> String {
	if snapshot.resources.is_empty() {
		return String::from("(none)");
	}

	snapshot
		.resources
		.iter()
		.map(|resource| {
			format!(
				"{}:{}:t{}:d{}",
				resource.initiator_type,
				shorten_for_log(&resource.name, 120),
				resource.transfer_size,
				resource.decoded_body_size,
			)
		})
		.collect::<Vec<_>>()
		.join(" | ")
}
