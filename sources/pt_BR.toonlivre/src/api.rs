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

use crate::{percent_encode, token_server};

const API_BASE: &str = "https://toonlivre.net/api";

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ApiPagination {
	#[serde(rename = "currentPage")]
	pub current_page: i64,
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
	pub page_count: Option<i64>,
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
	source_log!("[toonlivre] fetch_releases page={} limit={}", page, limit);
	request_json(&format!(
		"{API_BASE}/mangas/releases?page={page}&limit={limit}"
	))
}

pub(crate) fn search_mangas(query: &str, page: i32, limit: i32) -> Result<ApiListResponse> {
	source_log!(
		"[toonlivre] search_mangas query={} page={} limit={}",
		query,
		page,
		limit
	);
	let encoded = percent_encode(query.trim());
	request_json(&format!(
		"{API_BASE}/mangas/search?q={encoded}&page={page}&limit={limit}&sortBy=updated&sortOrder=desc"
	))
}

pub(crate) fn fetch_manga_by_id(id: &str) -> Result<ApiMangaById> {
	source_log!("[toonlivre] fetch_manga_by_id id={id}");
	request_json(&format!("{API_BASE}/mangas/{id}"))
}

pub(crate) fn fetch_manga_reader(id: &str) -> Result<ApiReaderManga> {
	source_log!("[toonlivre] fetch_manga_reader id={id}");
	request_json(&format!("{API_BASE}/mangas/{id}/reader"))
}

pub(crate) fn fetch_manga_by_slug(slug: &str) -> Result<ApiMangaBySlug> {
	source_log!("[toonlivre] fetch_manga_by_slug slug={}", slug);
	request_json(&format!(
		"{API_BASE}/manga-by-slug/{}",
		percent_encode(slug.trim_matches('/'))
	))
}

pub(crate) fn fetch_chapter(manga_id: &str, chapter_id: &str) -> Result<ApiChapterDetails> {
	source_log!(
		"[toonlivre] fetch_chapter manga_id={} chapter_id={}",
		manga_id,
		chapter_id
	);
	request_json(&format!(
		"{API_BASE}/mangas/{manga_id}/chapters/{chapter_id}"
	))
}

fn request_json<T>(url: &str) -> Result<T>
where
	T: serde::de::DeserializeOwned,
{
	source_log!("[toonlivre] request_json url={}", url);
	let tokens_url = token_server::full_tokens_url()
		.ok_or_else(|| AidokuError::Message(String::from("Failed to get tokens URL")))?;
	let response = Request::post(&tokens_url)?
		.body(
			serde_json::to_vec(&serde_json::json!({
				"url": url
			}))
			.unwrap()
			.as_slice(),
		)
		.header("content-type", "application/json")
		.send()
		.map_err(|e| AidokuError::Message(format!("Token server request failed: {:?}", e)))?;

	if response.status_code() != 200 {
		bail!(
			"ToonLivre request failed with status {}",
			response.status_code()
		);
	}

	let body = response.get_string().map_err(|e| {
		AidokuError::Message(format!("Token server response body read failed: {:?}", e))
	})?;

	let tokens: token_server::TokenServerResponse = serde_json::from_str(&body).map_err(|e| {
		AidokuError::Message(format!("Token server response parse failed: {:?}", e))
	})?;

	let headers = tokens.headers.ok_or_else(|| {
		AidokuError::Message(String::from("Token server response missing headers"))
	})?;
	let passphrase = tokens.passphrase.ok_or_else(|| {
		AidokuError::Message(String::from("Token server response missing passphrase"))
	})?;

	let request = Request::get(url)?
		.header("accept", "application/json, text/plain, */*")
		.header(
			"user-agent",
			"Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36",
		)
		.header("accept-language", "pt-BR,pt;q=0.9")
		.header("referer", "https://toonlivre.net/")
		.header("x-toon-signature", &headers.signature)
		.header("x-toon-verify", &headers.verify);

	let response = request.send().map_err(|error| {
		AidokuError::Message(format!(
			"ToonLivre request (with tokens) could not be sent.\nURL: {url}\nError: {error:?}"
		))
	})?;

	let status = response.status_code();
	let body = response.get_string().map_err(|error| {
		AidokuError::Message(format!(
			"Failed to read ToonLivre response body.\nURL: {url}\nStatus: {status}\nError: {error:?}"
		))
	})?;

	if !(200..300).contains(&status) {
		bail!("ToonLivre request failed with status {}", status);
	}

	let body = if url.contains("/chapters/") {
		source_log!("[toonlivre] decrypting chapter payload using token_server passphrase");
		let obj: serde_json::Map<String, serde_json::Value> =
			serde_json::from_str(&body).map_err(|error| {
				AidokuError::Message(format!("Failed to parse encrypted JSON container: {error}"))
			})?;
		let encrypted_val = obj.values().next().ok_or_else(|| {
			AidokuError::Message(String::from("Encrypted JSON container is empty"))
		})?;
		let encrypted_str = encrypted_val.as_str().ok_or_else(|| {
			AidokuError::Message(String::from("Encrypted JSON value is not a string"))
		})?;
		decrypt_cryptojs_rabbit(encrypted_str, &passphrase)?
	} else {
		body
	};

	serde_json::from_str(&body).map_err(|error| {
		AidokuError::Message(format!(
			"Failed to parse ToonLivre JSON response (with tokens).\nURL: {url}\nError: {error}"
		))
	})
}

fn decrypt_cryptojs_rabbit(encrypted_data: &str, password: &str) -> Result<String> {
	source_log!(
		"[toonlivre] decrypt_cryptojs_rabbit start encrypted_len={} password={}",
		encrypted_data.len(),
		password
	);
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
	source_log!(
		"[toonlivre] evp_bytes_to_key start password_len={} salt={} output_len={}",
		password.len(),
		hex_lower_string(salt),
		output_len
	);
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
	source_log!(
		"[toonlivre] evp_bytes_to_key done derived_len={} derived={}",
		output.len(),
		hex_lower_string(&output)
	);
	output
}

#[allow(dead_code)]
fn hex_lower_string(bytes: &[u8]) -> String {
	let mut output = String::new();
	for byte in bytes.iter() {
		output.push(hex_lower_digit(byte >> 4));
		output.push(hex_lower_digit(byte & 0x0F));
	}
	output
}

#[allow(dead_code)]
fn hex_lower_digit(value: u8) -> char {
	match value {
		0..=9 => (b'0' + value) as char,
		_ => (b'a' + (value - 10)) as char,
	}
}
