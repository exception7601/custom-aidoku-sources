use aidoku::{
	AidokuError, Result,
	alloc::{String, Vec, format},
	imports::{net::Request, std::current_date},
	prelude::*,
};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::{ACCEPT_LANGUAGE, BASE_URL, percent_encode};

const API_BASE: &str = "https://toonlivre.net/api";
const DIRECT_USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/151.0.0.0 Safari/537.36";

#[derive(Debug, Clone)]
struct AuthTokens {
	signature: String,
	session: String,
}

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
	#[serde(default, alias = "uploadSlug")]
	pub slug: Option<String>,
	#[serde(default, rename = "alternativeTitle")]
	pub alternative_title: Option<String>,
	#[serde(default, rename = "recentChapters")]
	pub recent_chapters: Vec<ApiChapter>,
	#[serde(default, rename = "registeredUsersOnly")]
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
	#[serde(default, rename = "recentChapters")]
	pub recent_chapters: Vec<ApiChapter>,
	#[serde(default, rename = "registeredUsersOnly")]
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
	#[serde(default, rename = "registeredUsersOnly")]
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
	#[serde(default, rename = "registeredUsersOnly")]
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

#[derive(Debug, Clone, Deserialize)]
struct SeedResponse {
	token: Option<String>,
}

fn generate_session() -> String {
	let now = current_date() as u64;
	let mixed = now.rotate_left(13) ^ now.wrapping_mul(0x9E37_79B9_7F4A_7C15);
	format!("{:x}{:x}", now, mixed)
}

fn parse_json<T>(url: &str, body: &str) -> Result<T>
where
	T: DeserializeOwned,
{
	serde_json::from_str(body).map_err(|error| {
		AidokuError::Message(format!(
			"Failed to parse JSON response.\nURL: {url}\nError: {error}"
		))
	})
}

fn response_error(url: &str, status: i32, body: &str) -> AidokuError {
	let message = serde_json::from_str::<Value>(body)
		.ok()
		.and_then(|json| {
			json.get("error")
				.and_then(|value| value.as_str())
				.map(String::from)
		})
		.unwrap_or_else(|| String::from(body.trim()));
	AidokuError::Message(format!(
		"Request failed.\nURL: {url}\nStatus: {status}\nError: {message}"
	))
}

fn send_request(
	url: &str,
	referer: &str,
	auth: Option<&AuthTokens>,
) -> Result<aidoku::imports::net::Response> {
	let mut request = Request::get(url)?
		.header("Accept", "application/json, text/plain, */*")
		.header("User-Agent", DIRECT_USER_AGENT)
		.header("Accept-Language", ACCEPT_LANGUAGE)
		.header("Referer", referer);

	if let Some(auth) = auth {
		request.set_header(String::from("Cookie"), format!("toon_v={}", auth.session));
		request.set_header(String::from("x-toon-signature"), auth.signature.clone());
	}

	request.send().map_err(|error| {
		AidokuError::Message(format!("Request failed.\nURL: {url}\nError: {error:?}"))
	})
}

fn fetch_seed_jwt() -> Result<String> {
	let url = format!("{API_BASE}/seed");
	let response = send_request(&url, BASE_URL, None)?;
	let status = response.status_code();
	let body = response.get_string().map_err(|error| {
		AidokuError::Message(format!(
			"Failed to read seed response body.\nURL: {url}\nStatus: {status}\nError: {error:?}"
		))
	})?;

	if !(200..300).contains(&status) {
		return Err(response_error(&url, status, &body));
	}

	let response_data: SeedResponse = parse_json(&url, &body)?;
	response_data.token.ok_or_else(|| {
		AidokuError::Message(format!("Seed response missing token field.\nURL: {url}"))
	})
}

fn get_auth_tokens() -> Result<AuthTokens> {
	let session = generate_session();
	let signature = fetch_seed_jwt()?;
	Ok(AuthTokens { signature, session })
}

fn request_json_with_auth<T>(url: &str, referer: &str, auth: &AuthTokens) -> Result<T>
where
	T: DeserializeOwned,
{
	let response = send_request(url, referer, Some(auth))?;
	let status = response.status_code();
	let body = response.get_string().map_err(|error| {
		AidokuError::Message(format!(
			"Failed to read response body.\nURL: {url}\nStatus: {status}\nError: {error:?}"
		))
	})?;

	if !(200..300).contains(&status) {
		return Err(response_error(url, status, &body));
	}

	parse_json(url, &body)
}

fn request_json<T>(url: &str, referer: &str) -> Result<T>
where
	T: DeserializeOwned,
{
	let auth = get_auth_tokens()?;
	request_json_with_auth(url, referer, &auth)
}

pub(crate) fn fetch_releases(page: i32, limit: i32) -> Result<ApiListResponse> {
	let url = format!("{API_BASE}/mangas/releases?page={page}&limit={limit}");
	source_log!("[toonlivre] fetch_releases page={} limit={}", page, limit);
	request_json(&url, BASE_URL)
}

pub(crate) fn search_mangas(query: &str, page: i32, limit: i32) -> Result<ApiListResponse> {
	let encoded = percent_encode(query.trim());
	let url = format!(
		"{API_BASE}/mangas/search?q={encoded}&page={page}&limit={limit}&sortBy=updated&sortOrder=desc"
	);
	source_log!(
		"[toonlivre] search_mangas query={} page={} limit={}",
		query,
		page,
		limit
	);
	request_json(&url, BASE_URL)
}

pub(crate) fn fetch_manga_by_id(id: &str) -> Result<ApiMangaById> {
	let url = format!("{API_BASE}/mangas/{}", percent_encode(id));
	source_log!("[toonlivre] fetch_manga_by_id id={}", id);
	request_json(&url, BASE_URL)
}

pub(crate) fn fetch_manga_reader(id: &str) -> Result<ApiReaderManga> {
	let url = format!("{API_BASE}/mangas/{}/reader", percent_encode(id));
	source_log!("[toonlivre] fetch_manga_reader id={}", id);
	request_json(&url, BASE_URL)
}

pub(crate) fn fetch_manga_by_slug(slug: &str) -> Result<ApiMangaBySlug> {
	let encoded = percent_encode(slug.trim_matches('/'));
	let url = format!("{API_BASE}/manga-by-slug/{encoded}");
	source_log!("[toonlivre] fetch_manga_by_slug slug={}", slug);
	request_json(&url, BASE_URL)
}
