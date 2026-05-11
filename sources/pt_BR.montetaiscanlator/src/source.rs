use aidoku::{
	Chapter, DeepLinkHandler, DeepLinkResult, FilterValue, ImageRequestProvider, Manga,
	MangaPageResult, Page, PageContent, PageContext, Result, Source,
	alloc::{String, Vec},
	imports::net::Request,
	prelude::*,
};

use crate::{
	BASE_URL, chapter_key_from_url, chapter_url, has_next_page, home_url, is_chapter_url,
	is_manga_url, manga_key_from_url, manga_url, normalize_text, parse_chapter_page_urls,
	parse_entries, parse_entries_fallback, parse_manga_chapters, parse_manga_cover,
	parse_manga_description, parse_manga_status, parse_manga_tags, parse_manga_title,
	parse_text_values, search_url,
};

pub(crate) struct MonteTaiScanlator;

impl Source for MonteTaiScanlator {
	fn new() -> Self {
		Self
	}

	fn get_search_manga_list(
		&self,
		query: Option<String>,
		page: i32,
		_filters: Vec<FilterValue>,
	) -> Result<MangaPageResult> {
		let page = page.max(1);
		let query = query
			.map(|value| normalize_text(&value))
			.filter(|value| !value.is_empty());
		let url = match query.as_deref() {
			Some(search_query) => search_url(search_query, page),
			None => home_url(page),
		};
		println!(
			"[montetai] search_start page={} query={} url={}",
			page,
			query.as_deref().unwrap_or(""),
			url
		);
		let html = Request::get(url)?.html()?;
		let mut entries = parse_entries(&html, query.is_some());
		let used_fallback = entries.is_empty();
		if used_fallback {
			entries = parse_entries_fallback(&html);
		}
		let has_next_page = has_next_page(&html);
		println!(
			"[montetai] search_result entries={} has_next={} fallback={}",
			entries.len(),
			has_next_page,
			used_fallback
		);
		Ok(MangaPageResult {
			entries,
			has_next_page,
		})
	}

	fn get_manga_update(
		&self,
		mut manga: Manga,
		needs_details: bool,
		needs_chapters: bool,
	) -> Result<Manga> {
		let manga_url = manga_url(&manga);
		println!(
			"[montetai] manga_update_start key={} details={} chapters={} url={}",
			manga.key, needs_details, needs_chapters, manga_url
		);
		let html = Request::get(&manga_url)?.html()?;

		if manga.key.is_empty() {
			if let Some(key) = manga_key_from_url(&manga_url) {
				manga.key = key;
			}
		}
		manga.url = Some(manga_url);

		if needs_details {
			if let Some(title) = parse_manga_title(&html) {
				manga.title = title;
			}
			if let Some(cover) = parse_manga_cover(&html) {
				manga.cover = Some(cover);
			}
			if let Some(description) = parse_manga_description(&html) {
				manga.description = Some(description);
			}

			let tags = parse_manga_tags(&html);
			if !tags.is_empty() {
				manga.tags = Some(tags);
			}

			let authors =
				parse_text_values(&html, ".post-content_item.mg_author .summary-content a");
			if !authors.is_empty() {
				manga.authors = Some(authors);
			}

			let artists =
				parse_text_values(&html, ".post-content_item.mg_artists .summary-content a");
			if !artists.is_empty() {
				manga.artists = Some(artists);
			}

			manga.status = parse_manga_status(&html);
		}

		if needs_chapters {
			let manga_key = (!manga.key.is_empty()).then_some(manga.key.as_str());
			let chapters = parse_manga_chapters(&html, manga_key);
			println!("[montetai] manga_update_chapters total={}", chapters.len());
			if !chapters.is_empty() {
				manga.chapters = Some(chapters);
			}
		}

		println!(
			"[montetai] manga_update_done title={} tags={} authors={} artists={} has_desc={} status={:?}",
			manga.title,
			manga.tags.as_ref().map(|v| v.len()).unwrap_or(0),
			manga.authors.as_ref().map(|v| v.len()).unwrap_or(0),
			manga.artists.as_ref().map(|v| v.len()).unwrap_or(0),
			manga
				.description
				.as_ref()
				.map(|v| !v.is_empty())
				.unwrap_or(false),
			manga.status
		);
		Ok(manga)
	}

	fn get_page_list(&self, manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		let chapter_page_url = chapter_url(&manga, &chapter);
		println!("[montetai] get_page_list chapter_url={chapter_page_url}");
		let html = Request::get(&chapter_page_url)?.html()?;
		let image_urls = parse_chapter_page_urls(&html);
		println!("[montetai] pages_found={}", image_urls.len());
		if image_urls.is_empty() {
			bail!("No chapter images found");
		}

		Ok(image_urls
			.into_iter()
			.map(|url| {
				let mut context = PageContext::new();
				context.insert(String::from("referer"), chapter_page_url.clone());
				Page {
					content: PageContent::url_context(url, context),
					..Default::default()
				}
			})
			.collect())
	}
}

impl DeepLinkHandler for MonteTaiScanlator {
	fn handle_deep_link(&self, url: String) -> Result<Option<DeepLinkResult>> {
		if !is_manga_url(&url) && !is_chapter_url(&url) {
			return Ok(None);
		}
		let Some(manga_key) = manga_key_from_url(&url) else {
			return Ok(None);
		};
		if is_chapter_url(&url) {
			if let Some(chapter_key) = chapter_key_from_url(&url) {
				return Ok(Some(DeepLinkResult::Chapter {
					manga_key,
					key: chapter_key,
				}));
			}
		}
		Ok(Some(DeepLinkResult::Manga { key: manga_key }))
	}
}

impl ImageRequestProvider for MonteTaiScanlator {
	fn get_image_request(&self, url: String, context: Option<PageContext>) -> Result<Request> {
		let mut request = Request::get(&url)?
			.header(
				"User-Agent",
				"Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1",
			)
			.header("Accept", "image/avif,image/webp,image/*,*/*;q=0.8")
			.header("Referer", BASE_URL);

		let mut referer_value = String::from(BASE_URL);
		if let Some(context) = context {
			if let Some(referer) = context.get("referer") {
				referer_value = referer.clone();
				request.set_header("Referer", referer.as_str());
			}
		}
		println!("[montetai] image_request url={url} referer={referer_value}");

		Ok(request)
	}
}
