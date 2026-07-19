use aidoku::{
	AidokuError, Chapter, DeepLinkHandler, DeepLinkResult, FilterValue, Home, HomeComponent,
	HomeComponentValue, HomeLayout, HomePartialResult, ImageRequestProvider, Link, LinkValue,
	Manga, MangaPageResult, MangaWithChapter, Page, PageContent, PageContext, Result, Source,
	Viewer,
	alloc::{String, Vec, vec},
	imports::{net::Request, std::send_partial_result},
	prelude::*,
};

use crate::{
	ACCEPT_LANGUAGE, ApiChapter, ApiListResponse, ApiMangaById, ApiMangaBySlug, ApiMangaCard,
	ApiReaderManga, chapter_key_or_number, chapter_numbers_match, chapter_url_from_slug_and_number,
	date_from_timestamp_millis, deep_link_result, fetch_chapter, fetch_manga_by_id,
	fetch_manga_by_slug, fetch_manga_reader, fetch_releases, manga_slug_from_manga,
	manga_status_from_text, manga_url_from_slug, normalize_chapter_number, parse_chapter_number,
	slugify_title,
};

pub(crate) struct ToonLivre;

const RELEASES_PAGE_SIZE: i32 = 48;
const SEARCH_PAGE_SIZE: i32 = 24;
const HOME_PAGE_SIZE: usize = 12;

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
		let raw_query = query.clone();
		let raw_page = page;
		let page = page.max(1);
		source_log!(
			"[toonlivre] get_search_manga_list start raw_page={} normalized_page={} query={:?}",
			raw_page,
			page,
			raw_query.as_deref()
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
		let chapter_details = fetch_chapter(&manga_id, &chapter_id)?;
		source_log!(
			"[toonlivre] get_page_list details id={} number={} timestamp={} pages={}",
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
	manga.authors = (!details.authors.is_empty()).then_some(details.authors.clone());
	manga.artists = (!details.artists.is_empty()).then_some(details.artists.clone());
	manga.tags = (!details.genres.is_empty()).then_some(details.genres.clone());
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
		manga.authors = (!details.authors.is_empty()).then_some(details.authors.clone());
		manga.artists = (!details.artists.is_empty()).then_some(details.artists.clone());
		manga.tags = (!details.genres.is_empty()).then_some(details.genres.clone());
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
		manga.authors = (!details.authors.is_empty()).then_some(details.authors.clone());
		manga.artists = (!details.artists.is_empty()).then_some(details.artists.clone());
		manga.tags = (!details.genres.is_empty()).then_some(details.genres.clone());
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
