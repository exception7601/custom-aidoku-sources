use aidoku::{
	Chapter, Link, LinkValue, Manga, MangaPageResult, MangaStatus, MangaWithChapter, Viewer,
	alloc::{String, Vec, format},
	imports::std::current_date,
};
use serde::Deserialize;

use crate::{
	ApiChapter, ApiListResponse, ApiMangaCard, chapter_title_from_number,
	chapter_url_from_slug_and_id, manga_status_from_text, manga_url_from_slug,
	parse_chapter_number,
};

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct WebViewMangaDetails {
	pub(crate) title: String,
	#[serde(
		default,
		rename = "coverUrl",
		alias = "cover_url",
		alias = "coverImage",
		alias = "bannerImage"
	)]
	pub(crate) cover_url: Option<String>,
	#[serde(default)]
	pub(crate) description: Option<String>,
	#[serde(default)]
	pub(crate) status: Option<String>,
	#[serde(default)]
	pub(crate) chapters: Vec<WebViewMangaChapter>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct WebViewMangaChapter {
	pub(crate) id: i64,
	#[serde(default)]
	pub(crate) number: Option<String>,
	#[serde(default)]
	pub(crate) title: Option<String>,
	#[serde(default, rename = "dateUploaded", alias = "date_uploaded")]
	pub(crate) date_uploaded: Option<String>,
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

pub(super) fn manga_with_recent_chapter(card: &ApiMangaCard) -> Option<MangaWithChapter> {
	let manga = manga_from_card(card);
	let chapter = chapter_from_api(card.recent_chapters.as_ref()?.first()?, manga.key.as_str());
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

pub(super) fn apply_details_from_webview(
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
			.unwrap_or(MangaStatus::Unknown);
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

pub(super) fn chapter_sort_key_from_webview(
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
