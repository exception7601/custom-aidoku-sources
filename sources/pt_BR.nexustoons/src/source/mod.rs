use aidoku::{
	AidokuError, Chapter, DeepLinkHandler, DeepLinkResult, Home, HomeComponent, HomeComponentValue,
	HomeLayout, HomePartialResult, ImageRequestProvider, Manga, MangaPageResult, Page, PageContent,
	PageContext, Result, Source,
	alloc::{String, Vec, vec},
	imports::{net::Request, std::send_partial_result},
	prelude::*,
};

use crate::{
	ACCEPT_LANGUAGE, chapter_key_or_id, chapter_url_from_slug_and_id, deep_link_result,
	fetch_releases, manga_slug_from_manga, search_mangas,
};

use self::{
	manga::{apply_details_from_webview, manga_to_link, manga_with_recent_chapter},
	webview::{fetch_chapter_pages_via_webview, fetch_manga_via_webview},
};

mod manga;
mod webview;
mod webview_support;

#[allow(unused_imports)]
pub(crate) use self::{
	manga::{chapter_from_api, manga_from_card, map_list_response},
	webview::{parse_webview_chapter_pages_cache, parse_webview_manga_cache},
	webview_support::{webview_chapter_storage_key, webview_manga_storage_key},
};

pub(crate) struct NexusToons;

const RELEASES_PAGE_SIZE: i32 = 50;
pub(super) const SEARCH_PAGE_SIZE: i32 = 24;
const HOME_PAGE_SIZE: usize = 12;
pub(super) const WEBVIEW_LOAD_ATTEMPTS: i32 = 4;
pub(super) const WEBVIEW_LOAD_DELAY_SECONDS: i32 = 1;
pub(super) const WEBVIEW_MANGA_CACHE_GLOBAL_KEY: &str = "__nexustoonsAidokuMangaCache";
pub(super) const WEBVIEW_CHAPTER_CACHE_GLOBAL_KEY: &str = "__nexustoonsAidokuChapterPagesCache";
#[cfg(not(test))]
pub(super) const WEBVIEW_VIEWPORT_WIDTH: i32 = 390;
#[cfg(not(test))]
pub(super) const WEBVIEW_VIEWPORT_HEIGHT: i32 = 844;
#[cfg(not(test))]
pub(super) const WEBVIEW_DEVICE_PIXEL_RATIO: f64 = 3.0;
#[cfg(not(test))]
pub(super) const WEBVIEW_MAX_TOUCH_POINTS: i32 = 5;
#[cfg(not(test))]
pub(super) const WEBVIEW_PLATFORM: &str = "iPhone";
#[cfg(not(test))]
pub(super) const WEBVIEW_VENDOR: &str = "Apple Computer, Inc.";
pub(super) const WEBVIEW_USER_AGENT: &str = concat!(
	"Mozilla/5.0 (iPhone; CPU iPhone OS 18_7 like Mac OS X) ",
	"AppleWebKit/605.1.15 (KHTML, like Gecko) Mobile/15E148",
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
		let slug = manga_slug_from_manga(&manga)
			.unwrap_or_else(|| String::from(manga.key.trim_matches('/')));
		let chapter_id = chapter_key_or_id(&chapter).unwrap_or_default();
		let chapter_url = chapter
			.url
			.clone()
			.or_else(|| {
				if slug.is_empty() || chapter_id.is_empty() {
					None
				} else {
					Some(chapter_url_from_slug_and_id(&slug, &chapter_id))
				}
			})
			.ok_or_else(|| {
				AidokuError::Message(String::from("Unable to resolve chapter URL for page list"))
			})?;
		let pages = fetch_chapter_pages_via_webview(&chapter_url, &slug, &chapter_id)?;
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
