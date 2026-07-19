use super::*;
use aidoku::{
	Chapter, DeepLinkHandler, DeepLinkResult, ImageRequestProvider, Manga, MangaStatus,
	PageContent, Source,
	alloc::{String, Vec},
	imports::net::Request,
};
use aidoku_test::aidoku_test;

const SAMPLE_MANGA_ID: &str = "obra-dbbabf0f";
const SAMPLE_MANGA_SLUG: &str = "contos-de-demonios-e-deuses";
const SAMPLE_MANGA_TITLE: &str = "Contos de Demônios e Deuses";
const SAMPLE_CHAPTER_ID: &str = "cap-dd9e898d-522_5";
const SAMPLE_CHAPTER_NUMBER: &str = "522.5";
const SAMPLE_MANGA_URL: &str = "https://toonlivre.net/contos-de-demonios-e-deuses";
const SAMPLE_CHAPTER_URL: &str = "https://toonlivre.net/contos-de-demonios-e-deuses/522.5";
const REMOTE_MANIFEST_URL_FOR_TESTS: &str =
	"https://exception7601.github.io/custom-aidoku-sources/manifest.json";
const REMOTE_MANIFEST_USER_AGENT_FOR_TESTS: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_0) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/127.0.0.0 Safari/537.36";

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

fn must<T, E: core::fmt::Debug>(label: &str, result: Result<T, E>) -> T {
	result.unwrap_or_else(|error| panic!("{label} failed: {error:?}"))
}

fn must_some<T>(label: &str, value: Option<T>) -> T {
	value.unwrap_or_else(|| panic!("{label} was missing"))
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
	let manifest = bundled_manifest();
	let token = request_verification_token();
	assert!(chapter_numbers_match("05", "5"));
	assert!(chapter_numbers_match("005", "5"));
	assert_eq!(manifest.schema_version, 1);
	assert_eq!(manifest.source_id, "pt_BR.toonlivre");
	assert_eq!(manifest.site_url, "https://toonlivre.net");
	assert!(manifest.request.user_agent.contains("Mozilla/5.0"));
	assert_eq!(manifest.request.accept_language, "en-US,en;q=0.9,pt;q=0.8");
	assert_eq!(
		signature_value_for_url(
			&manifest,
			"https://toonlivre.net/api/mangas/obra-dbbabf0f/chapters/cap-dd9e898d-522_5"
		),
		"t8v_authX9"
	);
	assert_eq!(
		signature_value_for_url(&manifest, SAMPLE_MANGA_URL),
		"t8v_decoy9"
	);
	assert_eq!(current_decryption_passphrase().len(), 30);
	assert_eq!(token.len(), 26);
	assert!(
		token
			.chars()
			.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit())
	);
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
fn manifest_fetches_remote_manifest() {
	let response = must(
		"remote manifest request",
		Request::get(REMOTE_MANIFEST_URL_FOR_TESTS)
			.and_then(|request| {
				request
					.header("accept", "application/json")
					.header("accept-language", ACCEPT_LANGUAGE)
					.header("user-agent", REMOTE_MANIFEST_USER_AGENT_FOR_TESTS)
					.send()
			})
			.map_err(|error| format!("request error: {error:?}")),
	);
	let status = response.status_code();
	let body = must(
		"remote manifest response body",
		response
			.get_string()
			.map_err(|error| format!("body error: {error:?}")),
	);
	assert!(
		(200..300).contains(&status),
		"remote manifest status was {status}; body: {}",
		body.chars().take(200).collect::<String>()
	);
	let manifest = must(
		"parse remote manifest json",
		serde_json::from_str::<ClientManifest>(&body).map_err(|error| {
			format!(
				"json error: {error:?}; body: {}",
				body.chars().take(200).collect::<String>()
			)
		}),
	);
	assert_eq!(manifest.schema_version, 1);
	assert_eq!(manifest.source_id, "pt_BR.toonlivre");
	assert_eq!(manifest.site_url, "https://toonlivre.net");
	assert_eq!(manifest.request.signature_header, "x-toon-signature");
	assert_eq!(manifest.request.verify_header, "x-toon-verify");
	assert_eq!(manifest.request.session_cookie.name, "toon_v");
	assert_eq!(manifest.decrypt.data_key_header, "x-toon-datakey");
}

#[aidoku_test]
fn api_fetches_public_lists() {
	let releases = must("fetch_releases", fetch_releases(1, 3));
	assert_eq!(releases.pagination.current_page, 1);
	assert!(!releases.mangas.is_empty());
	assert!(releases.mangas.iter().all(|manga| !manga.id.is_empty()));
	assert!(releases.pagination.has_next_page);

	let search = must("search_mangas", search_mangas("duque", 1, 3));
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
	let by_id = must("fetch_manga_by_id", fetch_manga_by_id(SAMPLE_MANGA_ID));
	assert_eq!(by_id.slug, SAMPLE_MANGA_SLUG);
	assert_eq!(by_id.title, SAMPLE_MANGA_TITLE);
	assert_eq!(by_id.status.as_deref(), Some("Ongoing"));

	let reader = must("fetch_manga_reader", fetch_manga_reader(SAMPLE_MANGA_ID));
	assert!(reader.chapters.len() > 100);
	assert!(
		reader
			.chapters
			.iter()
			.any(|chapter| chapter.id == SAMPLE_CHAPTER_ID)
	);

	let by_slug = must(
		"fetch_manga_by_slug",
		fetch_manga_by_slug(SAMPLE_MANGA_SLUG),
	);
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
	let chapter = must(
		"fetch_chapter",
		fetch_chapter(SAMPLE_MANGA_ID, SAMPLE_CHAPTER_ID),
	);
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
	let home = must(
		"get_search_manga_list home",
		source.get_search_manga_list(None, 1, Vec::new()),
	);
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

	let search = must(
		"get_search_manga_list search",
		source.get_search_manga_list(Some(String::from("duque")), 1, Vec::new()),
	);
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
	let updated = must(
		"get_manga_update by id",
		source.get_manga_update(make_id_manga(), true, true),
	);

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
	let updated = must(
		"get_manga_update by slug",
		source.get_manga_update(make_slug_manga(), true, true),
	);

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
	let pages = must(
		"get_page_list by ids",
		source.get_page_list(make_id_manga(), make_id_chapter()),
	);
	assert!(!pages.is_empty());
	assert!(pages.iter().all(|page| match &page.content {
		PageContent::Url(url, _) => url.starts_with("https://cdn.toonlivre.net/obras/"),
		_ => false,
	}));
}

#[aidoku_test]
fn source_maps_page_list_from_slug_and_number() {
	let source = ToonLivre::new();
	let pages = must(
		"get_page_list by slug and number",
		source.get_page_list(make_slug_manga(), make_number_chapter()),
	);
	assert!(!pages.is_empty());
	assert!(pages.iter().all(|page| match &page.content {
		PageContent::Url(url, _) => url.starts_with("https://cdn.toonlivre.net/obras/"),
		_ => false,
	}));
}

#[aidoku_test]
fn source_provides_image_requests() {
	let source = ToonLivre::new();
	let pages = must(
		"get_page_list for image request",
		source.get_page_list(make_id_manga(), make_id_chapter()),
	);
	let first = must_some(
		"first page URL",
		pages.into_iter().find_map(|page| match page.content {
			PageContent::Url(url, context) => Some((url, context)),
			_ => None,
		}),
	);
	let request = must(
		"get_image_request",
		source.get_image_request(first.0, first.1),
	);
	let response = must("image request send", request.send());
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

	match must(
		"handle_deep_link manga",
		source.handle_deep_link(String::from(SAMPLE_MANGA_URL)),
	) {
		Some(DeepLinkResult::Manga { key }) => assert_eq!(key, SAMPLE_MANGA_SLUG),
		_ => panic!("expected manga deep link"),
	}

	match must(
		"handle_deep_link chapter",
		source.handle_deep_link(String::from(SAMPLE_CHAPTER_URL)),
	) {
		Some(DeepLinkResult::Chapter { manga_key, key }) => {
			assert_eq!(manga_key, SAMPLE_MANGA_SLUG);
			assert_eq!(key, SAMPLE_CHAPTER_NUMBER);
		}
		_ => panic!("expected chapter deep link"),
	}
}
