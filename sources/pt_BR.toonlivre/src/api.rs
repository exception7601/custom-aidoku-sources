use aidoku::{
	AidokuError, Result,
	alloc::{String, Vec, format},
	imports::net::Request,
	prelude::*,
};
use serde::Deserialize;

// Load proxy host from res/proxy-server.json
// Default production: https://toons.4nd.xyz/api
// Tests always use: http://localhost:3000/api

fn get_proxy_base() -> String {
	#[cfg(test)]
	{
		// Tests always use localhost
		String::from("http://localhost:3000/api")
	}

	#[cfg(not(test))]
	{
		// Production uses remote server from proxy-server.json
		String::from("https://toons.4nd.xyz/api")
	}
}

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
	source_log!("[proxy] fetch_releases page={} limit={}", page, limit);
	let proxy_base = get_proxy_base();
	request_json(&format!(
		"{}/releases?page={}&limit={}",
		proxy_base, page, limit
	))
}

pub(crate) fn search_mangas(query: &str, page: i32, limit: i32) -> Result<ApiListResponse> {
	source_log!(
		"[proxy] search_mangas query={} page={} limit={}",
		query,
		page,
		limit
	);
	let proxy_base = get_proxy_base();
	request_json(&format!(
		"{}/search?q={}&page={}&limit={}",
		proxy_base, query, page, limit
	))
}

pub(crate) fn fetch_manga_by_id(id: &str) -> Result<ApiMangaById> {
	source_log!("[proxy] fetch_manga_by_id id={}", id);
	let proxy_base = get_proxy_base();
	request_json(&format!("{}/manga/{}", proxy_base, id))
}

pub(crate) fn fetch_manga_reader(id: &str) -> Result<ApiReaderManga> {
	source_log!("[proxy] fetch_manga_reader id={}", id);
	let proxy_base = get_proxy_base();
	request_json(&format!("{}/manga/{}/reader", proxy_base, id))
}

pub(crate) fn fetch_manga_by_slug(slug: &str) -> Result<ApiMangaBySlug> {
	source_log!("[proxy] fetch_manga_by_slug slug={}", slug);
	let proxy_base = get_proxy_base();
	request_json(&format!(
		"{}/manga-by-slug/{}",
		proxy_base,
		slug.trim_matches('/')
	))
}

pub(crate) fn fetch_chapter(manga_id: &str, chapter_id: &str) -> Result<ApiChapterDetails> {
	source_log!(
		"[proxy] fetch_chapter manga_id={} chapter_id={}",
		manga_id,
		chapter_id
	);
	let proxy_base = get_proxy_base();
	request_json(&format!(
		"{}/manga/{}/chapters/{}",
		proxy_base, manga_id, chapter_id
	))
}

fn request_json<T>(url: &str) -> Result<T>
where
	T: serde::de::DeserializeOwned,
{
	source_log!("[proxy] request_json url={}", url);
	let response = Request::get(url)?
		.header("accept", "application/json")
		.send()
		.map_err(|error| {
			AidokuError::Message(format!(
				"Proxy request failed.\nURL: {url}\nError: {error:?}"
			))
		})?;

	let status = response.status_code();
	let body = response.get_string().map_err(|error| {
		AidokuError::Message(format!(
			"Failed to read proxy response body.\nURL: {url}\nStatus: {status}\nError: {error:?}"
		))
	})?;

	if !(200..300).contains(&status) {
		bail!("Proxy request failed with status {}", status);
	}

	let response_obj: serde_json::Value = serde_json::from_str(&body).map_err(|error| {
		AidokuError::Message(format!(
			"Failed to parse proxy JSON response.\nURL: {url}\nError: {error}"
		))
	})?;

	let data = response_obj
		.get("data")
		.ok_or_else(|| AidokuError::Message(String::from("Proxy response missing data field")))?;

	serde_json::from_value(data.clone())
		.map_err(|error| AidokuError::Message(format!("Failed to parse proxy data: {error}")))
}
