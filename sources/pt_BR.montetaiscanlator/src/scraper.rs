use aidoku::{
	Chapter, Manga, MangaStatus,
	alloc::{String, Vec},
	imports::html::{Document, Element},
	prelude::*,
};
use serde::Deserialize;

use crate::{
	BASE_URL, absolute_url, attr_url, chapter_key_from_url, chapter_title_from_text,
	cover_image_url, extract_date_token, image_url, is_chapter_image, is_chapter_url,
	is_likely_cover_url, is_manga_url, looks_like_series_title, manga_key_from_url, meta_content,
	normalize_text, parse_chapter_number, parse_pt_br_date, percent_encode,
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

fn parse_entries_from_urls(document: &Document) -> Vec<Manga> {
	let mut entries = Vec::new();
	if let Some(links) = document.select("a[href*='/manga/']") {
		for link in links {
			let Some(manga) = manga_from_link(&link) else {
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

fn manga_from_link(link: &Element) -> Option<Manga> {
	let url = attr_url(link, "href")?;
	if !is_manga_url(&url) {
		return None;
	}

	let key = manga_key_from_url(&url)?;
	let title = extract_title_from_link(link, &url)?;
	let cover = extract_cover_from_link(link);
	Some(Manga {
		key,
		title,
		cover,
		url: Some(url),
		..Default::default()
	})
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

	if let Some(images) = link.select("img") {
		for image in images {
			if let Some(alt_text) = image.attr("alt") {
				let candidate = normalize_text(&alt_text);
				if looks_like_series_title(&candidate) {
					return Some(candidate);
				}
			}
		}
	}

	manga_key_from_url(url).map(|value| normalize_text(&value))
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
	if let Some(images) = element.select("img") {
		for image in images {
			let Some(url) = image_url(&image) else {
				continue;
			};
			if is_likely_cover_url(&url) {
				return Some(url);
			}
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
	if let Some(links) = container.select(
		"h1 a[href*='/manga/'], h2 a[href*='/manga/'], h3 a[href*='/manga/'], h4 a[href*='/manga/']",
	) {
		for link in links {
			let Some(url) = attr_url(&link, "href") else {
				continue;
			};
			if !is_manga_url(&url) {
				continue;
			}
			let title = normalize_text(&link.text().unwrap_or_default());
			if looks_like_series_title(&title) {
				return Some((url, title));
			}
		}
	}

	let mut fallback_url: Option<String> = None;
	let mut fallback_title: Option<String> = None;

	if let Some(links) = container.select("a[href*='/manga/']") {
		for link in links {
			let Some(url) = attr_url(&link, "href") else {
				continue;
			};
			if !is_manga_url(&url) {
				continue;
			}
			if fallback_url.is_none() {
				fallback_url = Some(url.clone());
			}

			let text = normalize_text(&link.text().unwrap_or_default());
			if looks_like_series_title(&text) {
				return Some((url, text));
			}

			if fallback_title.is_none() {
				if let Some(aria_label) = link.attr("aria-label") {
					let mut candidate = normalize_text(&aria_label);
					if let Some(stripped) = candidate.strip_prefix("Abrir ") {
						candidate = String::from(stripped);
					}
					if looks_like_series_title(&candidate) {
						fallback_title = Some(candidate);
					}
				}
			}
			if fallback_title.is_none() {
				if let Some(title_attr) = link.attr("title") {
					let candidate = normalize_text(&title_attr);
					if looks_like_series_title(&candidate) {
						fallback_title = Some(candidate);
					}
				}
			}
		}
	}

	if fallback_title.is_none() {
		if let Some(images) = container.select("img") {
			for image in images {
				if let Some(alt_text) = image.attr("alt") {
					let candidate = normalize_text(&alt_text);
					if looks_like_series_title(&candidate) {
						fallback_title = Some(candidate);
						break;
					}
				}
			}
		}
	}

	let url = fallback_url?;
	let title =
		fallback_title.or_else(|| manga_key_from_url(&url).map(|value| normalize_text(&value)))?;
	Some((url, title))
}

pub(crate) fn extract_cover_from_container(container: &Element) -> Option<String> {
	if let Some(images) = container.select("img") {
		for image in images {
			let Some(url) = image_url(&image) else {
				continue;
			};
			if is_likely_cover_url(&url) {
				return Some(url);
			}
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

pub(crate) fn parse_manga_cover(document: &Document) -> Option<String> {
	let selectors = [
		".mtx-cover img",
		".summary_image img",
		".profile-manga img.img-responsive",
		".tab-summary img.img-responsive",
	];
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
		for selector in selectors {
			if let Some(image) = document.select_first(selector) {
				if let Some(url) = image_url(&image) {
					if url == meta_cover {
						if let Some(preferred) = cover_image_url(&image) {
							if is_likely_cover_url(&preferred) {
								return Some(preferred);
							}
						}
					}
				}
			}
		}
		if is_likely_cover_url(&meta_cover) {
			return Some(meta_cover);
		}
	}

	for selector in selectors {
		if let Some(image) = document.select_first(selector) {
			if let Some(url) = cover_image_url(&image) {
				if is_likely_cover_url(&url) {
					return Some(url);
				}
			}
		}
	}
	if let Some(images) = document.select("img") {
		for image in images {
			if let Some(url) = cover_image_url(&image) {
				if is_likely_cover_url(&url) {
					return Some(url);
				}
			}
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

	let selectors = [
		".mtx-synopsis",
		".description-summary .summary__content",
		".summary__content",
		".post-content_item.mg_summary .summary-content",
	];
	for selector in selectors {
		if let Some(element) = document.select_first(selector) {
			if let Some(text) = element.text() {
				let normalized = normalize_text(&text);
				if normalized.len() > 30 {
					return Some(normalized);
				}
			}
		}
	}
	None
}

pub(crate) fn parse_manga_tags(document: &Document) -> Vec<String> {
	let mut tags = Vec::new();
	push_unique_text_values(document, "a[href*='/genero/']", &mut tags);
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
	let selectors = [
		".mtx-pill-status",
		".post-content_item.mg_status .summary-content",
		".summary_content_wrap .post-status .summary-content",
	];
	for selector in selectors {
		if let Some(element) = document.select_first(selector) {
			if let Some(text) = element.text() {
				if let Some(status) = status_from_text(&text) {
					return status;
				}
			}
		}
	}
	if let Some(body) = document.select_first("body") {
		if let Some(text) = body.text() {
			if let Some(status) = status_from_text(&text) {
				return status;
			}
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

	let key_marker = manga_key
		.filter(|value| !value.is_empty())
		.map(|value| format!("/manga/{value}/"));
	let mut chapters = Vec::new();

	for row in rows {
		let Some(raw_url) = row.url else {
			continue;
		};
		let url = absolute_url(&raw_url);
		if !is_chapter_url(&url) {
			continue;
		}
		if let Some(marker) = key_marker.as_ref() {
			if !url.contains(marker) {
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
		let date_uploaded = extract_date_token(&meta).and_then(|value| parse_pt_br_date(&value));
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

pub(crate) fn read_mtx_chapter_json_entries(
	document: &Document,
) -> Option<Vec<MtxChapterJsonEntry>> {
	let selectors = [
		"script.mtx-chapter-data[type='application/json']",
		".mtx-reading-zone script.mtx-chapter-data",
		"script.mtx-chapter-data",
	];

	for selector in selectors {
		let Some(script) = document.select_first(selector) else {
			println!("[montetai] mtx_json selector={} found=false", selector);
			continue;
		};
		let raw_json = script
			.untrimmed_text()
			.or_else(|| script.html())
			.or_else(|| script.text())
			.unwrap_or_default();
		println!(
			"[montetai] mtx_json selector={} found=true raw_len={}",
			selector,
			raw_json.len()
		);
		if raw_json.is_empty() {
			continue;
		}

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
		println!("[montetai] mtx_json parse_failed selector={}", selector);
	}

	None
}

pub(crate) fn parse_manga_chapters_from_links(
	document: &Document,
	manga_key: Option<&str>,
) -> Vec<Chapter> {
	let mut chapters = Vec::new();
	let key_marker = manga_key
		.filter(|value| !value.is_empty())
		.map(|value| format!("/manga/{value}/"));

	if let Some(links) = document.select("a[href*='/capitulo-']") {
		let total_links = links.size();
		println!("[montetai] chapters_links anchors_found={}", total_links);
		for link in links {
			let Some(url) = attr_url(&link, "href") else {
				continue;
			};
			if !is_chapter_url(&url) {
				continue;
			}
			if let Some(marker) = key_marker.as_ref() {
				if !url.contains(marker) {
					continue;
				}
			}

			let class_name = link.class_name().unwrap_or_default().to_lowercase();
			let link_text = normalize_text(&link.text().unwrap_or_default());
			let link_lower = link_text.to_lowercase();

			let mut title = link
				.select_first(".mtx-chapter-title, .mt-manga-catalog-card__chapter-label")
				.and_then(|element| element.text())
				.map(|value| normalize_text(&value))
				.unwrap_or_default();
			if title.is_empty() {
				title = chapter_title_from_text(&link_text);
			}

			if title.is_empty() {
				continue;
			}
			if !title.to_lowercase().contains("capitulo")
				&& !link_lower.contains("capitulo")
				&& !class_name.contains("chapter")
			{
				continue;
			}

			let Some(key) = chapter_key_from_url(&url) else {
				continue;
			};
			if chapters.iter().any(|chapter: &Chapter| chapter.key == key) {
				continue;
			}

			let date_uploaded = link
				.select_first(".mtx-chapter-meta, .mt-manga-catalog-card__chapter-side span, .chapter-release-date, .font-meta")
				.and_then(|element| element.text())
				.map(|value| normalize_text(&value))
				.and_then(|value| extract_date_token(&value))
				.and_then(|value| parse_pt_br_date(&value))
				.or_else(|| extract_date_token(&link_text).and_then(|value| parse_pt_br_date(&value)));

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

	println!("[montetai] chapters_links parsed_count={}", chapters.len());
	chapters
}

pub(crate) fn parse_chapter_page_urls(document: &Document) -> Vec<String> {
	let selectors = [
		".reading-content img",
		"img.wp-manga-chapter-img",
		".page-break img",
	];
	let mut urls = Vec::new();

	for selector in selectors {
		if let Some(images) = document.select(selector) {
			let candidates = images.size();
			println!(
				"[montetai] page_urls selector={} candidates={}",
				selector, candidates
			);
			for image in images {
				let Some(url) = image_url(&image) else {
					continue;
				};
				if !is_chapter_image(&image, &url) {
					continue;
				}
				if urls.iter().any(|existing| existing == &url) {
					continue;
				}
				urls.push(url);
			}
		}
	}

	println!("[montetai] page_urls final_count={}", urls.len());
	urls
}

pub(crate) fn has_next_page(document: &Document) -> bool {
	if document
		.select_first("a.next.page-numbers, a.page-numbers.next, .wp-pagenavi a.next, .mt-home-lab-catalog__pagination a.next, .mt-manga-catalog-lab__pagination a.next")
		.is_some()
	{
		return true;
	}
	if let Some(links) = document.select("a") {
		for link in links {
			let value = normalize_text(&link.text().unwrap_or_default()).to_lowercase();
			if value == "proximo" || value == "próximo" || value == "next" {
				return true;
			}
		}
	}
	false
}
