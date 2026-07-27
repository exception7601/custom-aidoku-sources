use super::*;
use aidoku::{DeepLinkHandler, DeepLinkResult, alloc::String};
use aidoku_test::aidoku_test;

const SAMPLE_MANGA_URL: &str = "https://toonlivre.net/contos-de-demonios-e-deuses";
const SAMPLE_MANGA_SLUG: &str = "contos-de-demonios-e-deuses";
const SAMPLE_MANGA_ID: &str = "obra-dbbabf0f";
const SAMPLE_CHAPTER_ID: &str = "cap-dd9e898d-522_5";

fn generate_random_chapter() -> String {
	let chapter_num = ((13 * 17) % 522) + 1;
	format!("contos-de-demonios-e-deuses/{}", chapter_num)
}

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
	let random_chapter = generate_random_chapter();
	let chapter_url = format!("https://toonlivre.net/{}", random_chapter);

	match deep_link_result(SAMPLE_MANGA_URL) {
		Some(DeepLinkResult::Manga { key }) => assert_eq!(key, SAMPLE_MANGA_SLUG),
		_ => panic!("expected manga deep link"),
	}

	match deep_link_result(&chapter_url) {
		Some(DeepLinkResult::Chapter { manga_key, key: _ }) => {
			assert_eq!(manga_key, SAMPLE_MANGA_SLUG);
		}
		_ => panic!("expected chapter deep link"),
	}

	match deep_link_result(
		"https://toonlivre.net/read/contos-de-demonios-e-deuses/obra-dbbabf0f/cap-dd9e898d-522_5",
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

	let random_chapter = generate_random_chapter();
	let chapter_url = format!("https://toonlivre.net/{}", random_chapter);

	match source
		.handle_deep_link(String::from(&chapter_url))
		.expect("handle_deep_link should succeed")
	{
		Some(DeepLinkResult::Chapter { manga_key, key: _ }) => {
			assert_eq!(manga_key, SAMPLE_MANGA_SLUG);
		}
		_ => panic!("expected chapter deep link"),
	}
}

// Live integration tests (require proxy server running on localhost:4000)

#[aidoku_test(live:test)]
fn live_proxy_server_health() {
	let result = aidoku::imports::net::Request::get("http://localhost:4000/health")
		.and_then(|req| req.send());

	if let Err(ref e) = result {
		source_log!("[proxy] health check failed: {:?}", e);
	}

	assert!(result.is_ok(), "proxy health endpoint should be reachable");

	let response = result.unwrap();
	assert_eq!(
		response.status_code(),
		200,
		"proxy health endpoint should return 200"
	);
}

#[aidoku_test(live:test)]
fn live_fetch_releases() {
	let result = api::fetch_releases(1, 3);

	if let Err(ref e) = result {
		source_log!("[proxy] fetch_releases error: {:?}", e);
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
		source_log!("[proxy] fetch_manga_by_slug error: {:?}", e);
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
		source_log!("[proxy] fetch_manga_by_id error: {:?}", e);
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
		source_log!("[proxy] fetch_manga_reader error: {:?}", e);
	}

	assert!(result.is_ok(), "fetch_manga_reader should succeed");

	let manga = result.unwrap();
	assert_eq!(manga.id, SAMPLE_MANGA_ID);
	assert!(!manga.chapters.is_empty());
}
