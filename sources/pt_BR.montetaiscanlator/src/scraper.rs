use aidoku::{
	Chapter, Manga, MangaStatus,
	alloc::{String, Vec},
	imports::{
		html::{Document, Element},
		std::current_date,
	},
	prelude::*,
};
use serde::Deserialize;

use crate::{
	BASE_URL, absolute_url, attr_url, chapter_date_from_text, chapter_key_from_url,
	chapter_title_from_text, cover_image_url, image_url, is_chapter_image, is_chapter_url,
	is_likely_cover_url, is_manga_url, looks_like_series_title, manga_key_from_url, meta_content,
	normalize_text, parse_chapter_number, percent_encode,
};

pub(crate) fn home_url(page: i32) -> String {
	if page <= 1 {
		String::from(BASE_URL)
	} else {
		format!("{BASE_URL}/page/{page}/")
	}
}

pub(crate) fn search_url(query: &str, page: i32) -> String {
	let encoded = percent_encode(query);
	if page <= 1 {
		format!("{BASE_URL}/?s={encoded}&post_type=wp-manga")
	} else {
		format!("{BASE_URL}/page/{page}/?s={encoded}&post_type=wp-manga")
	}
}

fn body_element(document: &Document) -> Option<Element> {
	document
		.select_first("body")
		.or_else(|| document.select_first("html"))
}

pub(crate) fn parse_entries(document: &Document, query_mode: bool) -> Vec<Manga> {
	let mut entries = parse_entries_from_urls(document);
	if entries.is_empty() {
		entries = if query_mode {
			let mut entries = parse_entries_from_selector(document, ".c-tabs-item__content");
			if entries.is_empty() {
				entries = parse_entries_from_selector(document, "article.mt-manga-catalog-card");
			}
			entries
		} else {
			let mut entries =
				parse_entries_from_selector(document, "article.mt-manga-catalog-card");
			if entries.is_empty() {
				entries = parse_entries_from_selector(document, ".c-tabs-item__content");
			}
			entries
		};
	}
	entries
}

fn collect_entries_from_links(element: &Element, entries: &mut Vec<Manga>) {
	if element.tag_name().as_deref() == Some("a") {
		if let Some(manga) = manga_from_link(element) {
			if !entries.iter().any(|entry: &Manga| entry.key == manga.key) {
				entries.push(manga);
			}
		}
	}

	for child in element.children() {
		collect_entries_from_links(&child, entries);
	}
}

fn parse_entries_from_urls(document: &Document) -> Vec<Manga> {
	let mut entries = Vec::new();
	if let Some(body) = body_element(document) {
		collect_entries_from_links(&body, &mut entries);
	}
	entries
}

fn manga_from_link(link: &Element) -> Option<Manga> {
	let url = attr_url(link, "href")?;
	if !is_manga_url(&url) {
		return None;
	}

	let key = manga_key_from_url(&url)?;
	let title = extract_title_from_link(link, &url)?;
	let cover = extract_cover_from_link(link)?;
	Some(Manga {
		key,
		title,
		cover: Some(cover),
		url: Some(url),
		..Default::default()
	})
}

fn find_series_title_in_heading_subtree(element: &Element) -> Option<String> {
	if let Some(tag_name) = element.tag_name() {
		match tag_name.as_str() {
			"h1" | "h2" | "h3" | "h4" => {
				let text = normalize_text(&element.text().unwrap_or_default());
				if looks_like_series_title(&text) {
					return Some(text);
				}
			}
			"a" => {
				let text = normalize_text(&element.text().unwrap_or_default());
				if looks_like_series_title(&text) {
					return Some(text);
				}

				for attr_name in ["aria-label", "title"] {
					if let Some(value) = element.attr(attr_name) {
						let mut candidate = normalize_text(&value);
						if let Some(stripped) = candidate.strip_prefix("Abrir ") {
							candidate = String::from(stripped);
						}
						if looks_like_series_title(&candidate) {
							return Some(candidate);
						}
					}
				}
			}
			_ => {}
		}
	}

	for child in element.children() {
		if let Some(title) = find_series_title_in_heading_subtree(&child) {
			return Some(title);
		}
	}

	None
}

fn find_series_title_in_image_subtree(element: &Element) -> Option<String> {
	if element.tag_name().as_deref() == Some("img") {
		if let Some(alt_text) = element.attr("alt") {
			let candidate = normalize_text(&alt_text);
			if looks_like_series_title(&candidate) {
				return Some(candidate);
			}
		}
	}

	for child in element.children() {
		if let Some(title) = find_series_title_in_image_subtree(&child) {
			return Some(title);
		}
	}

	None
}

fn extract_title_from_link(link: &Element, url: &str) -> Option<String> {
	let text = normalize_text(&link.text().unwrap_or_default());
	if looks_like_series_title(&text) {
		return Some(text);
	}

	for attr_name in ["aria-label", "title"] {
		if let Some(value) = link.attr(attr_name) {
			let mut candidate = normalize_text(&value);
			if let Some(stripped) = candidate.strip_prefix("Abrir ") {
				candidate = String::from(stripped);
			}
			if looks_like_series_title(&candidate) {
				return Some(candidate);
			}
		}
	}

	if let Some(title) = extract_title_from_related_heading(link, url) {
		return Some(title);
	}

	if let Some(title) = find_series_title_in_image_subtree(link) {
		return Some(title);
	}

	manga_key_from_url(url).map(|value| normalize_text(&value))
}

fn extract_title_from_related_heading(link: &Element, _url: &str) -> Option<String> {
	let mut parent = link.parent();
	for _ in 0..4 {
		let Some(element) = parent else {
			break;
		};

		if let Some(title) = find_series_title_in_heading_subtree(&element) {
			return Some(title);
		}

		parent = element.parent();
	}

	None
}

fn extract_cover_from_link(link: &Element) -> Option<String> {
	if let Some(url) = extract_cover_from_element(link) {
		return Some(url);
	}

	let mut parent = link.parent();
	for _ in 0..4 {
		let Some(element) = parent else {
			break;
		};
		if let Some(url) = extract_cover_from_element(&element) {
			return Some(url);
		}
		parent = element.parent();
	}

	None
}

fn extract_cover_from_element(element: &Element) -> Option<String> {
	if element.tag_name().as_deref() == Some("img") {
		if let Some(url) = cover_image_url(element) {
			if is_likely_cover_url(&url) {
				return Some(url);
			}
		}
	}

	for child in element.children() {
		if let Some(url) = extract_cover_from_element(&child) {
			return Some(url);
		}
	}

	None
}

pub(crate) fn parse_entries_fallback(document: &Document) -> Vec<Manga> {
	parse_entries_from_selector(document, "article, .row.c-tabs-item__content")
}

pub(crate) fn parse_entries_from_selector(document: &Document, selector: &str) -> Vec<Manga> {
	let mut entries = Vec::new();
	if let Some(containers) = document.select(selector) {
		for container in containers {
			let Some(manga) = manga_from_container(&container) else {
				continue;
			};
			if entries.iter().any(|entry: &Manga| entry.key == manga.key) {
				continue;
			}
			entries.push(manga);
		}
	}
	entries
}

fn collect_manga_urls(element: &Element, urls: &mut Vec<String>) {
	if element.tag_name().as_deref() == Some("a") {
		if let Some(url) = attr_url(element, "href") {
			if is_manga_url(&url) && !urls.iter().any(|existing| existing == &url) {
				urls.push(url);
			}
		}
	}

	for child in element.children() {
		collect_manga_urls(&child, urls);
	}
}

pub(crate) fn manga_from_container(container: &Element) -> Option<Manga> {
	let (url, title) = extract_manga_url_and_title(container)?;
	let key = manga_key_from_url(&url)?;
	let cover = extract_cover_from_container(container);
	Some(Manga {
		key,
		title,
		cover,
		url: Some(url),
		..Default::default()
	})
}

pub(crate) fn extract_manga_url_and_title(container: &Element) -> Option<(String, String)> {
	let mut urls = Vec::new();
	collect_manga_urls(container, &mut urls);
	let url = urls.first()?.clone();
	let title = find_series_title_in_heading_subtree(container)
		.or_else(|| find_series_title_in_image_subtree(container))
		.or_else(|| manga_key_from_url(&url).map(|value| normalize_text(&value)))?;
	Some((url, title))
}

pub(crate) fn extract_cover_from_container(container: &Element) -> Option<String> {
	extract_cover_from_element(container)
}

fn extract_mtx_cover_url(document: &Document) -> Option<String> {
	let body = body_element(document)?;
	find_mtx_cover_url(&body)
}

fn find_mtx_cover_url(element: &Element) -> Option<String> {
	if let Some(value) = element.attr("data-mtx-cover") {
		let url = absolute_url(&value);
		if is_likely_cover_url(&url) {
			return Some(url);
		}
	}

	for child in element.children() {
		if let Some(url) = find_mtx_cover_url(&child) {
			return Some(url);
		}
	}

	None
}

pub(crate) fn parse_manga_title(document: &Document) -> Option<String> {
	document
		.select_first("h1")
		.and_then(|element| element.text())
		.map(|value| normalize_text(&value))
		.filter(|value| !value.is_empty())
		.or_else(|| {
			meta_content(
				document,
				&["meta[property='og:title']", "meta[name='twitter:title']"],
			)
		})
}

fn find_cover_image_matching(element: &Element, target_url: &str) -> Option<String> {
	if element.tag_name().as_deref() == Some("img") {
		if let Some(url) = image_url(element) {
			if url == target_url {
				if let Some(preferred) = cover_image_url(element) {
					if is_likely_cover_url(&preferred) {
						return Some(preferred);
					}
				}
				if is_likely_cover_url(&url) {
					return Some(url);
				}
			}
		}
	}

	for child in element.children() {
		if let Some(url) = find_cover_image_matching(&child, target_url) {
			return Some(url);
		}
	}

	None
}

fn find_first_cover_image_in_subtree(element: &Element) -> Option<String> {
	if element.tag_name().as_deref() == Some("img") {
		if let Some(url) = cover_image_url(element) {
			if is_likely_cover_url(&url) {
				return Some(url);
			}
		}
	}

	for child in element.children() {
		if let Some(url) = find_first_cover_image_in_subtree(&child) {
			return Some(url);
		}
	}

	None
}

pub(crate) fn parse_manga_cover(document: &Document) -> Option<String> {
	if let Some(url) = extract_mtx_cover_url(document) {
		return Some(url);
	}

	let meta_cover = meta_content(
		document,
		&[
			"meta[property='og:image']",
			"meta[property='twitter:image']",
			"meta[name='twitter:image']",
			"meta[name='twitter:image:src']",
		],
	)
	.map(|value| absolute_url(&value));

	if let Some(meta_cover) = meta_cover {
		if let Some(body) = body_element(document) {
			if let Some(preferred) = find_cover_image_matching(&body, &meta_cover) {
				return Some(preferred);
			}
		}
		if is_likely_cover_url(&meta_cover) {
			return Some(meta_cover);
		}
	}

	if let Some(body) = body_element(document) {
		if let Some(url) = find_first_cover_image_in_subtree(&body) {
			return Some(url);
		}
	}
	None
}

fn collect_description_paragraphs(element: &Element, output: &mut Vec<String>) {
	if element.tag_name().as_deref() == Some("p") {
		if let Some(text) = element.text() {
			let value = normalize_text(&text);
			if value.len() > 30 && !output.iter().any(|existing| existing == &value) {
				output.push(value);
			}
		}
	}

	for child in element.children() {
		collect_description_paragraphs(&child, output);
	}
}

fn description_score(text: &str) -> usize {
	let mut score = text.len();
	if text.contains('.') || text.contains('!') || text.contains('?') {
		score += 40;
	}
	if text.contains(',') {
		score += 20;
	}
	if text.split_whitespace().count() > 20 {
		score += 10;
	}
	score
}

fn best_description_from_element(element: &Element) -> Option<String> {
	let mut candidates = Vec::new();
	collect_description_paragraphs(element, &mut candidates);

	let mut best: Option<String> = None;
	let mut best_score = 0;
	for candidate in candidates {
		let score = description_score(&candidate);
		if best.is_none() || score > best_score {
			best_score = score;
			best = Some(candidate);
		}
	}

	best
}

fn find_synopsis_description(element: &Element) -> Option<String> {
	if element.attr("data-mtx-panel").as_deref() == Some("synopsis") {
		if let Some(description) = best_description_from_element(element) {
			return Some(description);
		}
		let text = normalize_text(&element.text().unwrap_or_default());
		if text.len() > 30 {
			return Some(text);
		}
	}

	for child in element.children() {
		if let Some(description) = find_synopsis_description(&child) {
			return Some(description);
		}
	}

	None
}

fn collect_genre_tags_from_element(element: &Element, tags: &mut Vec<String>) {
	if element.tag_name().as_deref() == Some("a") {
		if let Some(url) = attr_url(element, "href") {
			if url.contains("/genero/") {
				let text = normalize_text(&element.text().unwrap_or_default());
				if !text.is_empty() && !tags.iter().any(|existing| existing == &text) {
					tags.push(text);
				}
			}
		}
	}

	for child in element.children() {
		collect_genre_tags_from_element(&child, tags);
	}
}

fn count_genre_tags(element: &Element) -> usize {
	let mut count = 0;
	if element.tag_name().as_deref() == Some("a") {
		if let Some(url) = attr_url(element, "href") {
			if url.contains("/genero/") {
				count += 1;
			}
		}
	}

	for child in element.children() {
		count += count_genre_tags(&child);
	}

	count
}

fn collect_status_from_element(element: &Element) -> Option<MangaStatus> {
	let text = normalize_text(&element.text().unwrap_or_default());
	if !text.is_empty() && text.len() <= 80 {
		if let Some(status) = status_from_text(&text) {
			return Some(status);
		}
	}

	for child in element.children() {
		if let Some(status) = collect_status_from_element(&child) {
			return Some(status);
		}
	}

	None
}

pub(crate) fn parse_manga_description(document: &Document) -> Option<String> {
	if let Some(description) = meta_content(
		document,
		&[
			"meta[name='description']",
			"meta[property='og:description']",
			"meta[name='twitter:description']",
		],
	) {
		if description.len() > 30 {
			return Some(description);
		}
	}

	if let Some(body) = body_element(document) {
		if let Some(description) = find_synopsis_description(&body) {
			return Some(description);
		}
		if let Some(description) = best_description_from_element(&body) {
			return Some(description);
		}
	}
	None
}

fn find_genre_cluster(element: &Element, tags: &mut Vec<String>) -> bool {
	for child in element.children() {
		if find_genre_cluster(&child, tags) {
			return true;
		}
	}

	let tag_name = element.tag_name();
	if matches!(tag_name.as_deref(), Some("body") | Some("html")) {
		return false;
	}
	if count_genre_tags(element) >= 2 {
		collect_genre_tags_from_element(element, tags);
		return !tags.is_empty();
	}

	false
}

pub(crate) fn parse_manga_tags(document: &Document) -> Vec<String> {
	let mut tags = Vec::new();
	if let Some(body) = body_element(document) {
		if !find_genre_cluster(&body, &mut tags) {
			collect_genre_tags_from_element(&body, &mut tags);
		}
	}
	tags
}

pub(crate) fn parse_text_values(document: &Document, selector: &str) -> Vec<String> {
	let mut values = Vec::new();
	push_unique_text_values(document, selector, &mut values);
	values
}

pub(crate) fn push_unique_text_values(
	document: &Document,
	selector: &str,
	output: &mut Vec<String>,
) {
	if let Some(elements) = document.select(selector) {
		for element in elements {
			let Some(text) = element.text() else {
				continue;
			};
			let value = normalize_text(&text);
			if value.is_empty() {
				continue;
			}
			if output.iter().any(|existing| existing == &value) {
				continue;
			}
			output.push(value);
		}
	}
}

pub(crate) fn parse_manga_status(document: &Document) -> MangaStatus {
	if let Some(body) = body_element(document) {
		if let Some(status) = collect_status_from_element(&body) {
			return status;
		}
		if let Some(status) = status_from_text(&body.text().unwrap_or_default()) {
			return status;
		}
	}
	MangaStatus::Unknown
}

pub(crate) fn status_from_text(text: &str) -> Option<MangaStatus> {
	let lower = normalize_text(text).to_lowercase();
	if lower.is_empty() {
		return None;
	}
	if lower.contains("progresso") || lower.contains("ongoing") {
		return Some(MangaStatus::Ongoing);
	}
	if lower.contains("completo") || lower.contains("conclu") || lower.contains("finalizado") {
		return Some(MangaStatus::Completed);
	}
	if lower.contains("hiato") || lower.contains("pausa") {
		return Some(MangaStatus::Hiatus);
	}
	if lower.contains("cancel") {
		return Some(MangaStatus::Cancelled);
	}
	None
}

#[derive(Deserialize)]
pub(crate) struct MtxChapterJsonEntry {
	url: Option<String>,
	title: Option<String>,
	meta: Option<String>,
	search: Option<String>,
}

pub(crate) fn parse_manga_chapters(document: &Document, manga_key: Option<&str>) -> Vec<Chapter> {
	let mut chapters = parse_manga_chapters_from_json(document, manga_key);
	if chapters.is_empty() {
		chapters = parse_manga_chapters_from_links(document, manga_key);
		println!(
			"[montetai] chapters_parse source=links count={}",
			chapters.len()
		);
	} else {
		println!(
			"[montetai] chapters_parse source=json count={}",
			chapters.len()
		);
	}
	chapters
}

pub(crate) fn parse_manga_chapters_from_json(
	document: &Document,
	manga_key: Option<&str>,
) -> Vec<Chapter> {
	let Some(rows) = read_mtx_chapter_json_entries(document) else {
		return Vec::new();
	};
	if rows.is_empty() {
		return Vec::new();
	}
	println!("[montetai] chapters_json_rows={}", rows.len());

	let manga_key = manga_key.filter(|value| !value.is_empty());
	let fallback_date = current_date();
	let mut chapters = Vec::new();

	for row in rows {
		let Some(raw_url) = row.url else {
			continue;
		};
		let url = absolute_url(&raw_url);
		if !is_chapter_url(&url) {
			continue;
		}
		if let Some(expected_key) = manga_key {
			if manga_key_from_url(&url).as_deref() != Some(expected_key) {
				continue;
			}
		}

		let Some(key) = chapter_key_from_url(&url) else {
			continue;
		};
		if chapters.iter().any(|chapter: &Chapter| chapter.key == key) {
			continue;
		}

		let mut title = row
			.title
			.as_ref()
			.map(|value| normalize_text(value))
			.unwrap_or_default();
		if title.is_empty() {
			title = chapter_title_from_text(&url);
		}
		if title.is_empty() {
			continue;
		}

		let meta = row
			.meta
			.as_ref()
			.map(|value| normalize_text(value))
			.unwrap_or_default();
		let date_uploaded = Some(chapter_date_from_text(&meta, fallback_date));
		let chapter_number = parse_chapter_number(&title, &url).or_else(|| {
			row.search
				.as_ref()
				.and_then(|value| parse_chapter_number(value, &url))
		});

		chapters.push(Chapter {
			key,
			title: Some(title),
			chapter_number,
			date_uploaded,
			url: Some(url),
			..Default::default()
		});
	}

	println!("[montetai] chapters_json_parsed={}", chapters.len());
	chapters
}

fn read_mtx_chapter_json_entries_from_element(
	element: &Element,
) -> Option<Vec<MtxChapterJsonEntry>> {
	if element.tag_name().as_deref() == Some("script") {
		let raw_json = element
			.untrimmed_text()
			.or_else(|| element.html())
			.or_else(|| element.text())
			.unwrap_or_default();
		if !raw_json.is_empty() {
			let normalized = raw_json
				.replace("&quot;", "\"")
				.replace("&#34;", "\"")
				.replace("&amp;", "&")
				.replace("&#039;", "'")
				.replace("&apos;", "'");

			if let Ok(entries) = serde_json::from_str::<Vec<MtxChapterJsonEntry>>(&normalized) {
				println!(
					"[montetai] mtx_json parse=normalized count={}",
					entries.len()
				);
				if !entries.is_empty() {
					return Some(entries);
				}
			}
			if let Ok(entries) = serde_json::from_str::<Vec<MtxChapterJsonEntry>>(&raw_json) {
				println!("[montetai] mtx_json parse=raw count={}", entries.len());
				if !entries.is_empty() {
					return Some(entries);
				}
			}
		}
	}

	for child in element.children() {
		if let Some(entries) = read_mtx_chapter_json_entries_from_element(&child) {
			return Some(entries);
		}
	}

	None
}

pub(crate) fn read_mtx_chapter_json_entries(
	document: &Document,
) -> Option<Vec<MtxChapterJsonEntry>> {
	let body = body_element(document)?;
	read_mtx_chapter_json_entries_from_element(&body)
}

fn collect_chapter_links(
	element: &Element,
	manga_key: Option<&str>,
	fallback_date: i64,
	chapters: &mut Vec<Chapter>,
) {
	if element.tag_name().as_deref() == Some("a") {
		if let Some(url) = attr_url(element, "href") {
			if is_chapter_url(&url) {
				if let Some(expected_key) = manga_key {
					if manga_key_from_url(&url).as_deref() != Some(expected_key) {
						return;
					}
				}

				let link_text = normalize_text(&element.text().unwrap_or_default());
				let mut title = chapter_title_from_text(&link_text);
				if title.is_empty() {
					for attr_name in ["aria-label", "title"] {
						if let Some(value) = element.attr(attr_name) {
							let candidate = normalize_text(&value);
							title = chapter_title_from_text(&candidate);
							if title.is_empty() && candidate.to_lowercase().contains("capitulo") {
								title = candidate;
							}
							if !title.is_empty() {
								break;
							}
						}
					}
				}
				if title.is_empty() {
					title = chapter_title_from_text(&url);
				}
				if title.is_empty() {
					return;
				}

				let Some(key) = chapter_key_from_url(&url) else {
					return;
				};
				if chapters.iter().any(|chapter: &Chapter| chapter.key == key) {
					return;
				}

				let date_uploaded = Some(chapter_date_from_text(&link_text, fallback_date));
				let chapter_number = parse_chapter_number(&title, &url)
					.or_else(|| parse_chapter_number(&link_text, &url));

				chapters.push(Chapter {
					key,
					title: Some(title),
					chapter_number,
					date_uploaded,
					url: Some(url),
					..Default::default()
				});
			}
		}
	}

	for child in element.children() {
		collect_chapter_links(&child, manga_key, fallback_date, chapters);
	}
}

pub(crate) fn parse_manga_chapters_from_links(
	document: &Document,
	manga_key: Option<&str>,
) -> Vec<Chapter> {
	let fallback_date = current_date();
	let mut chapters = Vec::new();
	if let Some(body) = body_element(document) {
		collect_chapter_links(&body, manga_key, fallback_date, &mut chapters);
	}
	println!("[montetai] chapters_links parsed_count={}", chapters.len());
	chapters
}

fn collect_chapter_page_urls(element: &Element, urls: &mut Vec<String>) {
	if element.tag_name().as_deref() == Some("img") {
		if let Some(url) = image_url(element) {
			if is_chapter_image(element, &url) && !urls.iter().any(|existing| existing == &url) {
				urls.push(url);
			}
		}
	}

	for child in element.children() {
		collect_chapter_page_urls(&child, urls);
	}
}

pub(crate) fn parse_chapter_page_urls(document: &Document) -> Vec<String> {
	let mut urls = Vec::new();
	if let Some(body) = body_element(document) {
		collect_chapter_page_urls(&body, &mut urls);
	}
	println!("[montetai] page_urls final_count={}", urls.len());
	urls
}

fn container_has_pagination_signals(element: &Element) -> bool {
	if element.attr("aria-current").as_deref() == Some("page") {
		return true;
	}

	if element.tag_name().as_deref() == Some("a") || element.tag_name().as_deref() == Some("span") {
		let value = normalize_text(&element.text().unwrap_or_default());
		if !value.is_empty() && value.chars().all(|ch| ch.is_ascii_digit()) {
			return true;
		}
	}

	for child in element.children() {
		if container_has_pagination_signals(&child) {
			return true;
		}
	}

	false
}

fn has_next_page_in_element(element: &Element) -> bool {
	if element.tag_name().as_deref() == Some("a") {
		let value = normalize_text(&element.text().unwrap_or_default()).to_lowercase();
		if value == "proximo" || value == "próximo" || value == "next" {
			let mut parent = element.parent();
			for _ in 0..4 {
				let Some(container) = parent else {
					break;
				};
				if container_has_pagination_signals(&container) {
					return true;
				}
				parent = container.parent();
			}
		}
	}

	for child in element.children() {
		if has_next_page_in_element(&child) {
			return true;
		}
	}

	false
}

pub(crate) fn has_next_page(document: &Document) -> bool {
	if let Some(body) = body_element(document) {
		return has_next_page_in_element(&body);
	}
	false
}
