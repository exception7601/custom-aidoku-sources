use super::*;
use aidoku::{DeepLinkHandler, DeepLinkResult, alloc::String};
use aidoku_test::aidoku_test;

const SAMPLE_MANGA_URL: &str = "https://toonlivre.net/fumando-nos-fundos-do-supermercado-com-voce";
const SAMPLE_MANGA_SLUG: &str = "fumando-nos-fundos-do-supermercado-com-voce";
const SAMPLE_MANGA_ID: &str = "obra-d398da67";
const SAMPLE_CHAPTER_URL: &str =
	"https://toonlivre.net/fumando-nos-fundos-do-supermercado-com-voce/01";
const SAMPLE_CHAPTER_ID: &str = "cap-d0b0082c-01";

#[aidoku_test]
fn helper_slugifies_titles_and_formats_chapters() {
	assert_eq!(
		slugify_title("Técnica do Deus Marcial da Estrela"),
		"tecnica-do-deus-marcial-da-estrela"
	);
	assert_eq!(
		slugify_title("Domador de Bestas - Vejo Todas as Evoluções"),
		"domador-de-bestas-vejo-todas-as-evolucoes"
	);
	assert_eq!(chapter_segment("5"), "05");
	assert_eq!(chapter_segment("05"), "05");
	assert_eq!(chapter_segment("105"), "105");
	assert_eq!(chapter_segment("522.5"), "522.5");

	assert!(chapter_numbers_match("05", "5"));
	assert!(chapter_numbers_match("005", "5"));
}

#[aidoku_test]
fn helper_parses_deep_links() {
	match deep_link_result(SAMPLE_MANGA_URL) {
		Some(DeepLinkResult::Manga { key }) => assert_eq!(key, SAMPLE_MANGA_SLUG),
		_ => panic!("expected manga deep link"),
	}

	match deep_link_result(SAMPLE_CHAPTER_URL) {
		Some(DeepLinkResult::Chapter { manga_key, key }) => {
			assert_eq!(manga_key, SAMPLE_MANGA_SLUG);
			assert_eq!(key, "01");
		}
		_ => panic!("expected chapter deep link"),
	}

	match deep_link_result(
		"https://toonlivre.net/read/fumando-nos-fundos-do-supermercado-com-voce/obra-d398da67/cap-d0b0082c-01",
	) {
		Some(DeepLinkResult::Chapter { manga_key, key }) => {
			assert_eq!(manga_key, SAMPLE_MANGA_ID);
			assert_eq!(key, SAMPLE_CHAPTER_ID);
		}
		_ => panic!("expected reader deep link"),
	}

	assert!(deep_link_result("https://toonlivre.net/favorites").is_none());
}

#[aidoku_test]
fn source_handles_deep_links() {
	let source = ToonLivre::new();

	match source
		.handle_deep_link(String::from(SAMPLE_MANGA_URL))
		.expect("handle_deep_link should succeed")
	{
		Some(DeepLinkResult::Manga { key }) => assert_eq!(key, SAMPLE_MANGA_SLUG),
		_ => panic!("expected manga deep link"),
	}

	match source
		.handle_deep_link(String::from(SAMPLE_CHAPTER_URL))
		.expect("handle_deep_link should succeed")
	{
		Some(DeepLinkResult::Chapter { manga_key, key }) => {
			assert_eq!(manga_key, SAMPLE_MANGA_SLUG);
			assert_eq!(key, "01");
		}
		_ => panic!("expected chapter deep link"),
	}
}

#[aidoku_test]
fn helper_parses_webview_chapter_cache() {
	let storage_key = webview_chapter_storage_key(SAMPLE_MANGA_ID, SAMPLE_CHAPTER_ID);
	assert_eq!(
		storage_key,
		"toonlivre_chapter_cache_v1:obra-d398da67:cap-d0b0082c-01"
	);

	let raw = r#"{
		"savedAt": 1730000000000,
		"chapter": {
			"id": "cap-d0b0082c-01",
			"pages": [
				"page-1.webp",
				"page-2.webp"
			],
			"title": "Chapter 1",
			"number": "01",
			"mangaId": "obra-d398da67",
			"timestamp": 1730000000,
			"releaseDate": "2024-11-01"
		}
	}"#;
	let chapter = parse_webview_chapter_cache(raw, &storage_key)
		.expect("parse_webview_chapter_cache should succeed");
	assert_eq!(chapter.id, SAMPLE_CHAPTER_ID);
	assert_eq!(chapter.manga_id, SAMPLE_MANGA_ID);
	assert_eq!(chapter.pages.len(), 2);

	let empty_pages = r#"{
		"savedAt": 1730000000,
		"chapter": {
			"id": "cap-d0b0082c-01",
			"pages": [],
			"title": "Chapter 1",
			"number": "01",
			"mangaId": "obra-d398da67"
		}
	}"#;
	assert!(parse_webview_chapter_cache(empty_pages, &storage_key).is_err());
}

// Live integration tests (require network access to toonlivre.net)

#[aidoku_test(live:test)]
fn live_fetch_releases() {
	let result = api::fetch_releases(1, 3);

	if let Err(ref e) = result {
		source_log!("[toonlivre] fetch_releases error: {:?}", e);
	}

	assert!(result.is_ok(), "fetch_releases should succeed");

	let response = result.unwrap();
	assert!(!response.mangas.is_empty(), "should return mangas");
	assert_eq!(response.pagination.current_page, 1);
}

#[aidoku_test(live:test)]
fn live_search_mangas() {
	let result = api::search_mangas("duque", 1, 3);
	assert!(result.is_ok(), "search_mangas should succeed");

	let response = result.unwrap();
	assert!(!response.mangas.is_empty(), "should return search results");
}

#[aidoku_test(live:test)]
fn live_fetch_manga_by_slug() {
	let result = api::fetch_manga_by_slug(SAMPLE_MANGA_SLUG);

	if let Err(ref e) = result {
		source_log!("[toonlivre] fetch_manga_by_slug error: {:?}", e);
	}

	assert!(result.is_ok(), "fetch_manga_by_slug should succeed");

	let manga = result.unwrap();
	assert_eq!(manga.id, SAMPLE_MANGA_ID);
	assert!(!manga.title.is_empty());
	assert!(!manga.chapters.is_empty());
}

#[aidoku_test(live:test)]
fn live_fetch_manga_by_id() {
	let result = api::fetch_manga_by_id(SAMPLE_MANGA_ID);

	if let Err(ref e) = result {
		source_log!("[toonlivre] fetch_manga_by_id error: {:?}", e);
	}

	assert!(result.is_ok(), "fetch_manga_by_id should succeed");

	let manga = result.unwrap();
	assert_eq!(manga.id, SAMPLE_MANGA_ID);
	assert_eq!(manga.slug, SAMPLE_MANGA_SLUG);
}

#[aidoku_test(live:test)]
fn live_fetch_manga_reader() {
	let result = api::fetch_manga_reader(SAMPLE_MANGA_ID);

	if let Err(ref e) = result {
		source_log!("[toonlivre] fetch_manga_reader error: {:?}", e);
	}

	assert!(result.is_ok(), "fetch_manga_reader should succeed");

	let manga = result.unwrap();
	assert_eq!(manga.id, SAMPLE_MANGA_ID);
	assert!(!manga.chapters.is_empty());
}
