use aidoku::{
	AidokuError, Result,
	alloc::{String, Vec, format},
	imports::net::Request,
	prelude::*,
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use md5::{Digest, Md5};
use rabbit::{
	Rabbit,
	cipher::{KeyIvInit, StreamCipher},
};
use serde::Deserialize;
use serde_json::Value;

use crate::{
	ACCEPT_LANGUAGE, BASE_URL, current_decryption_passphrase, percent_encode,
	request_verification_token,
};

const API_BASE: &str = "https://toonlivre.net/api";
const API_SIGNATURE_DEFAULT: &str = "t8v_decoy9";
const API_SIGNATURE_CHAPTER: &str = "t8v_authX9";

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ApiPagination {
	#[serde(rename = "currentPage")]
	pub current_page: i32,
	#[serde(rename = "hasNextPage")]
	pub has_next_page: bool,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ApiChapter {
	pub id: String,
	pub number: String,
	#[serde(default)]
	pub title: String,
	#[serde(default, rename = "releaseDate")]
	pub release_date: String,
	#[serde(default)]
	pub timestamp: i64,
	#[serde(default, rename = "pageCount")]
	pub page_count: Option<i32>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ApiMangaCard {
	pub id: String,
	pub title: String,
	#[serde(default, rename = "coverUrl")]
	pub cover_url: Option<String>,
	#[serde(default)]
	pub slug: Option<String>,
	#[serde(default, rename = "alternativeTitle")]
	pub alternative_title: Option<String>,
	#[serde(default)]
	pub recent_chapters: Vec<ApiChapter>,
	#[serde(default)]
	pub registered_users_only: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ApiListResponse {
	pub mangas: Vec<ApiMangaCard>,
	pub pagination: ApiPagination,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ApiMangaById {
	pub id: String,
	pub slug: String,
	pub title: String,
	#[serde(default, rename = "coverUrl")]
	pub cover_url: Option<String>,
	#[serde(default)]
	pub authors: Vec<String>,
	#[serde(default)]
	pub artists: Vec<String>,
	#[serde(default)]
	pub genres: Vec<String>,
	#[serde(default)]
	pub description: Option<String>,
	#[serde(default)]
	pub status: Option<String>,
	#[serde(default, rename = "alternativeTitle")]
	pub alternative_title: Option<String>,
	#[serde(default)]
	pub recent_chapters: Vec<ApiChapter>,
	#[serde(default)]
	pub registered_users_only: bool,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ApiReaderManga {
	pub id: String,
	pub title: String,
	#[serde(default)]
	pub slug: Option<String>,
	#[serde(default, rename = "coverUrl")]
	pub cover_url: Option<String>,
	#[serde(default)]
	pub authors: Vec<String>,
	#[serde(default)]
	pub artists: Vec<String>,
	#[serde(default)]
	pub genres: Vec<String>,
	#[serde(default)]
	pub description: Option<String>,
	#[serde(default)]
	pub status: Option<String>,
	#[serde(default, rename = "alternativeTitle")]
	pub alternative_title: Option<String>,
	#[serde(default)]
	pub chapters: Vec<ApiChapter>,
	#[serde(default)]
	pub registered_users_only: bool,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ApiMangaBySlug {
	pub id: String,
	#[serde(default)]
	pub slug: Option<String>,
	pub title: String,
	#[serde(default, rename = "coverUrl")]
	pub cover_url: Option<String>,
	#[serde(default)]
	pub authors: Vec<String>,
	#[serde(default)]
	pub artists: Vec<String>,
	#[serde(default)]
	pub genres: Vec<String>,
	#[serde(default)]
	pub description: Option<String>,
	#[serde(default)]
	pub status: Option<String>,
	#[serde(default, rename = "alternativeTitle")]
	pub alternative_title: Option<String>,
	#[serde(default)]
	pub chapters: Vec<ApiChapter>,
	#[serde(default)]
	pub registered_users_only: bool,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ApiChapterDetails {
	pub id: String,
	pub pages: Vec<String>,
	#[serde(default)]
	pub title: String,
	pub number: String,
	#[serde(rename = "mangaId")]
	pub manga_id: String,
	#[serde(default)]
	pub timestamp: i64,
	#[serde(default, rename = "releaseDate")]
	pub release_date: String,
}

pub(crate) fn fetch_releases(page: i32, limit: i32) -> Result<ApiListResponse> {
	request_json(
		&format!("{API_BASE}/mangas/releases?page={page}&limit={limit}"),
		false,
	)
}

pub(crate) fn search_mangas(query: &str, page: i32, limit: i32) -> Result<ApiListResponse> {
	let encoded = percent_encode(query.trim());
	request_json(
		&format!(
			"{API_BASE}/mangas/search?q={encoded}&page={page}&limit={limit}&sortBy=updated&sortOrder=desc"
		),
		false,
	)
}

pub(crate) fn fetch_manga_by_id(id: &str) -> Result<ApiMangaById> {
	request_json(&format!("{API_BASE}/mangas/{id}"), false)
}

pub(crate) fn fetch_manga_reader(id: &str) -> Result<ApiReaderManga> {
	request_json(&format!("{API_BASE}/mangas/{id}/reader"), false)
}

pub(crate) fn fetch_manga_by_slug(slug: &str) -> Result<ApiMangaBySlug> {
	request_json(
		&format!(
			"{API_BASE}/manga-by-slug/{}",
			percent_encode(slug.trim_matches('/'))
		),
		false,
	)
}

pub(crate) fn fetch_chapter(manga_id: &str, chapter_id: &str) -> Result<ApiChapterDetails> {
	request_json(
		&format!("{API_BASE}/mangas/{manga_id}/chapters/{chapter_id}"),
		true,
	)
}

fn request_json<T>(url: &str, is_chapter_endpoint: bool) -> Result<T>
where
	T: serde::de::DeserializeOwned,
{
	let verification_token = request_verification_token();
	let cookie = format!("toon_v={verification_token}");
	let response = Request::get(url)?
		.header("accept", "application/json, text/plain, */*")
		.header("accept-language", ACCEPT_LANGUAGE)
		.header("origin", BASE_URL)
		.header("referer", BASE_URL)
		.header(
			"x-toon-signature",
			if is_chapter_endpoint {
				API_SIGNATURE_CHAPTER
			} else {
				API_SIGNATURE_DEFAULT
			},
		)
		.header("x-toon-verify", &verification_token)
		.header("cookie", &cookie)
		.send()?;
	let status = response.status_code();
	let data_key = response.get_header("x-toon-datakey");
	let body = response.get_string()?;
	if !(200..300).contains(&status) {
		let message = extract_error_message(&body)
			.unwrap_or_else(|| format!("Request failed with status {status}"));
		bail!("{}", message);
	}
	let body = match data_key {
		Some(data_key) => decrypt_response_payload(&body, &data_key)?,
		None => body,
	};
	serde_json::from_str(&body)
		.map_err(|err| AidokuError::Message(format!("JSON parse error: {err}")))
}

fn extract_error_message(body: &str) -> Option<String> {
	let value: Value = serde_json::from_str(body).ok()?;
	value.get("error").and_then(Value::as_str).map(String::from)
}

fn decrypt_response_payload(body: &str, data_key: &str) -> Result<String> {
	let value: Value = serde_json::from_str(body)
		.map_err(|err| AidokuError::Message(format!("JSON parse error: {err}")))?;
	let encrypted_payload = value
		.get(data_key)
		.or_else(|| value.as_object().and_then(|object| object.values().next()))
		.and_then(Value::as_str)
		.ok_or_else(|| AidokuError::Message(String::from("Missing encrypted payload")))?;
	decrypt_cryptojs_rabbit(encrypted_payload, &current_decryption_passphrase())
}

fn decrypt_cryptojs_rabbit(encrypted_data: &str, password: &str) -> Result<String> {
	let raw = STANDARD.decode(encrypted_data).map_err(|_| {
		AidokuError::Message(String::from("Failed to decode base64 chapter payload"))
	})?;
	if raw.len() < 16 || &raw[..8] != b"Salted__" {
		bail!("Invalid encrypted chapter payload");
	}
	let salt = &raw[8..16];
	let mut ciphertext = raw[16..].to_vec();
	let key_iv = evp_bytes_to_key(password.as_bytes(), salt, 24);
	let key: [u8; 16] = key_iv[..16]
		.try_into()
		.map_err(|_| AidokuError::Message(String::from("Invalid Rabbit key length")))?;
	let iv: [u8; 8] = key_iv[16..24]
		.try_into()
		.map_err(|_| AidokuError::Message(String::from("Invalid Rabbit IV length")))?;
	let mut cipher = Rabbit::new(&key.into(), &iv.into());
	cipher.apply_keystream(&mut ciphertext);
	String::from_utf8(ciphertext)
		.map_err(|err| AidokuError::Message(format!("UTF-8 decode error: {err}")))
}

fn evp_bytes_to_key(password: &[u8], salt: &[u8], output_len: usize) -> Vec<u8> {
	let mut output = Vec::with_capacity(output_len);
	let mut previous = Vec::new();
	while output.len() < output_len {
		let mut hasher = Md5::new();
		if !previous.is_empty() {
			hasher.update(&previous);
		}
		hasher.update(password);
		hasher.update(salt);
		previous = hasher.finalize().to_vec();
		let remaining = output_len - output.len();
		let take = remaining.min(previous.len());
		output.extend_from_slice(&previous[..take]);
	}
	output
}
