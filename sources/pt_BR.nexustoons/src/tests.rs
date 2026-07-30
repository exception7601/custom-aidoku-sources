use super::*;
use crate::{
	map_list_response,
	source::{chapter_from_api, manga_from_card},
};
use aidoku::{
	DeepLinkResult, HomeComponentValue, ImageRequestProvider, Source,
	alloc::{String, Vec},
};
use aidoku_test::aidoku_test;

const SAMPLE_MANGA_SLUG: &str = "sono-akuyaku-kizoku-mama-heroine-ga-suki-sugiru-shinshi-na-doryoku-de-saikyou-to-nari-fuguu-na-oshi-chara-tasukemakuru";
const SAMPLE_MANGA_URL: &str = "https://nexustoons.com/manga/sono-akuyaku-kizoku-mama-heroine-ga-suki-sugiru-shinshi-na-doryoku-de-saikyou-to-nari-fuguu-na-oshi-chara-tasukemakuru";
const SAMPLE_CHAPTER_URL: &str = "https://nexustoons.com/ler/sono-akuyaku-kizoku-mama-heroine-ga-suki-sugiru-shinshi-na-doryoku-de-saikyou-to-nari-fuguu-na-oshi-chara-tasukemakuru/397924";

#[aidoku_test]
fn helper_builds_urls_and_parses_links() {
	assert_eq!(manga_url_from_slug(SAMPLE_MANGA_SLUG), SAMPLE_MANGA_URL);
	assert_eq!(
		manga_url_from_slug(&format!("  /{}  ", SAMPLE_MANGA_SLUG)),
		SAMPLE_MANGA_URL
	);
	assert_eq!(
		chapter_url_from_slug_and_id(SAMPLE_MANGA_SLUG, "397924"),
		SAMPLE_CHAPTER_URL
	);
	assert_eq!(
		chapter_url_from_slug_and_id(&format!("  /{}  ", SAMPLE_MANGA_SLUG), "  /397924/  "),
		SAMPLE_CHAPTER_URL
	);
	assert_eq!(
		manga_slug_from_url(SAMPLE_MANGA_URL),
		Some(String::from(SAMPLE_MANGA_SLUG))
	);
	assert_eq!(
		chapter_id_from_url(SAMPLE_CHAPTER_URL),
		Some(String::from("397924"))
	);

	match deep_link_result(SAMPLE_MANGA_URL) {
		Some(DeepLinkResult::Manga { key }) => assert_eq!(key, SAMPLE_MANGA_SLUG),
		_ => panic!("expected manga deep link"),
	}

	match deep_link_result(SAMPLE_CHAPTER_URL) {
		Some(DeepLinkResult::Chapter { manga_key, key }) => {
			assert_eq!(manga_key, SAMPLE_MANGA_SLUG);
			assert_eq!(key, "397924");
		}
		_ => panic!("expected chapter deep link"),
	}
}

#[aidoku_test]
fn helper_maps_card_to_manga() {
	let card = ApiMangaCard {
		id: 758,
		title: String::from("Test Manga"),
		slug: Some(String::from("test-manga")),
		cover_url: Some(String::from("https://example.com/cover.jpg")),
		alternative_title: Some(String::from("Alt title")),
		recent_chapters: Some(Vec::new()),
		authors: Some(Vec::new()),
		artists: Some(Vec::new()),
		genres: Some(Vec::new()),
		status: Some(String::from("ongoing")),
	};
	let manga = manga_from_card(&card);
	assert_eq!(manga.key, "test-manga");
	assert_eq!(manga.title, "Test Manga");
	assert_eq!(
		manga.cover.as_deref(),
		Some("https://example.com/cover.jpg")
	);
	assert_eq!(
		manga.url.as_deref(),
		Some("https://nexustoons.com/manga/test-manga")
	);
}

#[aidoku_test]
fn helper_maps_chapter_to_url_and_title() {
	let chapter = ApiChapter {
		id: 397924,
		number: String::from("45"),
		title: Some(String::new()),
		timestamp: Some(String::from("2025-07-29T21:01:46Z")),
		page_count: Some(18),
		release_status: None,
		scan_groups: None,
		manga_id: None,
	};
	let mapped = chapter_from_api(&chapter, SAMPLE_MANGA_SLUG);
	assert_eq!(mapped.key, "397924");
	assert_eq!(mapped.title.as_deref(), Some("Capítulo 45"));
	assert_eq!(mapped.url.as_deref(), Some(SAMPLE_CHAPTER_URL));
	assert_eq!(mapped.chapter_number, Some(45.0));
}

#[aidoku_test]
fn helper_maps_list_response() {
	let response: ApiListResponse = serde_json::from_str(
		r#"{
			"data": [
				{
					"id": 758,
					"title": "Test Manga",
					"slug": "test-manga",
					"coverUrl": "https://example.com/cover.jpg",
					"alternativeTitle": "Alt title",
					"recentChapters": []
				}
			],
			"limit": 50,
			"page": 1,
			"pages": 2,
			"total": 1
		}"#,
	)
	.expect("response should deserialize");
	let mapped = map_list_response(&response);
	assert_eq!(mapped.entries.len(), 1);
	assert!(mapped.has_next_page);
	assert_eq!(mapped.entries[0].key, "test-manga");
	assert_eq!(mapped.entries[0].title, "Test Manga");
	assert_eq!(mapped.entries[0].description.as_deref(), Some("Alt title"));
}

#[aidoku_test]
fn helper_extracts_page_urls_from_json() {
	let pages: Vec<String> = serde_json::from_str(
		r#"[
			"https://img.nx-toons.xyz/manga_pages/758/73801/page_001.webp",
			"https://img.nx-toons.xyz/manga_pages/758/73801/page_002.webp"
		]"#,
	)
	.expect("pages should deserialize");
	assert_eq!(pages.len(), 2);
	assert!(pages[0].contains("manga_pages"));
}

// Live integration tests.

#[aidoku_test(live:test)]
fn live_fetch_releases() {
	let result = fetch_releases(1, 5);
	if let Err(ref error) = result {
		source_log!("[nexustoons] fetch_releases error: {:?}", error);
	}
	assert!(result.is_ok(), "fetch_releases should succeed");
	let response = result.unwrap();
	assert!(!response.data.is_empty(), "should return manga entries");
}

#[aidoku_test(live:test)]
fn live_search_mangas() {
	let result = search_mangas("sono akuyaku kizoku", 1, 5);
	if let Err(ref error) = result {
		source_log!("[nexustoons] search_mangas error: {:?}", error);
	}
	assert!(result.is_ok(), "search_mangas should succeed");
	let response = result.unwrap();
	assert!(!response.data.is_empty(), "should return search results");
	assert_eq!(response.page, 1);
	assert!(response.pages >= 1);
	assert_eq!(response.data[0].slug.as_deref(), Some(SAMPLE_MANGA_SLUG));
}

#[aidoku_test(live:test)]
fn live_search_mangas_next_page() {
	let result = search_mangas("solo leveling", 2, 5);
	if let Err(ref error) = result {
		source_log!("[nexustoons] search_mangas page 2 error: {:?}", error);
	}
	assert!(result.is_ok(), "search_mangas page 2 should succeed");
	let response = result.unwrap();
	assert_eq!(response.page, 2);
	assert!(response.pages >= 1);
}

#[aidoku_test(live:test)]
fn source_builds_home_and_search_entries() {
	let source = NexusToons::new();
	let home = source.get_home().expect("get_home should succeed");
	assert_eq!(home.components.len(), 3);
	assert_eq!(home.components[0].title.as_deref(), Some("Lançamentos"));
	assert_eq!(
		home.components[1].title.as_deref(),
		Some("Capítulos recentes")
	);
	assert_eq!(home.components[2].title.as_deref(), Some("Mais obras"));
	match &home.components[0].value {
		HomeComponentValue::BigScroller { entries, .. } => assert!(!entries.is_empty()),
		_ => panic!("home should use BigScroller"),
	}
	match &home.components[1].value {
		HomeComponentValue::MangaChapterList { entries, .. } => assert!(!entries.is_empty()),
		_ => panic!("home should use MangaChapterList"),
	}
	match &home.components[2].value {
		HomeComponentValue::MangaList { entries, .. } => assert!(!entries.is_empty()),
		_ => panic!("home should use MangaList"),
	}

	let search = source
		.get_search_manga_list(Some(String::from("sono akuyaku kizoku")), 1, Vec::new())
		.expect("get_search_manga_list should succeed");
	assert!(!search.entries.is_empty(), "search should return entries");
}

#[aidoku_test(live:test)]
fn live_download_cover_image() {
	let result = search_mangas("solo leveling", 1, 1);
	if let Err(ref error) = result {
		source_log!("[nexustoons] search_mangas error: {:?}", error);
	}
	assert!(result.is_ok(), "search_mangas should succeed");
	let response = result.unwrap();
	let cover_url = response
		.data
		.first()
		.and_then(|manga| manga.cover_url.clone())
		.expect("search result should have cover url");
	let source = NexusToons::new();
	let request = source
		.get_image_request(cover_url.clone(), None)
		.expect("get_image_request should succeed");
	let response = request.send().expect("cover image request should succeed");
	let status = response.status_code();
	let content_type = response.get_header("content-type").unwrap_or_default();
	let bytes = response
		.get_data()
		.expect("cover image bytes should be readable");
	source_log!(
		"[nexustoons] live_download_cover_image url={} status={} content_type={} bytes={}",
		cover_url,
		status,
		content_type,
		bytes.len()
	);
	assert_eq!(status, 200);
	assert!(content_type.contains("image/"));
	assert!(!bytes.is_empty(), "cover image should download real bytes");
}
