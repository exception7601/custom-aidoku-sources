use aidoku::{
	AidokuError, Result,
	alloc::{String, Vec, format},
	imports::net::Request,
};
use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::{ACCEPT_LANGUAGE, BASE_URL, percent_encode};

const API_BASE: &str = "https://nexustoons.com/api";
const DIRECT_USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/151.0.0.0 Safari/537.36";

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ApiListResponse {
	pub data: Vec<ApiMangaCard>,
	pub page: i32,
	pub pages: i32,
	pub total: i32,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ApiChapter {
	pub id: i64,
	pub number: String,
	#[serde(default)]
	pub title: Option<String>,
	#[serde(default, rename = "createdAt", alias = "created_at")]
	pub timestamp: Option<String>,
	#[serde(default, rename = "pageCount", alias = "page_count")]
	pub page_count: Option<i64>,
	#[serde(default, rename = "releaseStatus", alias = "release_status")]
	pub release_status: Option<String>,
	#[serde(default, rename = "scanGroups", alias = "scan_groups")]
	pub scan_groups: Option<Vec<serde_json::Value>>,
	#[serde(default, rename = "mangaId", alias = "manga_id")]
	pub manga_id: Option<i64>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ApiMangaCard {
	pub id: i64,
	pub title: String,
	#[serde(default)]
	pub slug: Option<String>,
	#[serde(
		default,
		rename = "coverUrl",
		alias = "cover_url",
		alias = "coverImage"
	)]
	pub cover_url: Option<String>,
	#[serde(default, rename = "alternativeTitle", alias = "alternative_title")]
	pub alternative_title: Option<String>,
	#[serde(
		default,
		rename = "recentChapters",
		alias = "recent_chapters",
		alias = "chapters"
	)]
	pub recent_chapters: Option<Vec<ApiChapter>>,
	#[serde(default)]
	pub authors: Option<Vec<String>>,
	#[serde(default)]
	pub artists: Option<Vec<String>>,
	#[serde(default)]
	pub genres: Option<Vec<String>>,
	#[serde(default)]
	pub status: Option<String>,
}

fn request_json<T>(url: &str, referer: &str) -> Result<T>
where
	T: DeserializeOwned,
{
	source_log!("[nexustoons] request_json url={} referer={}", url, referer);
	let response = send_request(url, referer)?;
	let status = response.status_code();
	let body = response.get_string().map_err(|error| {
		AidokuError::Message(format!(
			"Failed to read response body.\nURL: {url}\nStatus: {status}\nError: {error:?}"
		))
	})?;
	if !(200..300).contains(&status) {
		return Err(response_error(url, status, &body));
	}
	serde_json::from_str(&body).map_err(|error| {
		AidokuError::Message(format!(
			"Failed to parse JSON response.\nURL: {url}\nError: {error}"
		))
	})
}

fn response_error(url: &str, status: i32, body: &str) -> AidokuError {
	let message = serde_json::from_str::<serde_json::Value>(body)
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

fn send_request(url: &str, referer: &str) -> Result<aidoku::imports::net::Response> {
	Request::get(url)?
		.header("Accept", "application/json, text/plain, */*")
		.header("User-Agent", DIRECT_USER_AGENT)
		.header("Accept-Language", ACCEPT_LANGUAGE)
		.header("Referer", referer)
		.send()
		.map_err(|error| {
			AidokuError::Message(format!("Request failed.\nURL: {url}\nError: {error:?}"))
		})
}

pub(crate) fn fetch_releases(page: i32, limit: i32) -> Result<ApiListResponse> {
	let url = format!("{API_BASE}/mangas?page={page}&limit={limit}");
	source_log!(
		"[nexustoons] fetch_releases page={} limit={} url={}",
		page,
		limit,
		url
	);
	request_json(&url, BASE_URL)
}

pub(crate) fn search_mangas(query: &str, page: i32, limit: i32) -> Result<ApiListResponse> {
	let encoded = percent_encode(query.trim());
	let url = format!("{API_BASE}/mangas?search={encoded}&page={page}&limit={limit}");
	source_log!(
		"[nexustoons] search_mangas query={} page={} limit={} url={}",
		query,
		page,
		limit,
		url
	);
	request_json(&url, BASE_URL)
}
