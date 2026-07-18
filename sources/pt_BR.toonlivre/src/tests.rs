use super::*;
use aidoku::{
	Chapter, DeepLinkHandler, DeepLinkResult, ImageRequestProvider, Manga, MangaStatus,
	PageContent, Source,
	alloc::{String, Vec},
};
use aidoku_test::aidoku_test;

const SAMPLE_MANGA_ID: &str = "obra-dbbabf0f";
const SAMPLE_MANGA_SLUG: &str = "contos-de-demonios-e-deuses";
const SAMPLE_MANGA_TITLE: &str = "Contos de Demônios e Deuses";
const SAMPLE_CHAPTER_ID: &str = "cap-dd9e898d-522_5";
const SAMPLE_CHAPTER_NUMBER: &str = "522.5";
const SAMPLE_MANGA_URL: &str = "https://toonlivre.net/contos-de-demonios-e-deuses";
const SAMPLE_CHAPTER_URL: &str = "https://toonlivre.net/contos-de-demonios-e-deuses/522.5";

fn make_id_manga() -> Manga {
	Manga {
		key: String::from(SAMPLE_MANGA_ID),
		title: String::from(SAMPLE_MANGA_TITLE),
		url: Some(String::from(SAMPLE_MANGA_URL)),
		..Default::default()
	}
}

fn make_slug_manga() -> Manga {
	Manga {
		key: String::from(SAMPLE_MANGA_SLUG),
		title: String::from(SAMPLE_MANGA_TITLE),
		url: Some(String::from(SAMPLE_MANGA_URL)),
		..Default::default()
	}
}

fn make_id_chapter() -> Chapter {
	Chapter {
		key: String::from(SAMPLE_CHAPTER_ID),
		url: Some(String::from(SAMPLE_CHAPTER_URL)),
		..Default::default()
	}
}

fn make_number_chapter() -> Chapter {
	Chapter {
		key: String::from(SAMPLE_CHAPTER_NUMBER),
		url: Some(String::from(SAMPLE_CHAPTER_URL)),
		..Default::default()
	}
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
	assert_eq!(current_decryption_passphrase().len(), 30);
	assert_eq!(request_verification_token().len(), 26);
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
			assert_eq!(key, SAMPLE_CHAPTER_NUMBER);
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
fn api_fetches_public_lists() {
	let releases = fetch_releases(1, 3).unwrap();
	assert_eq!(releases.pagination.current_page, 1);
	assert!(!releases.mangas.is_empty());
	assert!(releases.mangas.iter().all(|manga| !manga.id.is_empty()));
	assert!(releases.pagination.has_next_page);

	let search = search_mangas("duque", 1, 3).unwrap();
	assert!(!search.mangas.is_empty());
	assert!(search.mangas.iter().any(|manga| {
		manga.title.to_lowercase().contains("duque")
			|| manga
				.alternative_title
				.as_deref()
				.unwrap_or_default()
				.to_lowercase()
				.contains("duke")
	}));
}

#[aidoku_test]
fn api_fetches_manga_and_reader_data() {
	let by_id = fetch_manga_by_id(SAMPLE_MANGA_ID).unwrap();
	assert_eq!(by_id.slug, SAMPLE_MANGA_SLUG);
	assert_eq!(by_id.title, SAMPLE_MANGA_TITLE);
	assert_eq!(by_id.status.as_deref(), Some("Ongoing"));

	let reader = fetch_manga_reader(SAMPLE_MANGA_ID).unwrap();
	assert!(reader.chapters.len() > 100);
	assert!(
		reader
			.chapters
			.iter()
			.any(|chapter| chapter.id == SAMPLE_CHAPTER_ID)
	);

	let by_slug = fetch_manga_by_slug(SAMPLE_MANGA_SLUG).unwrap();
	assert_eq!(by_slug.id, SAMPLE_MANGA_ID);
	assert!(by_slug.chapters.len() > 100);
	assert!(
		by_slug
			.chapters
			.iter()
			.any(|chapter| chapter.id == SAMPLE_CHAPTER_ID)
	);
}

#[aidoku_test]
fn api_fetches_and_decrypts_chapter_pages() {
	let chapter = fetch_chapter(SAMPLE_MANGA_ID, SAMPLE_CHAPTER_ID).unwrap();
	assert_eq!(chapter.id, SAMPLE_CHAPTER_ID);
	assert_eq!(chapter.manga_id, SAMPLE_MANGA_ID);
	assert_eq!(chapter.number, SAMPLE_CHAPTER_NUMBER);
	assert!(!chapter.pages.is_empty());
	assert!(
		chapter
			.pages
			.iter()
			.all(|url| url.starts_with("https://cdn.toonlivre.net/obras/"))
	);
	assert!(chapter.timestamp > 0);
	assert!(!chapter.release_date.is_empty());
}

#[aidoku_test]
fn source_maps_home_and_search_entries() {
	let source = ToonLivre::new();
	let home = source.get_search_manga_list(None, 1, Vec::new()).unwrap();
	assert!(!home.entries.is_empty());
	assert!(home.has_next_page);
	assert!(
		home.entries
			.iter()
			.all(|entry| entry.key.starts_with("obra-"))
	);
	assert!(
		home.entries
			.iter()
			.all(|entry| entry.viewer == aidoku::Viewer::Vertical)
	);

	let search = source
		.get_search_manga_list(Some(String::from("duque")), 1, Vec::new())
		.unwrap();
	assert!(!search.entries.is_empty());
	assert!(
		search
			.entries
			.iter()
			.any(|entry| entry.title.to_lowercase().contains("duque"))
	);
}

#[aidoku_test]
fn source_maps_manga_details_and_chapters_from_id() {
	let source = ToonLivre::new();
	let updated = source
		.get_manga_update(make_id_manga(), true, true)
		.unwrap();

	assert_eq!(updated.key, SAMPLE_MANGA_ID);
	assert_eq!(updated.title, SAMPLE_MANGA_TITLE);
	assert!(updated.url.as_deref() == Some(SAMPLE_MANGA_URL));
	assert!(updated.viewer == aidoku::Viewer::Vertical);
	assert!(updated.status == MangaStatus::Ongoing);
	assert!(
		updated
			.description
			.as_deref()
			.unwrap_or_default()
			.contains("Tales of Demons and Gods")
	);
	let chapters = updated.chapters.unwrap_or_default();
	assert!(chapters.len() > 100);
	assert!(
		chapters
			.iter()
			.any(|chapter| chapter.key == SAMPLE_CHAPTER_ID)
	);
	assert!(chapters.iter().all(|chapter| chapter.url.is_some()));
}

#[aidoku_test]
fn source_maps_manga_details_and_chapters_from_slug() {
	let source = ToonLivre::new();
	let updated = source
		.get_manga_update(make_slug_manga(), true, true)
		.unwrap();

	assert_eq!(updated.key, SAMPLE_MANGA_ID);
	assert_eq!(updated.title, SAMPLE_MANGA_TITLE);
	assert!(updated.url.as_deref() == Some(SAMPLE_MANGA_URL));
	let chapters = updated.chapters.unwrap_or_default();
	assert!(chapters.len() > 100);
	assert!(
		chapters
			.iter()
			.any(|chapter| chapter.key == SAMPLE_CHAPTER_ID)
	);
}

#[aidoku_test]
fn source_maps_page_list_from_ids() {
	let source = ToonLivre::new();
	let pages = source
		.get_page_list(make_id_manga(), make_id_chapter())
		.unwrap();
	assert!(!pages.is_empty());
	assert!(pages.iter().all(|page| match &page.content {
		PageContent::Url(url, _) => url.starts_with("https://cdn.toonlivre.net/obras/"),
		_ => false,
	}));
}

#[aidoku_test]
fn source_maps_page_list_from_slug_and_number() {
	let source = ToonLivre::new();
	let pages = source
		.get_page_list(make_slug_manga(), make_number_chapter())
		.unwrap();
	assert!(!pages.is_empty());
	assert!(pages.iter().all(|page| match &page.content {
		PageContent::Url(url, _) => url.starts_with("https://cdn.toonlivre.net/obras/"),
		_ => false,
	}));
}

#[aidoku_test]
fn source_provides_image_requests() {
	let source = ToonLivre::new();
	let pages = source
		.get_page_list(make_id_manga(), make_id_chapter())
		.unwrap();
	let first = pages
		.into_iter()
		.find_map(|page| match page.content {
			PageContent::Url(url, context) => Some((url, context)),
			_ => None,
		})
		.unwrap();
	let request = source.get_image_request(first.0, first.1).unwrap();
	let response = request.send().unwrap();
	assert_eq!(response.status_code(), 200);
	assert!(
		response
			.get_header("content-type")
			.unwrap_or_default()
			.to_lowercase()
			.contains("image/")
	);
}

#[aidoku_test]
fn source_handles_deep_links() {
	let source = ToonLivre::new();

	match source
		.handle_deep_link(String::from(SAMPLE_MANGA_URL))
		.unwrap()
	{
		Some(DeepLinkResult::Manga { key }) => assert_eq!(key, SAMPLE_MANGA_SLUG),
		_ => panic!("expected manga deep link"),
	}

	match source
		.handle_deep_link(String::from(SAMPLE_CHAPTER_URL))
		.unwrap()
	{
		Some(DeepLinkResult::Chapter { manga_key, key }) => {
			assert_eq!(manga_key, SAMPLE_MANGA_SLUG);
			assert_eq!(key, SAMPLE_CHAPTER_NUMBER);
		}
		_ => panic!("expected chapter deep link"),
	}
}
