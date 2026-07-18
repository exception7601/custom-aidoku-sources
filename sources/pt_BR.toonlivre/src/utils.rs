use aidoku::{
	Chapter, DeepLinkResult, Manga, MangaStatus,
	alloc::{String, format},
	imports::std::current_date,
};
use md5::{Digest, Md5};
use unicode_normalization::{UnicodeNormalization, char::is_combining_mark};

use crate::BASE_URL;

const RESERVED_PATHS: [&str; 6] = [
	"favorites",
	"profile",
	"admin",
	"read",
	"api",
	"password-reset",
];
const VERIFY_TOKEN: &str = "aidoku-toonlivre";
const DECRYPTION_SALT: &str = "toonlivre.tv::v8";
const DECRYPTION_SUFFIX: &str = "t17_4v19_b2";
const DECRYPTION_PREFIX: &str = "Dealer-Critter-Catnip4";

pub(crate) fn verify_token() -> &'static str {
	VERIFY_TOKEN
}

pub(crate) fn manga_url_from_slug(slug: &str) -> String {
	format!("{BASE_URL}/{}", slug.trim_matches('/'))
}

pub(crate) fn chapter_url_from_slug_and_number(slug: &str, chapter_number: &str) -> String {
	format!(
		"{BASE_URL}/{}/{}",
		slug.trim_matches('/'),
		chapter_segment(chapter_number)
	)
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

pub(crate) fn normalize_chapter_number(value: &str) -> String {
	String::from(value.trim())
}

pub(crate) fn chapter_segment(value: &str) -> String {
	let value = normalize_chapter_number(value);
	if value.is_empty() {
		return value;
	}
	if value.chars().all(|ch| ch.is_ascii_digit()) {
		if let Ok(number) = value.parse::<i32>() {
			if number < 100 {
				return format!("{number:02}");
			}
		}
		return value;
	}
	if let Some(stripped) = value.strip_prefix('0') {
		if stripped
			.chars()
			.next()
			.map(|ch| ch.is_ascii_digit())
			.unwrap_or(false)
		{
			return String::from(stripped);
		}
	}
	value
}

pub(crate) fn chapter_numbers_match(left: &str, right: &str) -> bool {
	trim_leading_zeroes(left) == trim_leading_zeroes(right)
}

fn trim_leading_zeroes(value: &str) -> String {
	let trimmed = value.trim();
	if trimmed.is_empty() {
		return String::new();
	}
	let stripped = trimmed.trim_start_matches('0');
	if stripped.is_empty() {
		String::from("0")
	} else {
		String::from(stripped)
	}
}

pub(crate) fn parse_chapter_number(value: &str) -> Option<f32> {
	let normalized = normalize_chapter_number(value).replace(',', ".");
	normalized.parse::<f32>().ok()
}

pub(crate) fn date_from_timestamp_millis(timestamp: i64) -> Option<i64> {
	(timestamp > 0).then_some(timestamp / 1000)
}

pub(crate) fn slugify_title(title: &str) -> String {
	let mut slug = String::new();
	let mut last_was_dash = false;
	for ch in title.nfd() {
		if is_combining_mark(ch) {
			continue;
		}
		for lower in ch.to_lowercase() {
			if lower.is_ascii_alphanumeric() {
				slug.push(lower);
				last_was_dash = false;
			} else if !last_was_dash && !slug.is_empty() {
				slug.push('-');
				last_was_dash = true;
			}
		}
	}
	while slug.ends_with('-') {
		slug.pop();
	}
	if slug.starts_with("custom-") {
		return format!("obra-{}", &slug[7..]);
	}
	slug
}

pub(crate) fn path_segments(url: &str) -> aidoku::alloc::Vec<String> {
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

fn is_reserved_segment(segment: &str) -> bool {
	RESERVED_PATHS.iter().any(|reserved| reserved == &segment)
}

pub(crate) fn deep_link_result(url: &str) -> Option<DeepLinkResult> {
	let segments = path_segments(url);
	if segments.is_empty() {
		return None;
	}

	if segments[0] == "read" {
		if segments.len() >= 4 {
			return Some(DeepLinkResult::Chapter {
				manga_key: segments[2].clone(),
				key: segments[3].clone(),
			});
		}
		if segments.len() >= 3 {
			return Some(DeepLinkResult::Chapter {
				manga_key: segments[1].clone(),
				key: segments[2].clone(),
			});
		}
		return None;
	}

	if segments[0] == "manga" {
		if segments.len() >= 3 {
			return Some(DeepLinkResult::Manga {
				key: segments[2].clone(),
			});
		}
		if segments.len() >= 2 {
			return Some(DeepLinkResult::Manga {
				key: segments[1].clone(),
			});
		}
		return None;
	}

	if is_reserved_segment(&segments[0]) {
		return None;
	}

	if segments.len() >= 2 {
		return Some(DeepLinkResult::Chapter {
			manga_key: segments[0].clone(),
			key: segments[1].clone(),
		});
	}

	Some(DeepLinkResult::Manga {
		key: segments[0].clone(),
	})
}

pub(crate) fn manga_slug_from_manga(manga: &Manga) -> Option<String> {
	if let Some(url) = manga.url.as_deref() {
		if let Some(slug) = manga_slug_from_url(url) {
			return Some(slug);
		}
	}
	let key = manga.key.trim();
	if key.is_empty() || key.starts_with("obra-") {
		return None;
	}
	if key.starts_with("http://") || key.starts_with("https://") {
		return manga_slug_from_url(key);
	}
	Some(String::from(key.trim_matches('/')))
}

pub(crate) fn manga_slug_from_url(url: &str) -> Option<String> {
	match deep_link_result(url)? {
		DeepLinkResult::Manga { key } => {
			if key.starts_with("obra-") {
				None
			} else {
				Some(key)
			}
		}
		DeepLinkResult::Chapter { manga_key, .. } => {
			if manga_key.starts_with("obra-") {
				None
			} else {
				Some(manga_key)
			}
		}
		DeepLinkResult::Listing(_) => None,
	}
}

pub(crate) fn chapter_key_or_number(chapter: &Chapter) -> Option<String> {
	if !chapter.key.trim().is_empty() {
		return Some(String::from(chapter.key.trim()));
	}
	if let Some(url) = chapter.url.as_deref() {
		if let Some(value) = chapter_number_from_url(url) {
			return Some(value);
		}
	}
	chapter.chapter_number.map(|value| {
		let mut text = format!("{value}");
		if text.ends_with(".0") {
			text.truncate(text.len() - 2);
		}
		text
	})
}

pub(crate) fn chapter_number_from_url(url: &str) -> Option<String> {
	match deep_link_result(url)? {
		DeepLinkResult::Chapter { key, .. } => Some(key),
		DeepLinkResult::Manga { .. } | DeepLinkResult::Listing(_) => None,
	}
}

pub(crate) fn current_decryption_passphrase() -> String {
	let date = current_utc_date_string();
	let seed = format!("{date}{DECRYPTION_SALT}{DECRYPTION_SUFFIX}");
	let mut hasher = Md5::new();
	hasher.update(seed.as_bytes());
	let digest = hasher.finalize();
	let mut suffix = String::new();
	for byte in digest[..4].iter() {
		suffix.push(hex_digit(byte >> 4));
		suffix.push(hex_digit(byte & 0x0F));
	}
	format!("{DECRYPTION_PREFIX}{suffix}")
}

fn hex_digit(value: u8) -> char {
	match value {
		0..=9 => (b'0' + value) as char,
		_ => (b'a' + (value - 10)) as char,
	}
}

fn current_utc_date_string() -> String {
	let days = current_date().div_euclid(86_400);
	let (year, month, day) = civil_from_days(days);
	format!("{year:04}-{month:02}-{day:02}")
}

fn civil_from_days(days_since_unix_epoch: i64) -> (i32, u32, u32) {
	let z = days_since_unix_epoch + 719_468;
	let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
	let doe = z - era * 146_097;
	let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
	let y = yoe + era * 400;
	let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
	let mp = (5 * doy + 2) / 153;
	let day = doy - (153 * mp + 2) / 5 + 1;
	let month = mp + if mp < 10 { 3 } else { -9 };
	let year = y + if month <= 2 { 1 } else { 0 };
	(year as i32, month as u32, day as u32)
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
