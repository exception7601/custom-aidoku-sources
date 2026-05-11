use aidoku::{
	Chapter, Manga,
	alloc::{String, Vec},
	imports::{
		html::{Document, Element},
		std::parse_local_date,
	},
	prelude::*,
};

use crate::BASE_URL;

fn url_path_segments(url: &str) -> Vec<String> {
	let cleaned = sanitize_url(url);
	let mut path = cleaned.as_str();
	if let Some((_, rest)) = path.split_once("://") {
		path = rest;
		if let Some((_, rest)) = path.split_once('/') {
			path = rest;
		} else {
			path = "";
		}
	} else if path.starts_with("//") {
		let stripped = &path[2..];
		if let Some((_, rest)) = stripped.split_once('/') {
			path = rest;
		} else {
			path = "";
		}
	} else if path.starts_with('/') {
		path = path.trim_start_matches('/');
	}
	let path = path.split('?').next().unwrap_or(path);
	let path = path.split('#').next().unwrap_or(path);
	path.split('/')
		.filter(|segment| !segment.is_empty())
		.map(String::from)
		.collect()
}

fn path_segment_is_noise(segment: &str) -> bool {
	let lower = normalize_text(segment).to_lowercase();
	if lower.is_empty() {
		return true;
	}
	if lower.chars().all(|ch| ch.is_ascii_digit()) {
		return true;
	}
	matches!(
		lower.as_str(),
		"page"
			| "perfil"
			| "login" | "register"
			| "cadastro"
			| "bookmarks"
			| "favoritos"
			| "historico"
			| "histórico"
			| "history"
			| "search"
			| "buscar"
			| "tag" | "tags"
			| "categoria"
			| "categorias"
			| "author"
			| "autores"
			| "feed" | "manga"
			| "post" | "home"
			| "index" | "inicio"
			| "privacy"
			| "privacy-policy"
			| "privacidade"
			| "politica"
			| "politica-de-privacidade"
			| "dmca" | "terms"
			| "termos"
			| "about" | "sobre"
			| "contact"
			| "contato"
			| "cookies"
			| "cookie"
	) || lower.starts_with("wp-")
		|| lower.starts_with("feed/")
		|| lower.starts_with("wp-json")
}

fn is_supported_site_url(url: &str) -> bool {
	let cleaned = sanitize_url(url);
	if cleaned.starts_with('/') || cleaned.starts_with("manga/") {
		return true;
	}

	let Some(rest) = cleaned
		.strip_prefix("https://")
		.or_else(|| cleaned.strip_prefix("http://"))
		.or_else(|| cleaned.strip_prefix("//"))
	else {
		return false;
	};
	let host = rest.split('/').next().unwrap_or_default().to_lowercase();
	host == "montetaiscanlator.xyz" || host == "www.montetaiscanlator.xyz"
}

fn path_segment_looks_like_slug(segment: &str) -> bool {
	let lower = normalize_text(segment).to_lowercase();
	if path_segment_is_noise(&lower) || lower.starts_with("capitulo-") || lower.contains('.') {
		return false;
	}
	let alpha_count = lower.chars().filter(|ch| ch.is_ascii_alphabetic()).count();
	alpha_count >= 4 && (lower.contains('-') || lower.contains('_') || lower.len() >= 4)
}

pub(crate) fn manga_url(manga: &Manga) -> String {
	if let Some(url) = manga.url.as_ref() {
		if is_manga_url(url) || is_chapter_url(url) {
			return absolute_url(url);
		}
		if url.starts_with("http://") || url.starts_with("https://") || url.starts_with('/') {
			return absolute_url(url);
		}
	}

	let key = normalize_text(&manga.key);
	if key.starts_with("http://") || key.starts_with("https://") {
		return absolute_url(&key);
	}
	if key.starts_with("manga/") || key.starts_with("/manga/") {
		return absolute_url(&key);
	}
	if !key.is_empty() {
		return format!("{BASE_URL}/manga/{}/", key.trim_matches('/'));
	}
	String::from(BASE_URL)
}

pub(crate) fn chapter_url(manga: &Manga, chapter: &Chapter) -> String {
	if let Some(url) = chapter.url.as_ref() {
		if is_chapter_url(url) || url.starts_with("http://") || url.starts_with("https://") {
			return absolute_url(url);
		}
		if url.starts_with('/') {
			return absolute_url(url);
		}
	}

	let key = normalize_text(&chapter.key);
	if key.starts_with("http://") || key.starts_with("https://") {
		return absolute_url(&key);
	}
	if key.starts_with("manga/") || key.starts_with("/manga/") {
		return absolute_url(&key);
	}
	if key.starts_with("capitulo-") {
		let manga_base = String::from(manga_url(manga).trim_end_matches('/'));
		return format!("{manga_base}/{key}/");
	}

	let manga_base = String::from(manga_url(manga).trim_end_matches('/'));
	format!("{manga_base}/{}/", key.trim_matches('/'))
}

pub(crate) fn normalize_text(value: &str) -> String {
	let mut normalized = String::new();
	for part in value.split_whitespace() {
		if !normalized.is_empty() {
			normalized.push(' ');
		}
		normalized.push_str(part);
	}
	normalized
}

pub(crate) fn sanitize_url(url: &str) -> String {
	normalize_text(url)
		.replace("&amp;", "&")
		.replace("&#038;", "&")
		.replace("#038;", "&")
}

pub(crate) fn absolute_url(url: &str) -> String {
	let cleaned = sanitize_url(url);
	if cleaned.starts_with("http://") || cleaned.starts_with("https://") {
		return cleaned;
	}
	if cleaned.starts_with("//") {
		return format!("https:{cleaned}");
	}
	if cleaned.starts_with('/') {
		return format!("{BASE_URL}{cleaned}");
	}
	format!("{BASE_URL}/{}", cleaned.trim_start_matches('/'))
}

pub(crate) fn attr_url(element: &Element, attr_name: &str) -> Option<String> {
	element
		.attr(format!("abs:{attr_name}"))
		.or_else(|| element.attr(attr_name))
		.map(|value| absolute_url(&value))
}

pub(crate) fn image_url(image: &Element) -> Option<String> {
	image
		.attr("src")
		.or_else(|| image.attr("data-src"))
		.or_else(|| image.attr("data-lazy-src"))
		.or_else(|| image.attr("data-original"))
		.or_else(|| image.attr("abs:src"))
		.map(|value| absolute_url(&value))
}

pub(crate) fn meta_content(document: &Document, selectors: &[&str]) -> Option<String> {
	for selector in selectors {
		let Some(element) = document.select_first(selector) else {
			continue;
		};
		let Some(value) = element
			.attr("content")
			.or_else(|| element.attr("value"))
			.or_else(|| element.text())
		else {
			continue;
		};
		let normalized = normalize_text(&value);
		if !normalized.is_empty() {
			return Some(normalized);
		}
	}
	None
}

pub(crate) fn image_srcset_url(image: &Element) -> Option<String> {
	let srcset = image.attr("srcset").or_else(|| image.attr("data-srcset"))?;
	let mut best_width = u32::MAX;
	let mut best_url: Option<String> = None;

	for candidate in srcset.split(',') {
		let entry = normalize_text(candidate);
		if entry.is_empty() {
			continue;
		}
		let mut parts = entry.split_whitespace();
		let Some(url) = parts.next() else {
			continue;
		};
		let descriptor = parts.next().unwrap_or_default();
		let width = descriptor
			.strip_suffix('w')
			.and_then(|value| value.parse::<u32>().ok())
			.unwrap_or(u32::MAX);
		if best_url.is_none() || width < best_width {
			best_width = width;
			best_url = Some(absolute_url(url));
		}
	}

	best_url
}

pub(crate) fn cover_image_url(image: &Element) -> Option<String> {
	image_srcset_url(image).or_else(|| image_url(image))
}

pub(crate) fn is_likely_cover_url(url: &str) -> bool {
	let lower = sanitize_url(url).to_lowercase();
	if lower.is_empty() || !lower.starts_with("http") {
		return false;
	}
	if lower.contains("logo-monte-tai")
		|| lower.contains("favicon")
		|| lower.contains("/reactions/")
		|| lower.contains("graphstyle-comments")
	{
		return false;
	}
	if lower.contains("/wp-content/uploads/") {
		return true;
	}
	lower.contains(".png")
		|| lower.contains(".jpg")
		|| lower.contains(".jpeg")
		|| lower.contains(".webp")
		|| lower.contains(".gif")
}

pub(crate) fn is_chapter_image(image: &Element, url: &str) -> bool {
	let lower = sanitize_url(url).to_lowercase();
	if lower.is_empty() || !lower.starts_with("http") {
		return false;
	}
	if lower.contains("logo-monte-tai")
		|| lower.contains("favicon")
		|| lower.contains("/reactions/")
		|| lower.contains("/graphstyle-comments")
	{
		return false;
	}
	if is_chapter_url(&lower) {
		return false;
	}
	if lower.contains("mt_madara_s3_image") {
		return true;
	}

	let looks_like_image = has_image_extension(&lower)
		|| lower.contains("/wp-content/uploads/wp-manga/data/")
		|| lower.contains("/wp-content/uploads/");
	if !looks_like_image {
		return false;
	}

	let class_name = image.class_name().unwrap_or_default().to_lowercase();
	if class_name.contains("wp-manga-chapter-img") {
		return true;
	}

	let parent_class = image
		.parent()
		.and_then(|parent| parent.class_name())
		.unwrap_or_default()
		.to_lowercase();
	parent_class.contains("page-break") || parent_class.contains("reading-content")
}

pub(crate) fn has_image_extension(url: &str) -> bool {
	let no_query = url.split('?').next().unwrap_or(url);
	let no_fragment = no_query.split('#').next().unwrap_or(no_query);
	no_fragment.ends_with(".jpg")
		|| no_fragment.ends_with(".jpeg")
		|| no_fragment.ends_with(".png")
		|| no_fragment.ends_with(".webp")
		|| no_fragment.ends_with(".gif")
		|| no_fragment.ends_with(".avif")
}

pub(crate) fn is_manga_url(url: &str) -> bool {
	manga_key_from_url(url).is_some() && !is_chapter_url(url)
}

pub(crate) fn is_chapter_url(url: &str) -> bool {
	chapter_key_from_url(url).is_some()
}

pub(crate) fn looks_like_series_title(text: &str) -> bool {
	let normalized = normalize_text(text);
	if normalized.is_empty() {
		return false;
	}
	let lower = normalized.to_lowercase();
	if lower.starts_with('#') {
		return false;
	}
	if lower == "manhwa"
		|| lower == "manhua"
		|| lower == "manga"
		|| lower == "novo"
		|| lower == "ler agora"
		|| lower == "biblioteca"
		|| lower == "anterior"
		|| lower == "proximo"
		|| lower == "próximo"
	{
		return false;
	}
	true
}

pub(crate) fn manga_key_from_url(url: &str) -> Option<String> {
	if !is_supported_site_url(url) {
		return None;
	}
	let segments = url_path_segments(url);
	if segments.is_empty() {
		return None;
	}

	if let Some(index) = segments.iter().position(|segment| {
		normalize_text(segment)
			.to_lowercase()
			.starts_with("capitulo-")
	}) {
		if index == 0 {
			return None;
		}
		let candidate = normalize_text(&segments[index - 1]);
		if path_segment_looks_like_slug(&candidate) {
			return Some(candidate);
		}
		return None;
	}

	let Some(last) = segments.last() else {
		return None;
	};
	let candidate = normalize_text(last);
	if path_segment_looks_like_slug(&candidate) {
		Some(candidate)
	} else {
		None
	}
}

pub(crate) fn chapter_key_from_url(url: &str) -> Option<String> {
	if !is_supported_site_url(url) {
		return None;
	}
	let segments = url_path_segments(url);
	let index = segments.iter().position(|segment| {
		normalize_text(segment)
			.to_lowercase()
			.starts_with("capitulo-")
	})?;
	if index == 0 {
		return None;
	}
	Some(segments[..=index].join("/"))
}

pub(crate) fn chapter_title_from_text(text: &str) -> String {
	let normalized = normalize_text(text);
	let lower = normalized.to_lowercase();
	let Some(index) = lower.find("capitulo") else {
		return String::new();
	};
	let suffix = &normalized[index..];
	let mut words = suffix.split_whitespace();
	let first = words.next().unwrap_or_default();
	let second = words.next().unwrap_or_default();
	if first.is_empty() || second.is_empty() {
		return String::new();
	}
	format!("{first} {second}")
}

pub(crate) fn parse_chapter_number(text: &str, url: &str) -> Option<f32> {
	extract_number_after_capitulo(text)
		.and_then(|value| parse_number_token(&value))
		.or_else(|| extract_number_after_capitulo(url).and_then(|value| parse_number_token(&value)))
}

pub(crate) fn extract_number_after_capitulo(text: &str) -> Option<String> {
	let lower = text.to_lowercase();
	let marker = if lower.contains("capitulo-") {
		"capitulo-"
	} else if lower.contains("capitulo") {
		"capitulo"
	} else {
		return None;
	};
	let start = lower.find(marker)? + marker.len();
	let suffix = &text[start..];
	let mut number = String::new();
	let mut started = false;

	for ch in suffix.chars() {
		if ch.is_ascii_digit() {
			number.push(ch);
			started = true;
			continue;
		}
		if started && (ch == '.' || ch == ',' || ch == '-') {
			number.push('.');
			continue;
		}
		if started {
			break;
		}
	}

	if number.is_empty() {
		None
	} else {
		Some(number)
	}
}

pub(crate) fn parse_number_token(token: &str) -> Option<f32> {
	let mut normalized = String::new();
	let mut last_was_dot = false;
	for ch in token.chars() {
		if ch.is_ascii_digit() {
			normalized.push(ch);
			last_was_dot = false;
			continue;
		}
		if ch == '.' && !last_was_dot {
			normalized.push('.');
			last_was_dot = true;
		}
	}
	while normalized.ends_with('.') {
		normalized.pop();
	}
	if normalized.is_empty() {
		None
	} else {
		normalized.parse::<f32>().ok()
	}
}

pub(crate) fn extract_date_token(text: &str) -> Option<String> {
	for raw_token in text.split_whitespace() {
		let token = raw_token
			.trim_matches(|ch: char| ch == ',' || ch == ';' || ch == '.')
			.chars()
			.filter(|ch| ch.is_ascii_digit() || *ch == '/')
			.collect::<String>();
		if token.matches('/').count() == 2 && token.len() >= 8 {
			return Some(token);
		}
	}
	None
}

pub(crate) fn parse_pt_br_date(text: &str) -> Option<i64> {
	let value = normalize_text(text);
	if value.is_empty() {
		return None;
	}
	let value = format!("{value} 00:00");
	parse_local_date(&value, "dd/MM/yyyy HH:mm")
		.or_else(|| parse_local_date(&value, "d/M/yyyy HH:mm"))
}

pub(crate) fn chapter_date_from_text(text: &str, fallback_date: i64) -> i64 {
	let value = normalize_text(text);
	if let Some(token) = extract_date_token(&value) {
		if let Some(timestamp) = parse_pt_br_date(&token) {
			return timestamp;
		}
	}
	if let Some(timestamp) = parse_relative_chapter_date(&value, fallback_date) {
		return timestamp;
	}
	fallback_date
}

fn parse_relative_chapter_date(text: &str, fallback_date: i64) -> Option<i64> {
	let lower = normalize_text(text).to_lowercase();
	if lower.is_empty() {
		return None;
	}

	let mut previous_token: Option<&str> = None;
	for token in lower.split_whitespace() {
		if let Some(unit_seconds) = relative_unit_seconds(token) {
			let count = previous_token.and_then(parse_relative_count)?;
			let offset = count.checked_mul(unit_seconds)?;
			let timestamp = fallback_date.checked_sub(offset)?;
			return Some(timestamp);
		}
		previous_token = Some(token);
	}

	None
}

fn relative_unit_seconds(token: &str) -> Option<i64> {
	if token.starts_with("minut") {
		return Some(60);
	}
	if token.starts_with("hor") {
		return Some(60 * 60);
	}
	if token.starts_with("dia") {
		return Some(60 * 60 * 24);
	}
	None
}

fn parse_relative_count(token: &str) -> Option<i64> {
	let count = token
		.chars()
		.filter(|ch| ch.is_ascii_digit())
		.collect::<String>();
	if count.is_empty() {
		None
	} else {
		count.parse::<i64>().ok()
	}
}

pub(crate) fn percent_encode(input: &str) -> String {
	let mut output = String::new();
	for byte in input.as_bytes() {
		match byte {
			b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
				output.push(*byte as char)
			}
			b' ' => output.push('+'),
			_ => {
				output.push('%');
				output.push(hex_digit(byte >> 4));
				output.push(hex_digit(byte & 0x0F));
			}
		}
	}
	output
}

pub(crate) fn hex_digit(value: u8) -> char {
	match value {
		0..=9 => (b'0' + value) as char,
		_ => (b'A' + (value - 10)) as char,
	}
}
