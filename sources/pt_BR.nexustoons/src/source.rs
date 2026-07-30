use aidoku::{
	AidokuError, Chapter, DeepLinkHandler, DeepLinkResult, Home, HomeComponent, HomeComponentValue,
	HomeLayout, HomePartialResult, ImageRequestProvider, Link, LinkValue, Manga, MangaPageResult,
	MangaWithChapter, Page, PageContent, PageContext, Result, Source, Viewer,
	alloc::{String, Vec, vec},
	imports::{
		js::WebView,
		net::Request,
		std::{current_date, send_partial_result, sleep},
	},
	prelude::*,
};

#[cfg(not(test))]
use aidoku::imports::js::WebViewUserScript;

use serde::Deserialize;

use crate::{
	ACCEPT_LANGUAGE, ApiChapter, ApiListResponse, ApiMangaCard, chapter_key_or_id,
	chapter_title_from_number, chapter_url_from_slug_and_id, deep_link_result, fetch_releases,
	manga_slug_from_manga, manga_status_from_text, manga_url_from_slug, parse_chapter_number,
	search_mangas,
};

pub(crate) struct NexusToons;

const RELEASES_PAGE_SIZE: i32 = 50;
const SEARCH_PAGE_SIZE: i32 = 24;
const HOME_PAGE_SIZE: usize = 12;
const WEBVIEW_CHAPTER_LOAD_ATTEMPTS: i32 = 4;
const WEBVIEW_CHAPTER_LOAD_DELAY_SECONDS: i32 = 1;
const WEBVIEW_USER_AGENT: &str = concat!(
	"Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) ",
	"AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 ",
	"Mobile/15E148 Safari/604.1",
);

impl Source for NexusToons {
	fn new() -> Self {
		Self
	}

	fn get_search_manga_list(
		&self,
		query: Option<String>,
		page: i32,
		_filters: Vec<aidoku::FilterValue>,
	) -> Result<MangaPageResult> {
		let page = page.max(1);
		let raw_query = query.clone();
		source_log!(
			"[nexustoons] get_search_manga_list start page={} query={:?}",
			page,
			raw_query.as_deref()
		);
		let response = match query.map(|value| String::from(value.trim())) {
			Some(query) if !query.is_empty() => search_mangas(&query, page, SEARCH_PAGE_SIZE)?,
			_ => fetch_releases(page, RELEASES_PAGE_SIZE)?,
		};
		source_log!(
			"[nexustoons] get_search_manga_list response items={} page={} pages={} total={}",
			response.data.len(),
			response.page,
			response.pages,
			response.total
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
			"[nexustoons] get_manga_update start key={} title={} needs_details={} needs_chapters={}",
			manga.key,
			manga.title,
			needs_details,
			needs_chapters
		);
		let slug = manga_slug_from_manga(&manga)
			.ok_or_else(|| AidokuError::Message(String::from("Unable to resolve manga slug")))?;
		let details = fetch_manga_via_webview(&slug)?;
		source_log!(
			"[nexustoons] get_manga_update fetched title={} chapters={} cover={:?}",
			details.title,
			details.chapters.len(),
			details.cover_url
		);
		apply_details_from_webview(&mut manga, &details, &slug, needs_details, needs_chapters);
		source_log!(
			"[nexustoons] get_manga_update done key={} chapters={} url={:?}",
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
			"[nexustoons] get_page_list start manga_key={} manga_title={} chapter_key={} chapter_url={:?}",
			manga.key,
			manga.title,
			chapter.key,
			chapter.url.as_deref()
		);
		let chapter_url = chapter
			.url
			.clone()
			.or_else(|| {
				let slug = manga_slug_from_manga(&manga)
					.unwrap_or_else(|| String::from(manga.key.trim_matches('/')));
				let chapter_id = chapter_key_or_id(&chapter).unwrap_or_default();
				if slug.is_empty() || chapter_id.is_empty() {
					None
				} else {
					Some(chapter_url_from_slug_and_id(&slug, &chapter_id))
				}
			})
			.ok_or_else(|| {
				AidokuError::Message(String::from("Unable to resolve chapter URL for page list"))
			})?;
		let pages = fetch_chapter_pages_via_webview(&chapter_url)?;
		source_log!(
			"[nexustoons] get_page_list fetched page_urls={} chapter_url={}",
			pages.len(),
			chapter_url
		);
		if pages.is_empty() {
			bail!("No chapter pages found");
		}
		Ok(pages
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

impl Home for NexusToons {
	fn get_home(&self) -> Result<HomeLayout> {
		source_log!("[nexustoons] get_home start releases_page_size={RELEASES_PAGE_SIZE}");
		let response = fetch_releases(1, RELEASES_PAGE_SIZE)?;
		source_log!(
			"[nexustoons] get_home response items={} page={} pages={} total={}",
			response.data.len(),
			response.page,
			response.pages,
			response.total
		);
		let entries = response
			.data
			.iter()
			.take(HOME_PAGE_SIZE)
			.map(manga_from_card)
			.collect::<Vec<_>>();
		let recent_chapters = response
			.data
			.iter()
			.filter_map(manga_with_recent_chapter)
			.take(HOME_PAGE_SIZE)
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

impl DeepLinkHandler for NexusToons {
	fn handle_deep_link(&self, url: String) -> Result<Option<DeepLinkResult>> {
		Ok(deep_link_result(&url))
	}
}

impl ImageRequestProvider for NexusToons {
	fn get_image_request(&self, url: String, context: Option<PageContext>) -> Result<Request> {
		let referer = context
			.as_ref()
			.and_then(|ctx| ctx.get("referer"))
			.map(String::as_str)
			.unwrap_or(crate::BASE_URL);
		source_log!(
			"[nexustoons] get_image_request url={} referer={}",
			url,
			referer
		);
		let mut request = Request::get(&url)?
			.header("User-Agent", WEBVIEW_USER_AGENT)
			.header("Accept", "image/avif,image/webp,image/*,*/*;q=0.8")
			.header("accept-language", ACCEPT_LANGUAGE);
		request.set_header(String::from("Referer"), String::from(referer));
		Ok(request)
	}
}

pub(crate) fn map_list_response(response: &ApiListResponse) -> MangaPageResult {
	MangaPageResult {
		entries: response.data.iter().map(manga_from_card).collect(),
		has_next_page: response.page < response.pages,
	}
}

pub(crate) fn manga_from_card(card: &ApiMangaCard) -> Manga {
	let slug = card.slug.clone().unwrap_or_else(|| format!("{}", card.id));
	Manga {
		key: slug.clone(),
		title: card.title.clone(),
		cover: card.cover_url.clone(),
		description: card.alternative_title.clone(),
		url: Some(manga_url_from_slug(&slug)),
		viewer: Viewer::Vertical,
		content_rating: aidoku::ContentRating::Safe,
		..Default::default()
	}
}

fn manga_with_recent_chapter(card: &ApiMangaCard) -> Option<MangaWithChapter> {
	let manga = manga_from_card(card);
	let chapter = chapter_from_api(card.recent_chapters.as_ref()?.first()?, manga.key.as_str());
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

#[derive(Debug, Clone, Deserialize)]
struct WebViewMangaDetails {
	title: String,
	#[serde(
		default,
		rename = "coverUrl",
		alias = "cover_url",
		alias = "coverImage",
		alias = "bannerImage"
	)]
	cover_url: Option<String>,
	#[serde(default)]
	description: Option<String>,
	#[serde(default)]
	status: Option<String>,
	#[serde(default)]
	chapters: Vec<WebViewMangaChapter>,
}

#[derive(Debug, Clone, Deserialize)]
struct WebViewMangaChapter {
	id: i64,
	#[serde(default)]
	number: Option<String>,
	#[serde(default)]
	title: Option<String>,
	#[serde(
		default,
		alias = "date_uploaded",
		alias = "dateUploaded",
		alias = "createdAt"
	)]
	date_uploaded: Option<serde_json::Value>,
}

fn fetch_manga_via_webview(slug: &str) -> Result<WebViewMangaDetails> {
	source_log!("[nexustoons] fetch_manga_via_webview start slug={}", slug);
	let webview = WebView::new();
	add_webview_visibility_patch(&webview);
	add_webview_json_capture(&webview);

	let url = manga_url_from_slug(slug);
	let request = Request::get(&url)?
		.header(
			"Accept",
			"text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
		)
		.header("Accept-Language", ACCEPT_LANGUAGE)
		.header("User-Agent", WEBVIEW_USER_AGENT)
		.header("Referer", crate::BASE_URL);

	webview.load_blocking(request).map_err(|error| {
		AidokuError::Message(format!(
			"WebView manga load failed.\nURL: {url}\nError: {error:?}"
		))
	})?;
	sleep(WEBVIEW_CHAPTER_LOAD_DELAY_SECONDS);
	webview.eval(&chapter_list_open_script()).map_err(|error| {
		AidokuError::Message(format!(
			"WebView manga open chapters failed.\nURL: {url}\nError: {error:?}"
		))
	})?;

	let mut captured = String::new();
	for attempt in 1..=4 {
		sleep(WEBVIEW_CHAPTER_LOAD_DELAY_SECONDS);
		captured = webview
			.eval(&chapter_list_captured_json_script())
			.map_err(|error| {
				AidokuError::Message(format!(
					"WebView manga capture failed.\nURL: {url}\nError: {error:?}"
				))
			})?;
		source_log!(
			"[nexustoons] fetch_manga_via_webview attempt={} captured_json_len={}",
			attempt,
			captured.len()
		);
		if !captured.trim().is_empty() {
			break;
		}
	}

	let capture_details = if !captured.trim().is_empty() {
		Some(
			serde_json::from_str::<WebViewMangaDetails>(&captured).map_err(|error| {
				AidokuError::Message(format!(
					"Failed to parse captured manga data.\nURL: {url}\nError: {error}"
				))
			})?,
		)
	} else {
		None
	};

	let raw = webview
		.eval(&chapter_list_extract_script())
		.map_err(|error| {
			AidokuError::Message(format!(
				"WebView manga extract failed.\nURL: {url}\nError: {error:?}"
			))
		})?;
	source_log!("[nexustoons] fetch_manga_via_webview raw_len={}", raw.len());
	let extract_details = serde_json::from_str::<WebViewMangaDetails>(&raw).map_err(|error| {
		AidokuError::Message(format!(
			"Failed to parse manga data from WebView.\nURL: {url}\nError: {error}"
		))
	})?;

	let mut details = match (capture_details, extract_details) {
		(Some(captured_details), extracted_details) => {
			if captured_details.chapters.len() >= extracted_details.chapters.len() {
				captured_details
			} else {
				extracted_details
			}
		}
		(None, extracted_details) => extracted_details,
	};
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

fn apply_details_from_webview(
	manga: &mut Manga,
	details: &WebViewMangaDetails,
	slug: &str,
	needs_details: bool,
	needs_chapters: bool,
) {
	if needs_details {
		manga.key = String::from(slug);
		manga.title = details.title.clone();
		manga.cover = details.cover_url.clone();
		manga.authors = Some(Vec::new());
		manga.artists = Some(Vec::new());
		manga.tags = Some(Vec::new());
		manga.description = details.description.clone();
		manga.status = details
			.status
			.as_deref()
			.map(manga_status_from_text)
			.unwrap_or_default();
		manga.url = Some(manga_url_from_slug(slug));
		manga.viewer = Viewer::Vertical;
		manga.content_rating = aidoku::ContentRating::Safe;
	}
	if needs_chapters {
		manga.chapters = Some(
			details
				.chapters
				.iter()
				.map(|chapter| chapter_from_webview(chapter, slug))
				.collect(),
		);
	}
}

fn chapter_from_webview(chapter: &WebViewMangaChapter, slug: &str) -> Chapter {
	source_log!(
		"[nexustoons] chapter_from_webview id={} number={:?} date_uploaded={:?} slug={}",
		chapter.id,
		chapter.number,
		chapter.date_uploaded,
		slug
	);
	let chapter_id = format!("{}", chapter.id);
	let number = chapter.number.clone().unwrap_or_default();
	Chapter {
		key: chapter_id.clone(),
		title: Some(
			match chapter
				.title
				.as_deref()
				.map(str::trim)
				.filter(|value| !value.is_empty())
			{
				Some(title) => String::from(title),
				None => chapter_title_from_number(&number),
			},
		),
		chapter_number: parse_chapter_number(&number),
		date_uploaded: Some(current_date()),
		url: Some(chapter_url_from_slug_and_id(slug, &chapter_id)),
		language: Some(String::from("pt-BR")),
		locked: false,
		..Default::default()
	}
}

fn chapter_sort_key_from_webview(
	left: &WebViewMangaChapter,
	right: &WebViewMangaChapter,
) -> core::cmp::Ordering {
	let left_number = left
		.number
		.as_deref()
		.unwrap_or("")
		.parse::<f32>()
		.unwrap_or(f32::MAX);
	let right_number = right
		.number
		.as_deref()
		.unwrap_or("")
		.parse::<f32>()
		.unwrap_or(f32::MAX);
	left_number
		.partial_cmp(&right_number)
		.unwrap_or(core::cmp::Ordering::Equal)
		.then_with(|| left.id.cmp(&right.id))
}

fn chapter_list_open_script() -> String {
	String::from(
		r#"(function() {
  const button = Array.from(document.querySelectorAll('button')).find((el) => /Cap[ií]tulos/i.test(el.textContent || ''));
  if (button) {
    button.click();
  }
  return 'ok';
})()"#,
	)
}

fn chapter_list_captured_json_script() -> String {
	String::from(
		r#"(function() {
  const captured = window.__nexusToonsCapturedJson;
  return captured && typeof captured.json === 'string' ? captured.json : '';
})()"#,
	)
}

fn chapter_list_extract_script() -> String {
	String::from(
		r#"(function() {
  const root = Array.from(document.querySelectorAll('div.custom-scrollbar'))
    .find((el) => el.querySelectorAll('a[href^="/r/"]').length > 0);
  if (!root) {
    return '';
  }
  const fiberKey = Object.getOwnPropertyNames(root).find((key) => key.startsWith('__reactFiber'));
  const fiber = fiberKey ? root[fiberKey] : null;
  if (!fiber) {
    return '';
  }
  const seen = new Set();
  const queue = [fiber];
  const push = (value) => {
    if (value && typeof value === 'object' && !seen.has(value)) {
      seen.add(value);
      queue.push(value);
    }
  };
  const isChapter = (chapter) =>
    chapter &&
    typeof chapter === 'object' &&
    typeof chapter.id === 'number' &&
    typeof chapter.number === 'string';
  const toTimestamp = (value) => {
    if (!value) {
      return null;
    }
    const timestamp = new Date(value).getTime();
    return Number.isFinite(timestamp) ? Math.floor(timestamp / 1000) : null;
  };
  const buildResult = (value) => {
    const source = value.manga && typeof value.manga === 'object' ? value.manga : value;
    const chapters = Array.isArray(value.chapters)
      ? value.chapters.filter(isChapter).map((chapter) => ({
          id: chapter.id,
          number: chapter.number,
          title: typeof chapter.title === 'string' ? chapter.title : '',
          date_uploaded: toTimestamp(chapter.createdAt || chapter.dateUploaded || chapter.date_uploaded),
        }))
      : [];
    return JSON.stringify({
      title: source.title || document.title || '',
      coverUrl:
        source.coverImage ||
        source.cover_url ||
        source.coverUrl ||
        document.querySelector('img[src*="/covers/"]')?.currentSrc ||
        document.querySelector('meta[property="og:image"]')?.getAttribute('content') ||
        null,
      description: source.description || null,
      status: source.status || null,
      chapters,
    });
  };
  let best = null;
  while (queue.length) {
    const current = queue.shift();
    if (Array.isArray(current)) {
      for (const item of current.slice(0, 20)) {
        push(item);
      }
      continue;
    }
    if (current && typeof current === 'object') {
      const chapters = Array.isArray(current.chapters) ? current.chapters.filter(isChapter) : [];
      if (chapters.length > 0) {
        if (!best || chapters.length > best.chapters.length) {
          best = { value: current, chapters };
        }
      }
      for (const key of Object.keys(current).slice(0, 50)) {
        try {
          push(current[key]);
        } catch (error) {}
      }
      const proto = Object.getPrototypeOf(current);
      if (proto && proto !== Object.prototype) {
        push(proto);
      }
    }
  }
  return best ? buildResult(best.value) : '';
})()"#,
	)
}

pub(crate) fn chapter_from_api(chapter: &ApiChapter, slug: &str) -> Chapter {
	let chapter_number = String::from(chapter.number.trim());
	source_log!(
		"[nexustoons] chapter_from_api id={} number={} timestamp={:?} page_count={:?}",
		chapter.id,
		chapter_number,
		chapter.timestamp,
		chapter.page_count
	);
	let chapter_id = format!("{}", chapter.id);
	Chapter {
		key: chapter_id.clone(),
		title: match chapter
			.title
			.as_deref()
			.map(str::trim)
			.filter(|value| !value.is_empty())
		{
			Some(title) => Some(String::from(title)),
			None => Some(chapter_title_from_number(&chapter_number)),
		},
		chapter_number: parse_chapter_number(&chapter_number),
		date_uploaded: Some(current_date()),
		url: Some(chapter_url_from_slug_and_id(slug, &chapter_id)),
		language: Some(String::from("pt-BR")),
		locked: false,
		..Default::default()
	}
}

fn fetch_chapter_pages_via_webview(chapter_url: &str) -> Result<Vec<String>> {
	source_log!(
		"[nexustoons] fetch_chapter_pages_via_webview start url={}",
		chapter_url
	);
	let webview = WebView::new();
	add_webview_visibility_patch(&webview);
	source_log!(
		"[nexustoons] fetch_chapter_pages_via_webview patch_applied url={}",
		chapter_url
	);

	let request = Request::get(chapter_url)?
		.header(
			"Accept",
			"text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
		)
		.header("Accept-Language", ACCEPT_LANGUAGE)
		.header("User-Agent", WEBVIEW_USER_AGENT)
		.header("Referer", crate::BASE_URL);

	webview.load_blocking(request).map_err(|error| {
		source_log!(
			"[nexustoons] fetch_chapter_pages_via_webview load_failed url={} error={:?}",
			chapter_url,
			error
		);
		AidokuError::Message(format!(
			"WebView chapter load failed.\nURL: {chapter_url}\nError: {error:?}"
		))
	})?;

	sleep(WEBVIEW_CHAPTER_LOAD_DELAY_SECONDS);

	for attempt in 1..=WEBVIEW_CHAPTER_LOAD_ATTEMPTS {
		source_log!(
			"[nexustoons] fetch_chapter_pages_via_webview load_ok url={}",
			chapter_url
		);
		let raw = webview
			.eval(&chapter_image_collector_script())
			.map_err(|error| {
				source_log!(
					"[nexustoons] fetch_chapter_pages_via_webview eval_failed url={} error={:?}",
					chapter_url,
					error
				);
				AidokuError::Message(format!(
					"WebView chapter eval failed.\nURL: {chapter_url}\nError: {error:?}"
				))
			})?;
		let pages = parse_image_urls(&raw)?;
		source_log!(
			"[nexustoons] fetch_chapter_pages_via_webview attempt={} pages={}",
			attempt,
			pages.len()
		);
		if !pages.is_empty() {
			return Ok(pages);
		}
		if attempt < WEBVIEW_CHAPTER_LOAD_ATTEMPTS {
			sleep(WEBVIEW_CHAPTER_LOAD_DELAY_SECONDS);
		}
	}

	Ok(Vec::new())
}

fn parse_image_urls(raw: &str) -> Result<Vec<String>> {
	serde_json::from_str(raw).map_err(|error| {
		AidokuError::Message(format!(
			"Failed to parse chapter image URLs.\nError: {error}"
		))
	})
}

fn chapter_image_collector_script() -> String {
	String::from(
		r#"(function() {
  const seen = new Set();
  const out = [];
  const normalize = (img) => img.currentSrc || img.dataset?.src || img.dataset?.lazySrc || img.getAttribute('data-src') || img.getAttribute('src') || img.src || '';
  const collect = () => {
    for (const img of document.querySelectorAll('img')) {
      const value = normalize(img);
      if (!value || !value.includes('manga_pages')) {
        continue;
      }
      if (!seen.has(value)) {
        seen.add(value);
        out.push(value);
      }
    }
  };
  collect();
  const max = Math.max(0, document.documentElement.scrollHeight - window.innerHeight);
  for (let step = 1; step <= 24; step++) {
    window.scrollTo(0, Math.round((max * step) / 24));
    window.dispatchEvent(new Event('scroll'));
    collect();
  }
  window.scrollTo(0, document.documentElement.scrollHeight);
  collect();
  return JSON.stringify(out);
})()"#,
	)
}

#[cfg(not(test))]
fn add_webview_visibility_patch(webview: &WebView) {
	let user_script = WebViewUserScript {
		source: String::from(
			r#"(() => {
  try {
    const patch = (target, key, descriptor) => {
      try {
        Object.defineProperty(target, key, descriptor);
      } catch (error) {}
    };

    patch(document, 'visibilityState', { configurable: true, get: () => 'visible' });
    patch(document, 'hidden', { configurable: true, get: () => false });
    patch(window, 'innerHeight', { configurable: true, get: () => 1920 });
    patch(window, 'outerHeight', { configurable: true, get: () => 1920 });

    const originalIntersectionObserver = window.IntersectionObserver;
    if (originalIntersectionObserver) {
      window.IntersectionObserver = class extends originalIntersectionObserver {
        constructor(callback, options) {
          super((entries, observer) => {
            const forced = entries.map((entry) => ({
              ...entry,
              isIntersecting: true,
              intersectionRatio: 1,
            }));
            callback(forced, observer);
          }, options);
        }
      };
    }

    const trigger = () => {
      window.dispatchEvent(new Event('scroll'));
      window.dispatchEvent(new MouseEvent('mousemove'));
      window.dispatchEvent(new Event('focus'));
    };

    setTimeout(trigger, 500);
    setTimeout(trigger, 1500);
    setTimeout(trigger, 3000);
  } catch (error) {}
})()"#,
		),
		at_document_end: false,
		for_main_frame_only: true,
	};

	if let Err(error) = webview.add_user_script(user_script) {
		source_log!(
			"[nexustoons] Failed to add visibility patch user script: {:?}",
			error
		);
	}
}

#[cfg(not(test))]
fn add_webview_json_capture(webview: &WebView) {
	let user_script = WebViewUserScript {
		source: String::from(
			r#"(() => {
  try {
    const captured = { json: '', count: 0 };
    const shouldCapture = (value) => {
      return (
        value &&
        typeof value === 'object' &&
        typeof value.title === 'string' &&
        Array.isArray(value.chapters) &&
        value.chapters.some((chapter) => chapter && typeof chapter.id === 'number' && typeof chapter.number === 'string')
      );
    };

    const store = (value) => {
      try {
        const count = Array.isArray(value.chapters) ? value.chapters.length : 0;
        if (count >= captured.count) {
          captured.count = count;
          captured.json = JSON.stringify({
            title: value.title || '',
            coverUrl: value.coverUrl || value.coverImage || value.cover_url || null,
            description: value.description || null,
            status: value.status || null,
            chapters: (value.chapters || []).map((chapter) => ({
              id: chapter.id,
              number: chapter.number,
              title: typeof chapter.title === 'string' ? chapter.title : '',
              date_uploaded: null,
            })),
          });
        }
      } catch (error) {}
    };

    const originalParse = JSON.parse;
    JSON.parse = function(text, reviver) {
      const value = originalParse.call(this, text, reviver);
      try {
        if (shouldCapture(value)) {
          store(value);
        }
      } catch (error) {}
      return value;
    };

    const originalFetch = window.fetch;
    if (originalFetch) {
      window.fetch = async (...args) => {
        const response = await originalFetch(...args);
        try {
          const url = args[0] instanceof Request ? args[0].url : String(args[0] || '');
          if (String(url).includes('/api/manga/')) {
            const clone = response.clone();
            clone.json().then((value) => {
              if (shouldCapture(value)) {
                store(value);
              }
            }).catch(() => {});
          }
        } catch (error) {}
        return response;
      };
    }

    const originalOpen = XMLHttpRequest.prototype.open;
    const originalSend = XMLHttpRequest.prototype.send;
    XMLHttpRequest.prototype.open = function(method, url, ...rest) {
      this.__nexusToonsUrl = String(url || '');
      return originalOpen.call(this, method, url, ...rest);
    };
    XMLHttpRequest.prototype.send = function(body) {
      this.addEventListener('load', () => {
        try {
          if (String(this.__nexusToonsUrl || '').includes('/api/manga/')) {
            const text = this.responseText || '';
            if (text) {
              try {
                const value = JSON.parse(text);
                if (shouldCapture(value)) {
                  store(value);
                }
              } catch (error) {}
            }
          }
        } catch (error) {}
      });
      return originalSend.call(this, body);
    };

    window.__nexusToonsCapturedJson = captured;
  } catch (error) {}
})()"#,
		),
		at_document_end: false,
		for_main_frame_only: true,
	};

	if let Err(error) = webview.add_user_script(user_script) {
		source_log!(
			"[nexustoons] Failed to add JSON capture user script: {:?}",
			error
		);
	}
}

#[cfg(test)]
fn add_webview_visibility_patch(_webview: &WebView) {}

#[cfg(test)]
fn add_webview_json_capture(_webview: &WebView) {}
