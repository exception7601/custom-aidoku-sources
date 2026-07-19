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
	BASE_URL, active_manifest, current_decryption_passphrase_for_manifest,
	generate_session_cookie_value, percent_encode, signature_value_for_url,
};

const API_BASE: &str = "https://toonlivre.net/api";
const ERROR_BODY_SNIPPET_LIMIT: usize = 220;

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

struct RequestFailureContext<'a> {
	url: &'a str,
	status: i32,
	body: &'a str,
	manifest: &'a crate::ClientManifest,
	signature_value: &'a str,
	content_type: Option<&'a str>,
	cf_ray: Option<&'a str>,
	retry_after: Option<&'a str>,
	rate_remaining: Option<&'a str>,
	rate_reset: Option<&'a str>,
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
	let manifest = active_manifest();
	let signature_value = String::from(signature_value_for_url(&manifest, url));
	let verification_token = generate_session_cookie_value(&manifest);
	source_log!(
		"[toonlivre] request_json start url={} signature_header={} verify_header={} cookie_name={} passphrase={}",
		url,
		manifest.request.signature_header,
		manifest.request.verify_header,
		manifest.request.session_cookie.name,
		current_decryption_passphrase_for_manifest(&manifest)
	);
	let cookie = format!(
		"{}={verification_token}",
		manifest.request.session_cookie.name
	);
	let mut request = Request::get(url)?
		.header("accept", "application/json, text/plain, */*")
		.header("accept-language", &manifest.request.accept_language)
		.header("user-agent", &manifest.request.user_agent)
		.header("origin", BASE_URL)
		.header("referer", BASE_URL)
		.header("cookie", &cookie);
	request.set_header(manifest.request.signature_header.as_str(), &signature_value);
	request.set_header(&manifest.request.verify_header, &verification_token);
	for header_name in manifest.request.session_cookie.mirrors_into.iter() {
		request.set_header(header_name, &verification_token);
	}
	let response = request.send().map_err(|error| {
		AidokuError::Message(format!(
			"ToonLivre request could not be sent.\nURL: {url}\nError: {error:?}\nHint: verifique rede, Cloudflare e tente novamente."
		))
	})?;
	let status = response.status_code();
	let data_key = response.get_header("x-toon-datakey");
	let content_type = response.get_header("content-type");
	let cf_ray = response.get_header("cf-ray");
	let retry_after = response.get_header("retry-after");
	let rate_remaining = response.get_header("ratelimit-remaining");
	let rate_reset = response.get_header("ratelimit-reset");
	source_log!(
		"[toonlivre] request_json response url={} status={} data_key={:?} content_type={:?} cf_ray={:?}",
		url,
		status,
		data_key.as_deref(),
		content_type.as_deref(),
		cf_ray.as_deref()
	);
	let body = response.get_string().map_err(|error| {
		AidokuError::Message(format!(
			"Failed to read ToonLivre response body.\nURL: {url}\nStatus: {status}\nError: {error:?}"
		))
	})?;
	source_log!(
		"[toonlivre] request_json body url={} snippet={}",
		url,
		summarize_body(&body)
	);
	if !(200..300).contains(&status) {
		bail!(
			"{}",
			format_request_failure(RequestFailureContext {
				url,
				status,
				body: &body,
				manifest: &manifest,
				signature_value: &signature_value,
				content_type: content_type.as_deref(),
				cf_ray: cf_ray.as_deref(),
				retry_after: retry_after.as_deref(),
				rate_remaining: rate_remaining.as_deref(),
				rate_reset: rate_reset.as_deref(),
			})
		);
	}
	let body = match data_key.as_deref() {
		Some(data_key) => {
			source_log!(
				"[toonlivre] request_json decrypt start url={} data_key={} body_snippet={}",
				url,
				data_key,
				summarize_body(&body)
			);
			decrypt_response_payload(&body, data_key, &manifest).map_err(|error| {
				AidokuError::Message(format_payload_failure(
					url,
					Some(data_key),
					&body,
					&manifest,
					&format!("{error:?}"),
					"decrypt",
				))
			})?
		}
		None if url.contains("/chapters/") => {
			bail!(
				"{}",
				format_payload_failure(
					url,
					None,
					&body,
					&manifest,
					"Missing `x-toon-datakey` response header",
					"decrypt",
				)
			);
		}
		None => body,
	};
	source_log!(
		"[toonlivre] request_json final_payload url={} snippet={}",
		url,
		summarize_body(&body)
	);
	serde_json::from_str(&body).map_err(|error| {
		AidokuError::Message(format!(
			"Failed to parse ToonLivre JSON response.\nURL: {url}\nError: {error}\nBody: {}",
			summarize_body(&body)
		))
	})
}

fn extract_error_message(body: &str) -> Option<String> {
	let value: Value = serde_json::from_str(body).ok()?;
	value
		.get("error")
		.or_else(|| value.get("message"))
		.and_then(Value::as_str)
		.map(String::from)
}

fn format_request_failure(context: RequestFailureContext<'_>) -> String {
	let mut message = String::from("ToonLivre request failed.");
	push_detail_line(&mut message, "URL", context.url);
	push_detail_line(
		&mut message,
		"Status",
		&format!("{} {}", context.status, describe_status(context.status)),
	);
	push_detail_line(
		&mut message,
		"Request signature",
		&format!(
			"{}={}",
			context.manifest.request.signature_header, context.signature_value
		),
	);
	push_detail_line(
		&mut message,
		"Token mirror",
		&format!(
			"{} + cookie {}",
			context.manifest.request.verify_header, context.manifest.request.session_cookie.name
		),
	);

	let mut response_headers = Vec::new();
	if let Some(content_type) = context.content_type {
		response_headers.push(format!("content-type={content_type}"));
	}
	if let Some(cf_ray) = context.cf_ray {
		response_headers.push(format!("cf-ray={cf_ray}"));
	}
	if let Some(retry_after) = context.retry_after {
		response_headers.push(format!("retry-after={retry_after}"));
	}
	if let Some(rate_remaining) = context.rate_remaining {
		response_headers.push(format!("ratelimit-remaining={rate_remaining}"));
	}
	if let Some(rate_reset) = context.rate_reset {
		response_headers.push(format!("ratelimit-reset={rate_reset}"));
	}
	if !response_headers.is_empty() {
		push_detail_line(
			&mut message,
			"Response headers",
			&response_headers.join(", "),
		);
	}

	if let Some(api_error) = extract_error_message(context.body) {
		push_detail_line(&mut message, "Response error", &api_error);
	}
	if let Some(hint) = request_failure_hint(
		context.status,
		context.body,
		context.content_type,
		context.retry_after,
		context.rate_reset,
	) {
		push_detail_line(&mut message, "Hint", &hint);
	}
	let snippet = summarize_body(context.body);
	if !snippet.is_empty() {
		push_detail_line(&mut message, "Body", &snippet);
	}
	message
}

fn format_payload_failure(
	url: &str,
	data_key: Option<&str>,
	body: &str,
	manifest: &crate::ClientManifest,
	cause: &str,
	stage: &str,
) -> String {
	let mut message = if stage == "parse" {
		String::from("ToonLivre chapter payload was decrypted, but JSON parsing failed.")
	} else {
		String::from("ToonLivre chapter payload could not be decrypted.")
	};
	push_detail_line(&mut message, "URL", url);
	push_detail_line(
		&mut message,
		"Data key",
		&format!(
			"{}={}",
			manifest.decrypt.data_key_header,
			data_key.unwrap_or("missing")
		),
	);
	push_detail_line(&mut message, "Algorithm", &manifest.decrypt.algorithm);
	push_detail_line(&mut message, "Cause", cause);
	if stage == "parse" {
		push_detail_line(
			&mut message,
			"Hint",
			"A descriptografia funcionou, mas o formato JSON retornado mudou e precisa ser revisado.",
		);
	} else {
		push_detail_line(
			&mut message,
			"Hint",
			"A receita do manifesto para data key, algoritmo ou passphrase pode ter ficado desatualizada.",
		);
	}
	let snippet = summarize_body(body);
	if !snippet.is_empty() {
		push_detail_line(
			&mut message,
			if stage == "parse" { "Payload" } else { "Body" },
			&snippet,
		);
	}
	message
}

fn request_failure_hint(
	status: i32,
	body: &str,
	content_type: Option<&str>,
	retry_after: Option<&str>,
	rate_reset: Option<&str>,
) -> Option<String> {
	if status == 403 {
		return Some(String::from(
			"O endpoint rejeitou o token espelhado ou a assinatura do manifesto. Confira os headers de capítulo e reextraia o manifesto.",
		));
	}
	if status == 429 {
		if let Some(wait_seconds) = retry_after.or(rate_reset) {
			return Some(format!(
				"O site limitou as requisições. Aguarde {wait_seconds} segundo(s) antes de tentar novamente."
			));
		}
		return Some(String::from(
			"O site limitou as requisições. Aguarde alguns instantes antes de tentar novamente.",
		));
	}
	if is_html_like_response(body, content_type) {
		return Some(String::from(
			"O site respondeu HTML/Cloudflare em vez de JSON. Pode ser um bloqueio temporário ou desafio anti-bot.",
		));
	}
	if status >= 500 {
		return Some(String::from(
			"O ToonLivre respondeu com erro interno. Tente novamente mais tarde.",
		));
	}
	None
}

fn describe_status(status: i32) -> &'static str {
	match status {
		400 => "Bad Request",
		401 => "Unauthorized",
		403 => "Forbidden",
		404 => "Not Found",
		429 => "Too Many Requests",
		500 => "Internal Server Error",
		502 => "Bad Gateway",
		503 => "Service Unavailable",
		504 => "Gateway Timeout",
		_ => "Unexpected Response",
	}
}

fn is_html_like_response(body: &str, content_type: Option<&str>) -> bool {
	let content_type = content_type.unwrap_or_default().to_lowercase();
	let normalized_body = body.trim().to_lowercase();
	content_type.contains("text/html")
		|| normalized_body.starts_with("<!doctype html")
		|| normalized_body.starts_with("<html")
		|| normalized_body.contains("cloudflare")
		|| normalized_body.contains("cdn-cgi")
}

fn summarize_body(body: &str) -> String {
	let trimmed = body.trim();
	if trimmed.is_empty() {
		return String::new();
	}
	let mut output = String::new();
	let mut previous_was_space = false;
	for ch in trimmed.chars() {
		let normalized = if ch.is_whitespace() { ' ' } else { ch };
		if normalized == ' ' {
			if previous_was_space {
				continue;
			}
			previous_was_space = true;
		} else {
			previous_was_space = false;
		}
		if output.len() >= ERROR_BODY_SNIPPET_LIMIT {
			output.push_str("...");
			break;
		}
		output.push(normalized);
	}
	output
}

fn push_detail_line(message: &mut String, label: &str, value: &str) {
	if value.trim().is_empty() {
		return;
	}
	message.push('\n');
	message.push_str(label);
	message.push_str(": ");
	message.push_str(value);
}

fn decrypt_response_payload(
	body: &str,
	data_key: &str,
	manifest: &crate::ClientManifest,
) -> Result<String> {
	source_log!(
		"[toonlivre] decrypt_response_payload start data_key={} selector={} body_snippet={}",
		data_key,
		manifest.decrypt.payload_selector,
		summarize_body(body)
	);
	let value: Value = serde_json::from_str(body)
		.map_err(|err| AidokuError::Message(format!("JSON parse error: {err}")))?;
	let encrypted_payload = value
		.get(data_key)
		.or_else(|| value.as_object().and_then(|object| object.values().next()))
		.and_then(Value::as_str)
		.ok_or_else(|| AidokuError::Message(String::from("Missing encrypted payload")))?;
	source_log!(
		"[toonlivre] decrypt_response_payload extracted key={} payload_len={}",
		data_key,
		encrypted_payload.len()
	);
	decrypt_cryptojs_rabbit(
		encrypted_payload,
		&current_decryption_passphrase_for_manifest(manifest),
	)
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
	source_log!(
		"[toonlivre] decrypt_cryptojs_rabbit decoded raw_len={} ciphertext_len={} salt={}",
		raw.len(),
		ciphertext.len(),
		hex_lower_string(salt)
	);
	let key_iv = evp_bytes_to_key(password.as_bytes(), salt, 24);
	let key: [u8; 16] = key_iv[..16]
		.try_into()
		.map_err(|_| AidokuError::Message(String::from("Invalid Rabbit key length")))?;
	let iv: [u8; 8] = key_iv[16..24]
		.try_into()
		.map_err(|_| AidokuError::Message(String::from("Invalid Rabbit IV length")))?;
	let mut cipher = Rabbit::new(&key.into(), &iv.into());
	cipher.apply_keystream(&mut ciphertext);
	source_log!(
		"[toonlivre] decrypt_cryptojs_rabbit key={} iv={} ciphertext_after_len={}",
		hex_lower_string(&key),
		hex_lower_string(&iv),
		ciphertext.len()
	);
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

fn hex_lower_string(bytes: &[u8]) -> String {
	let mut output = String::new();
	for byte in bytes.iter() {
		output.push(hex_lower_digit(byte >> 4));
		output.push(hex_lower_digit(byte & 0x0F));
	}
	output
}

fn hex_lower_digit(value: u8) -> char {
	match value {
		0..=9 => (b'0' + value) as char,
		_ => (b'a' + (value - 10)) as char,
	}
}
