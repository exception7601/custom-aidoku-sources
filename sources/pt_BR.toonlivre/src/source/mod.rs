use aidoku::{
	AidokuError, Chapter, DeepLinkHandler, DeepLinkResult, FilterValue, Home, HomeComponent,
	HomeComponentValue, HomeLayout, HomePartialResult, ImageRequestProvider, Manga,
	MangaPageResult, Page, PageContent, PageContext, Result, Source,
	alloc::{String, Vec, vec},
	imports::{net::Request, std::send_partial_result},
	prelude::*,
};

use crate::{
	ACCEPT_LANGUAGE, deep_link_result, fetch_manga_by_id, fetch_manga_by_slug, fetch_manga_reader,
	fetch_releases, manga_slug_from_manga,
};

use self::{
	manga::{
		apply_details_from_id, apply_details_from_reader, apply_details_from_slug, manga_from_card,
		manga_to_link, manga_with_recent_chapter, map_list_response, resolve_chapter_identity,
		search_response,
	},
	webview::fetch_chapter_via_webview,
};

mod manga;
mod webview;
mod webview_support;

pub(crate) use self::webview::{parse_webview_chapter_cache, webview_chapter_storage_key};

pub(crate) struct ToonLivre;

const RELEASES_PAGE_SIZE: i32 = 48;
pub(super) const SEARCH_PAGE_SIZE: i32 = 24;
const HOME_PAGE_SIZE: usize = 12;
pub(super) const WEBVIEW_CHAPTER_LOAD_ATTEMPTS: i32 = 10;
pub(super) const WEBVIEW_CHAPTER_LOAD_DELAY_SECONDS: i32 = 1;
pub(super) const WEBVIEW_CHAPTER_CACHE_GLOBAL_KEY: &str = "__toonlivreAidokuChapterCache";
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
			.header("User-Agent", WEBVIEW_USER_AGENT)
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
