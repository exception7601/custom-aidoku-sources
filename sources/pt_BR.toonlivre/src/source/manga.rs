use aidoku::{
	AidokuError, Chapter, Link, LinkValue, Manga, MangaPageResult, MangaWithChapter, Result,
	Viewer,
	alloc::{String, format},
};

use crate::{
	ApiChapter, ApiListResponse, ApiMangaById, ApiMangaBySlug, ApiMangaCard, ApiReaderManga,
	chapter_key_or_number, chapter_numbers_match, chapter_url_from_slug_and_number,
	date_from_timestamp_millis, fetch_manga_by_slug, manga_slug_from_manga, manga_status_from_text,
	manga_url_from_slug, normalize_chapter_number, parse_chapter_number, slugify_title,
};

use super::SEARCH_PAGE_SIZE;

pub(super) fn search_response(query: &str, page: i32) -> Result<ApiListResponse> {
	crate::search_mangas(query, page, SEARCH_PAGE_SIZE)
}

pub(super) fn map_list_response(response: &ApiListResponse) -> MangaPageResult {
	MangaPageResult {
		entries: response.mangas.iter().map(manga_from_card).collect(),
		has_next_page: response.pagination.has_next_page,
	}
}

pub(super) fn manga_from_card(card: &ApiMangaCard) -> Manga {
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

pub(super) fn manga_with_recent_chapter(card: &ApiMangaCard) -> Option<MangaWithChapter> {
	let manga = manga_from_card(card);
	let chapter = chapter_from_api(
		card.recent_chapters.first()?,
		card.slug.as_deref(),
		&card.title,
	);
	Some(MangaWithChapter { manga, chapter })
}

pub(super) fn manga_to_link(manga: Manga) -> Link {
	Link {
		title: manga.title.clone(),
		subtitle: manga.description.clone(),
		image_url: manga.cover.clone(),
		value: Some(LinkValue::Manga(manga)),
	}
}

pub(super) fn apply_details_from_id(manga: &mut Manga, details: &ApiMangaById) {
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

pub(super) fn apply_details_from_reader(
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

pub(super) fn apply_details_from_slug(
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

pub(super) fn resolve_chapter_identity(
	manga: &Manga,
	chapter: &Chapter,
) -> Result<(String, String, String)> {
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
