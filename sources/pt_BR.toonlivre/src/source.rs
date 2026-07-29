use aidoku::{
	AidokuError, Chapter, DeepLinkHandler, DeepLinkResult, FilterValue, Home, HomeComponent,
	HomeComponentValue, HomeLayout, HomePartialResult, ImageRequestProvider, Link, LinkValue,
	Manga, MangaPageResult, MangaWithChapter, Page, PageContent, PageContext, Result, Source,
	Viewer,
	alloc::{String, Vec, vec},
	imports::{
		js::WebView,
		net::Request,
		std::{send_partial_result, sleep},
	},
	prelude::*,
};

#[cfg(not(test))]
use aidoku::imports::js::WebViewUserScript;

use core::cell::RefCell;

use serde::Deserialize;

use crate::{
	ACCEPT_LANGUAGE, ApiChapter, ApiChapterDetails, ApiListResponse, ApiMangaById, ApiMangaBySlug,
	ApiMangaCard, ApiReaderManga, chapter_key_or_number, chapter_numbers_match,
	chapter_url_from_slug_and_number, date_from_timestamp_millis, deep_link_result,
	fetch_manga_by_id, fetch_manga_by_slug, fetch_manga_reader, fetch_releases, generate_session,
	manga_slug_from_manga, manga_status_from_text, manga_url_from_slug, normalize_chapter_number,
	parse_chapter_number, slugify_title,
};

pub(crate) struct ToonLivre;

const RELEASES_PAGE_SIZE: i32 = 48;
const SEARCH_PAGE_SIZE: i32 = 24;
const HOME_PAGE_SIZE: usize = 12;
const WEBVIEW_CHAPTER_LOAD_ATTEMPTS: i32 = 10;
const WEBVIEW_CHAPTER_LOAD_DELAY_SECONDS: i32 = 1;
#[allow(dead_code)]
const WEBVIEW_USER_AGENT: &str = concat!(
	"Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) ",
	"AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 ",
	"Mobile/15E148 Safari/604.1",
);

impl Source for ToonLivre {
	fn new() -> Self {
		Self
	}

	fn get_search_manga_list(
		&self,
		query: Option<String>,
		page: i32,
		_filters: Vec<FilterValue>,
	) -> Result<MangaPageResult> {
		let _raw_query = query.clone();
		let _raw_page = page;
		let page = page.max(1);
		source_log!(
			"[toonlivre] get_search_manga_list start raw_page={} normalized_page={} query={:?}",
			_raw_page,
			page,
			_raw_query.as_deref()
		);
		let response = match query.map(|value| String::from(value.trim())) {
			Some(query) if !query.is_empty() => search_response(&query, page)?,
			_ => fetch_releases(page, RELEASES_PAGE_SIZE)?,
		};
		source_log!(
			"[toonlivre] get_search_manga_list response mangas={} current_page={} has_next_page={}",
			response.mangas.len(),
			response.pagination.current_page,
			response.pagination.has_next_page
		);
		Ok(map_list_response(&response))
	}

	fn get_manga_update(
		&self,
		mut manga: Manga,
		needs_details: bool,
		needs_chapters: bool,
	) -> Result<Manga> {
		source_log!(
			"[toonlivre] get_manga_update start key={} title={} needs_details={} needs_chapters={}",
			manga.key,
			manga.title,
			needs_details,
			needs_chapters
		);
		if manga.key.starts_with("obra-") {
			let details = fetch_manga_by_id(&manga.key)?;
			source_log!(
				"[toonlivre] get_manga_update by_id id={} slug={} recent_chapters={}",
				details.id,
				details.slug,
				details.recent_chapters.len()
			);
			if needs_details {
				apply_details_from_id(&mut manga, &details);
			}
			if needs_chapters {
				let reader = fetch_manga_reader(&details.id)?;
				source_log!(
					"[toonlivre] get_manga_update reader id={} chapters={}",
					reader.id,
					reader.chapters.len()
				);
				apply_details_from_reader(&mut manga, &reader, needs_details, true);
			}
			source_log!(
				"[toonlivre] get_manga_update done key={} chapters={} url={:?}",
				manga.key,
				manga
					.chapters
					.as_ref()
					.map(|chapters| chapters.len())
					.unwrap_or_default(),
				manga.url.as_deref()
			);
			return Ok(manga);
		}

		let slug = manga_slug_from_manga(&manga)
			.ok_or_else(|| AidokuError::Message(String::from("Unable to resolve manga slug")))?;
		source_log!("[toonlivre] get_manga_update resolved_slug={slug}");
		let details = fetch_manga_by_slug(&slug)?;
		source_log!(
			"[toonlivre] get_manga_update by_slug id={} slug={:?} chapters={}",
			details.id,
			details.slug.as_deref(),
			details.chapters.len()
		);
		apply_details_from_slug(&mut manga, &details, needs_details, needs_chapters);
		source_log!(
			"[toonlivre] get_manga_update done key={} chapters={} url={:?}",
			manga.key,
			manga
				.chapters
				.as_ref()
				.map(|chapters| chapters.len())
				.unwrap_or_default(),
			manga.url.as_deref()
		);
		Ok(manga)
	}

	fn get_page_list(&self, manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		source_log!(
			"[toonlivre] get_page_list start manga_key={} manga_title={} chapter_key={} chapter_url={:?} chapter_number={:?}",
			manga.key,
			manga.title,
			chapter.key,
			chapter.url.as_deref(),
			chapter.chapter_number
		);
		let (manga_id, chapter_id, chapter_url) = resolve_chapter_identity(&manga, &chapter)?;
		source_log!(
			"[toonlivre] get_page_list resolved manga_id={} chapter_id={} chapter_url={}",
			manga_id,
			chapter_id,
			chapter_url
		);
		let chapter_details = fetch_chapter_via_webview(&chapter_url, &manga_id, &chapter_id)?;
		source_log!(
			"[toonlivre] get_page_list webview details id={} number={} timestamp={} pages={}",
			chapter_details.id,
			chapter_details.number,
			chapter_details.timestamp,
			chapter_details.pages.len()
		);
		if chapter_details.pages.is_empty() {
			bail!("No chapter pages found");
		}

		Ok(chapter_details
			.pages
			.into_iter()
			.map(|url| {
				let mut context = PageContext::new();
				context.insert(String::from("referer"), chapter_url.clone());
				Page {
					content: PageContent::url_context(url, context),
					..Default::default()
				}
			})
			.collect())
	}
}

impl Home for ToonLivre {
	fn get_home(&self) -> Result<HomeLayout> {
		source_log!("[toonlivre] get_home start releases_page_size={RELEASES_PAGE_SIZE}");
		let response = fetch_releases(1, RELEASES_PAGE_SIZE)?;
		source_log!(
			"[toonlivre] get_home response mangas={} current_page={} has_next_page={}",
			response.mangas.len(),
			response.pagination.current_page,
			response.pagination.has_next_page
		);
		let entries = response
			.mangas
			.iter()
			.take(HOME_PAGE_SIZE)
			.map(manga_from_card)
			.collect::<Vec<_>>();
		let recent_chapters = response
			.mangas
			.iter()
			.take(HOME_PAGE_SIZE)
			.filter_map(manga_with_recent_chapter)
			.collect::<Vec<_>>();

		send_partial_result(&HomePartialResult::Layout(HomeLayout {
			components: Vec::new(),
		}));

		Ok(HomeLayout {
			components: vec![
				HomeComponent {
					title: Some(String::from("Lançamentos")),
					subtitle: None,
					value: HomeComponentValue::BigScroller {
						entries: entries.clone(),
						auto_scroll_interval: None,
					},
				},
				HomeComponent {
					title: Some(String::from("Capítulos recentes")),
					subtitle: None,
					value: HomeComponentValue::MangaChapterList {
						page_size: Some(HOME_PAGE_SIZE as i32),
						entries: recent_chapters,
						listing: None,
					},
				},
				HomeComponent {
					title: Some(String::from("Mais obras")),
					subtitle: None,
					value: HomeComponentValue::MangaList {
						ranking: false,
						page_size: Some(HOME_PAGE_SIZE as i32),
						entries: entries.into_iter().map(manga_to_link).collect(),
						listing: None,
					},
				},
			],
		})
	}
}

impl DeepLinkHandler for ToonLivre {
	fn handle_deep_link(&self, url: String) -> Result<Option<DeepLinkResult>> {
		Ok(deep_link_result(&url))
	}
}

impl ImageRequestProvider for ToonLivre {
	fn get_image_request(&self, url: String, context: Option<PageContext>) -> Result<Request> {
		let mut request = Request::get(&url)?
			.header(
				"User-Agent",
				"Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1",
			)
			.header("Accept", "image/avif,image/webp,image/*,*/*;q=0.8")
			.header("accept-language", ACCEPT_LANGUAGE);
		let referer = context
			.as_ref()
			.and_then(|ctx| ctx.get("referer"))
			.map(String::as_str)
			.unwrap_or(crate::BASE_URL);
		request.set_header("Referer", referer);
		Ok(request)
	}
}

fn search_response(query: &str, page: i32) -> Result<ApiListResponse> {
	crate::search_mangas(query, page, SEARCH_PAGE_SIZE)
}

fn map_list_response(response: &ApiListResponse) -> MangaPageResult {
	MangaPageResult {
		entries: response.mangas.iter().map(manga_from_card).collect(),
		has_next_page: response.pagination.has_next_page,
	}
}

fn manga_from_card(card: &ApiMangaCard) -> Manga {
	Manga {
		key: card.id.clone(),
		title: card.title.clone(),
		cover: card.cover_url.clone(),
		description: card.alternative_title.clone(),
		url: None,
		viewer: Viewer::Vertical,
		..Default::default()
	}
}

fn manga_with_recent_chapter(card: &ApiMangaCard) -> Option<MangaWithChapter> {
	let manga = manga_from_card(card);
	let chapter = chapter_from_api(
		card.recent_chapters.first()?,
		card.slug.as_deref(),
		&card.title,
	);
	Some(MangaWithChapter { manga, chapter })
}

fn manga_to_link(manga: Manga) -> Link {
	Link {
		title: manga.title.clone(),
		subtitle: manga.description.clone(),
		image_url: manga.cover.clone(),
		value: Some(LinkValue::Manga(manga)),
	}
}

fn apply_details_from_id(manga: &mut Manga, details: &ApiMangaById) {
	manga.key = details.id.clone();
	manga.title = details.title.clone();
	manga.cover = details.cover_url.clone();
	manga.authors = Some(details.authors.clone());
	manga.artists = Some(details.artists.clone());
	manga.tags = Some(details.genres.clone());
	manga.description = merge_description(
		details.alternative_title.as_deref(),
		details.description.as_deref(),
	);
	manga.status = details
		.status
		.as_deref()
		.map(manga_status_from_text)
		.unwrap_or_default();
	manga.url = Some(manga_url_from_slug(&details.slug));
	manga.viewer = Viewer::Vertical;
	manga.content_rating = aidoku::ContentRating::Safe;
}

fn apply_details_from_reader(
	manga: &mut Manga,
	details: &ApiReaderManga,
	needs_details: bool,
	needs_chapters: bool,
) {
	if needs_details {
		manga.key = details.id.clone();
		manga.title = details.title.clone();
		manga.cover = details.cover_url.clone();
		manga.authors = Some(details.authors.clone());
		manga.artists = Some(details.artists.clone());
		manga.tags = Some(details.genres.clone());
		manga.description = merge_description(
			details.alternative_title.as_deref(),
			details.description.as_deref(),
		);
		manga.status = details
			.status
			.as_deref()
			.map(manga_status_from_text)
			.unwrap_or_default();
		if let Some(slug) = details.slug.as_deref() {
			manga.url = Some(manga_url_from_slug(slug));
		}
		manga.viewer = Viewer::Vertical;
		manga.content_rating = aidoku::ContentRating::Safe;
	}
	if needs_chapters {
		let slug = details
			.slug
			.clone()
			.unwrap_or_else(|| slugify_title(&details.title));
		manga.chapters = Some(
			details
				.chapters
				.iter()
				.map(|chapter| chapter_from_api(chapter, Some(&slug), &details.title))
				.collect(),
		);
	}
}

fn apply_details_from_slug(
	manga: &mut Manga,
	details: &ApiMangaBySlug,
	needs_details: bool,
	needs_chapters: bool,
) {
	let slug = details
		.slug
		.clone()
		.unwrap_or_else(|| slugify_title(&details.title));
	if needs_details {
		manga.key = details.id.clone();
		manga.title = details.title.clone();
		manga.cover = details.cover_url.clone();
		manga.authors = Some(details.authors.clone());
		manga.artists = Some(details.artists.clone());
		manga.tags = Some(details.genres.clone());
		manga.description = merge_description(
			details.alternative_title.as_deref(),
			details.description.as_deref(),
		);
		manga.status = details
			.status
			.as_deref()
			.map(manga_status_from_text)
			.unwrap_or_default();
		manga.url = Some(manga_url_from_slug(&slug));
		manga.viewer = Viewer::Vertical;
		manga.content_rating = aidoku::ContentRating::Safe;
	}
	if needs_chapters {
		manga.chapters = Some(
			details
				.chapters
				.iter()
				.map(|chapter| chapter_from_api(chapter, Some(&slug), &details.title))
				.collect(),
		);
	}
}

fn merge_description(alternative_title: Option<&str>, description: Option<&str>) -> Option<String> {
	let alternative_title = alternative_title
		.map(str::trim)
		.filter(|value| !value.is_empty());
	let description = description.map(str::trim).filter(|value| !value.is_empty());
	match (alternative_title, description) {
		(Some(alternative_title), Some(description)) => {
			Some(format!("{alternative_title}\n\n{description}"))
		}
		(Some(alternative_title), None) => Some(String::from(alternative_title)),
		(None, Some(description)) => Some(String::from(description)),
		(None, None) => None,
	}
}

fn chapter_from_api(chapter: &ApiChapter, slug: Option<&str>, manga_title: &str) -> Chapter {
	let chapter_number = normalize_chapter_number(&chapter.number);
	source_log!(
		"[toonlivre] chapter_from_api id={} raw_number={} normalized_number={} timestamp={} page_count={:?} title={}",
		chapter.id,
		chapter.number,
		chapter_number,
		chapter.timestamp,
		chapter.page_count,
		chapter.title
	);
	let slug = slug
		.map(String::from)
		.unwrap_or_else(|| slugify_title(manga_title));
	Chapter {
		key: chapter.id.clone(),
		title: if chapter.title.trim().is_empty() {
			Some(format!("Capítulo {chapter_number}"))
		} else {
			Some(chapter.title.clone())
		},
		chapter_number: parse_chapter_number(&chapter_number),
		date_uploaded: date_from_timestamp_millis(chapter.timestamp),
		url: Some(chapter_url_from_slug_and_number(&slug, &chapter_number)),
		language: Some(String::from("pt-BR")),
		locked: false,
		..Default::default()
	}
}

fn resolve_chapter_identity(manga: &Manga, chapter: &Chapter) -> Result<(String, String, String)> {
	source_log!(
		"[toonlivre] resolve_chapter_identity start manga_key={} chapter_key={} chapter_url={:?}",
		manga.key,
		chapter.key,
		chapter.url.as_deref()
	);
	if manga.key.starts_with("obra-") && chapter.key.starts_with("cap-") {
		let chapter_url = chapter.url.clone().unwrap_or_else(|| {
			let slug = manga_slug_from_manga(manga).unwrap_or_else(|| slugify_title(&manga.title));
			let chapter_number = chapter_key_or_number(chapter).unwrap_or_default();
			chapter_url_from_slug_and_number(&slug, &chapter_number)
		});
		source_log!(
			"[toonlivre] resolve_chapter_identity direct manga_id={} chapter_id={} chapter_url={}",
			manga.key,
			chapter.key,
			chapter_url
		);
		return Ok((manga.key.clone(), chapter.key.clone(), chapter_url));
	}

	let slug = manga_slug_from_manga(manga).ok_or_else(|| {
		AidokuError::Message(String::from(
			"Unable to resolve manga slug for chapter lookup",
		))
	})?;
	let details = fetch_manga_by_slug(&slug)?;
	let target = chapter_key_or_number(chapter).ok_or_else(|| {
		AidokuError::Message(String::from(
			"Unable to resolve chapter number for page list",
		))
	})?;
	let matched = details
		.chapters
		.iter()
		.find(|candidate| {
			candidate.id == target || chapter_numbers_match(&candidate.number, &target)
		})
		.ok_or_else(|| AidokuError::Message(String::from("Chapter not found in manga data")))?;
	source_log!(
		"[toonlivre] resolve_chapter_identity matched target={} matched_id={} matched_number={} total_candidates={}",
		target,
		matched.id,
		matched.number,
		details.chapters.len()
	);
	let chapter_url = chapter
		.url
		.clone()
		.unwrap_or_else(|| chapter_url_from_slug_and_number(&slug, &matched.number));
	Ok((details.id, matched.id.clone(), chapter_url))
}

#[derive(Debug, Clone, Deserialize)]
struct WebViewChapterCache {
	chapter: ApiChapterDetails,
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
struct WebViewDebugSnapshot {
	href: String,
	title: String,
	#[serde(rename = "readyState")]
	ready_state: String,
	cookie: String,
	#[serde(rename = "userAgent")]
	user_agent: String,
	language: String,
	languages: Vec<String>,
	platform: String,
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
	#[serde(rename = "scrollY")]
	scroll_y: f64,
	#[serde(rename = "scrollHeight")]
	scroll_height: i32,
	#[serde(rename = "sessionStorageKeys")]
	session_storage_keys: Vec<String>,
	#[serde(rename = "sessionStorageLength")]
	session_storage_length: usize,
	resources: Vec<WebViewResourceSnapshot>,
	#[serde(rename = "bodyTextPreview")]
	body_text_preview: String,
	#[serde(rename = "bodyHtmlPreview")]
	body_html_preview: String,
}

struct WebViewCookieCache(RefCell<Option<String>>);

unsafe impl Sync for WebViewCookieCache {}

static WEBVIEW_COOKIE_CACHE: WebViewCookieCache = WebViewCookieCache(RefCell::new(None));

pub(crate) fn webview_chapter_storage_key(manga_id: &str, chapter_id: &str) -> String {
	format!("toonlivre_chapter_cache_v1:{manga_id}:{chapter_id}")
}

#[allow(dead_code)]
fn shorten_for_log(value: &str, max_chars: usize) -> String {
	let mut output: String = value.chars().take(max_chars).collect();
	if value.chars().nth(max_chars).is_some() {
		output.push_str("...");
	}
	output
}

fn fetch_webview_debug_snapshot(webview: &WebView) -> Result<WebViewDebugSnapshot> {
	let raw = webview.eval(
		"JSON.stringify({href: location.href, title: document.title, readyState: document.readyState, cookie: document.cookie || '', userAgent: navigator.userAgent, language: navigator.language || '', languages: navigator.languages || [], platform: navigator.platform || '', maxTouchPoints: navigator.maxTouchPoints || 0, referrer: document.referrer || '', visibilityState: document.visibilityState, innerWidth: window.innerWidth, innerHeight: window.innerHeight, scrollY: window.scrollY, scrollHeight: document.documentElement.scrollHeight || document.body.scrollHeight || 0, sessionStorageKeys: Object.keys(sessionStorage), sessionStorageLength: sessionStorage.length, resources: performance.getEntriesByType('resource').slice(-20).map(resource => ({name: resource.name, initiatorType: resource.initiatorType, transferSize: resource.transferSize, decodedBodySize: resource.decodedBodySize})), bodyTextPreview: document.body ? document.body.innerText.replace(/\\s+/g, ' ').trim().slice(0, 500) : '', bodyHtmlPreview: document.body ? document.body.innerHTML.replace(/\\s+/g, ' ').trim().slice(0, 500) : ''})",
	)?;
	serde_json::from_str(&raw).map_err(|error| {
		AidokuError::Message(format!("Failed to parse WebView snapshot.\nError: {error}"))
	})
}

fn webview_cookie_cache() -> Option<String> {
	WEBVIEW_COOKIE_CACHE.0.borrow().clone()
}

fn update_webview_cookie_cache(cookie_header: &str) {
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

fn build_webview_cookie_header() -> String {
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

fn force_webview_visible_layout(webview: &WebView) -> Result<()> {
	let _raw = webview.eval(
		r#"(() => {
			try {
				const width = 1280;
				const height = 1920;
				const patch = (target, key, descriptor) => {
					try {
						Object.defineProperty(target, key, descriptor);
						return true;
					} catch (error) {
						return false;
					}
				};
				const patched = {
					innerWidth: patch(window, 'innerWidth', { configurable: true, get: () => width }),
					innerHeight: patch(window, 'innerHeight', { configurable: true, get: () => height }),
					outerWidth: patch(window, 'outerWidth', { configurable: true, get: () => width }),
					outerHeight: patch(window, 'outerHeight', { configurable: true, get: () => height }),
					visibilityState: patch(document, 'visibilityState', { configurable: true, get: () => 'visible' }),
					hidden: patch(document, 'hidden', { configurable: true, get: () => false }),
					matchMedia: patch(window, 'matchMedia', {
						configurable: true,
						writable: true,
						value: (query) => {
							const minWidth = /min-width:\s*(\d+)px/.exec(query);
							const maxWidth = /max-width:\s*(\d+)px/.exec(query);
							const min = minWidth ? Number(minWidth[1]) : null;
							const max = maxWidth ? Number(maxWidth[1]) : null;
							const matches = (min === null || width >= min) && (max === null || width <= max);
							return {
								matches,
								media: query,
								onchange: null,
								addListener() {},
								removeListener() {},
								addEventListener() {},
								removeEventListener() {},
								dispatchEvent() {
									return false;
								},
							};
						},
					}),
				};
				document.dispatchEvent(new Event('visibilitychange'));
				window.dispatchEvent(new Event('resize'));
				window.dispatchEvent(new Event('focus'));
				window.dispatchEvent(new Event('orientationchange'));
				return JSON.stringify({
					patched,
					innerWidth: window.innerWidth,
					innerHeight: window.innerHeight,
					visibilityState: document.visibilityState,
					hidden: document.hidden,
					matchMedia: window.matchMedia('(max-width: 767px)').matches,
				});
			} catch (error) {
				return JSON.stringify({ error: String(error) });
			}
		})()"#,
	)?;
	source_log!("[toonlivre] webview layout patch result={_raw}");
	Ok(())
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

#[allow(dead_code)]
fn log_webview_debug_snapshot(_label: &str, _snapshot: &WebViewDebugSnapshot) {
	source_log!(
		"[toonlivre] {} href={} title={} ready_state={} visibility_state={} size={}x{} scroll_y={} scroll_height={} cookie={} user_agent={} language={} languages={} platform={} max_touch_points={} referrer={} session_storage_keys={} session_storage_length={}",
		_label,
		_snapshot.href,
		_snapshot.title,
		_snapshot.ready_state,
		_snapshot.visibility_state,
		_snapshot.inner_width,
		_snapshot.inner_height,
		_snapshot.scroll_y,
		_snapshot.scroll_height,
		shorten_for_log(&_snapshot.cookie, 240),
		shorten_for_log(&_snapshot.user_agent, 180),
		shorten_for_log(&_snapshot.language, 40),
		_snapshot.languages.join("|"),
		shorten_for_log(&_snapshot.platform, 40),
		_snapshot.max_touch_points,
		shorten_for_log(&_snapshot.referrer, 80),
		_snapshot.session_storage_keys.join("|"),
		_snapshot.session_storage_length,
	);
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

fn fetch_chapter_via_webview(
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

	let webview = WebView::new();

	// Inject script at document start to force visibility and patch layout size
	#[cfg(not(test))]
	let script_source = String::from(
		r#"(() => {
			try {
				const patch = (target, key, descriptor) => {
					try {
						Object.defineProperty(target, key, descriptor);
					} catch (e) {}
				};
				patch(document, 'visibilityState', { configurable: true, get: () => 'visible' });
				patch(document, 'hidden', { configurable: true, get: () => false });
				patch(window, 'innerWidth', { configurable: true, get: () => 1280 });
				patch(window, 'innerHeight', { configurable: true, get: () => 1920 });
				patch(window, 'outerWidth', { configurable: true, get: () => 1280 });
				patch(window, 'outerHeight', { configurable: true, get: () => 1920 });
				patch(navigator, 'language', { configurable: true, get: () => 'pt-BR' });
				patch(navigator, 'languages', { configurable: true, get: () => ['pt-BR', 'pt'] });
				patch(Event.prototype, 'isTrusted', { configurable: true, get: () => true });

				// Override fetch to force Accept-Language header
				const originalFetch = window.fetch;
				window.fetch = async function(resource, init) {
					init = init || {};
					init.headers = init.headers || {};
					if (init.headers instanceof Headers) {
						init.headers.set('Accept-Language', 'pt-BR,pt;q=0.9');
					} else if (Array.isArray(init.headers)) {
						let found = false;
						for (let i = 0; i < init.headers.length; i++) {
							if (init.headers[i][0].toLowerCase() === 'accept-language') {
								init.headers[i][1] = 'pt-BR,pt;q=0.9';
								found = true;
								break;
							}
						}
						if (!found) {
							init.headers.push(['Accept-Language', 'pt-BR,pt;q=0.9']);
						}
					} else {
						init.headers['Accept-Language'] = 'pt-BR,pt;q=0.9';
					}
					return originalFetch.apply(this, arguments);
				};

				// Override XMLHttpRequest to force Accept-Language header
				const originalOpen = XMLHttpRequest.prototype.open;
				XMLHttpRequest.prototype.open = function(method, url) {
					this._url = url;
					return originalOpen.apply(this, arguments);
				};
				const originalSend = XMLHttpRequest.prototype.send;
				XMLHttpRequest.prototype.send = function() {
					try {
						if (this._url && (this._url.startsWith('/') || this._url.includes('toonlivre.net'))) {
							this.setRequestHeader('Accept-Language', 'pt-BR,pt;q=0.9');
						}
					} catch (e) {}
					return originalSend.apply(this, arguments);
				};

				const triggerEvents = () => {
					window.dispatchEvent(new Event('scroll'));
					window.dispatchEvent(new MouseEvent('mousemove'));
					window.dispatchEvent(new Event('focus'));
				};
				setTimeout(triggerEvents, 500);
				setTimeout(triggerEvents, 1500);
				setTimeout(triggerEvents, 3000);
			} catch (e) {}
		})()"#,
	);

	#[cfg(not(test))]
	{
		let user_script = WebViewUserScript {
			source: script_source,
			at_document_end: false,
			for_main_frame_only: true,
		};

		if let Err(_error) = webview.add_user_script(user_script) {
			source_log!(
				"[toonlivre] Failed to add visibility patch user script: {:?}",
				_error
			);
		}
	}

	let cookie_header = build_webview_cookie_header();
	source_log!(
		"[toonlivre] webview cookie header prepared size={} value={}",
		cookie_header.len(),
		shorten_for_log(&cookie_header, 240)
	);

	// 1. Load the homepage to acquire Cloudflare clearance and other initialization cookies
	// only if we do not have cached cookies yet.
	if webview_cookie_cache().is_none() {
		let base_request = Request::get(crate::BASE_URL)?
			.header(
				"Accept",
				"text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
			)
			.header("Accept-Language", ACCEPT_LANGUAGE)
			.header("Cookie", cookie_header.as_str());

		source_log!("[toonlivre] No cached cookies. Loading base URL to acquire cookies");
		if let Err(_error) = webview.load_blocking(base_request) {
			source_log!("[toonlivre] Base URL load failed: {:?}", _error);
		}

		// Trigger early scroll/mousemove on homepage to ensure cookies are created
		let _ = webview.eval(
			"window.dispatchEvent(new Event('scroll')); window.dispatchEvent(new MouseEvent('mousemove'));",
		);
		sleep(1);
	} else {
		source_log!("[toonlivre] Using cached cookies. Skipping base URL load");
	}

	// 2. Load the chapter URL
	let request = Request::get(chapter_url)?
		.header(
			"Accept",
			"text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
		)
		.header("Accept-Language", ACCEPT_LANGUAGE)
		.header("Referer", crate::BASE_URL)
		.header("Cookie", cookie_header.as_str());

	source_log!("[toonlivre] Loading chapter URL: {}", chapter_url);
	webview.load_blocking(request).map_err(|error| {
		AidokuError::Message(format!(
			"WebView chapter load failed.\nURL: {chapter_url}\nError: {error:?}"
		))
	})?;

	force_webview_visible_layout(&webview)?;
	source_log!("[toonlivre] webview visibility/layout forced");
	let snapshot = fetch_webview_debug_snapshot(&webview)?;
	update_webview_cookie_cache(&snapshot.cookie);
	log_webview_debug_snapshot("webview after load", &snapshot);

	let storage_key = webview_chapter_storage_key(manga_id, chapter_id);
	for attempt in 1..=WEBVIEW_CHAPTER_LOAD_ATTEMPTS {
		let script = format!("sessionStorage.getItem({:?}) || \"\"", storage_key);
		let raw = webview.eval(&script).map_err(|error| {
			AidokuError::Message(format!(
				"WebView chapter eval failed.\nURL: {chapter_url}\nKey: {storage_key}\nError: {error:?}"
			))
		})?;
		let value = raw.trim();
		if !value.is_empty() && value != "null" && value != "undefined" {
			let chapter = parse_webview_chapter_cache(value, &storage_key)?;
			source_log!(
				"[toonlivre] webview chapter cache ready attempt={} key={} pages={}",
				attempt,
				storage_key,
				chapter.pages.len()
			);
			return Ok(chapter);
		}
		let snapshot = fetch_webview_debug_snapshot(&webview)?;
		update_webview_cookie_cache(&snapshot.cookie);
		log_webview_debug_snapshot(&format!("webview wait attempt={attempt}"), &snapshot);
		if attempt < WEBVIEW_CHAPTER_LOAD_ATTEMPTS {
			sleep(WEBVIEW_CHAPTER_LOAD_DELAY_SECONDS);
		}
	}

	bail!(
		"WebView chapter cache not populated. URL: {chapter_url} manga_id={manga_id} chapter_id={chapter_id}"
	)
}
