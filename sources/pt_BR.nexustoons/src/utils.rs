use aidoku::{
	Chapter, DeepLinkResult, Manga, MangaStatus,
	alloc::{String, Vec, format},
};

use crate::BASE_URL;

pub(crate) fn manga_url_from_slug(slug: &str) -> String {
	format!("{BASE_URL}/manga/{}", slug.trim().trim_matches('/'))
}

pub(crate) fn chapter_url_from_slug_and_id(slug: &str, chapter_id: &str) -> String {
	format!(
		"{BASE_URL}/ler/{}/{}",
		slug.trim().trim_matches('/'),
		chapter_id.trim().trim_matches('/')
	)
}

pub(crate) fn manga_slug_from_manga(manga: &Manga) -> Option<String> {
	if let Some(url) = manga.url.as_deref()
		&& let Some(slug) = manga_slug_from_url(url)
	{
		return Some(slug);
	}
	let key = manga.key.trim();
	if key.is_empty() {
		return None;
	}
	Some(String::from(key.trim_matches('/')))
}

pub(crate) fn manga_slug_from_url(url: &str) -> Option<String> {
	match deep_link_result(url)? {
		DeepLinkResult::Manga { key } => Some(key),
		DeepLinkResult::Chapter { manga_key, .. } => Some(manga_key),
		DeepLinkResult::Listing(_) => None,
	}
}

pub(crate) fn chapter_id_from_url(url: &str) -> Option<String> {
	match deep_link_result(url)? {
		DeepLinkResult::Chapter { key, .. } => Some(key),
		DeepLinkResult::Manga { .. } | DeepLinkResult::Listing(_) => None,
	}
}

pub(crate) fn chapter_key_or_id(chapter: &Chapter) -> Option<String> {
	if !chapter.key.trim().is_empty() {
		return Some(String::from(chapter.key.trim()));
	}
	chapter.url.as_deref().and_then(chapter_id_from_url)
}

pub(crate) fn chapter_title_from_number(number: &str) -> String {
	format!("Capítulo {}", number.trim())
}

pub(crate) fn parse_chapter_number(value: &str) -> Option<f32> {
	let normalized = value.trim().replace(',', ".");
	normalized.parse::<f32>().ok()
}

pub(crate) fn manga_status_from_text(status: &str) -> MangaStatus {
	match status.trim().to_lowercase().as_str() {
		"ongoing" | "em lancamento" | "em lançamento" => MangaStatus::Ongoing,
		"completed" | "completo" | "concluido" | "concluído" => MangaStatus::Completed,
		"cancelled" | "cancelado" => MangaStatus::Cancelled,
		"hiatus" | "hiato" => MangaStatus::Hiatus,
		_ => MangaStatus::Unknown,
	}
}

pub(crate) fn path_segments(url: &str) -> Vec<String> {
	let mut cleaned = url.trim();
	if let Some((_, rest)) = cleaned.split_once("://") {
		cleaned = rest;
		if let Some((_, rest)) = cleaned.split_once('/') {
			cleaned = rest;
		} else {
			cleaned = "";
		}
	}
	cleaned = cleaned.trim_start_matches('/');
	let cleaned = cleaned.split('?').next().unwrap_or(cleaned);
	let cleaned = cleaned.split('#').next().unwrap_or(cleaned);
	cleaned
		.split('/')
		.filter(|segment| !segment.is_empty())
		.map(String::from)
		.collect()
}

pub(crate) fn deep_link_result(url: &str) -> Option<DeepLinkResult> {
	let segments = path_segments(url);
	if segments.is_empty() {
		return None;
	}

	match segments[0].as_str() {
		"manga" if segments.len() >= 2 => Some(DeepLinkResult::Manga {
			key: segments[1].clone(),
		}),
		"ler" if segments.len() >= 3 => Some(DeepLinkResult::Chapter {
			manga_key: segments[1].clone(),
			key: segments[2].clone(),
		}),
		_ => None,
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
				output.push(upper_hex_digit(byte >> 4));
				output.push(upper_hex_digit(byte & 0x0F));
			}
		}
	}
	output
}

fn upper_hex_digit(value: u8) -> char {
	match value {
		0..=9 => (b'0' + value) as char,
		_ => (b'A' + (value - 10)) as char,
	}
}
