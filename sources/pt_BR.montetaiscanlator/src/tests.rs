use super::*;
use aidoku::{
	Chapter, DeepLinkHandler, DeepLinkResult, ImageRequestProvider, Manga, MangaStatus,
	PageContent, Source,
	alloc::{String, Vec},
	imports::net::Request,
};
use aidoku_test::aidoku_test;

const SAMPLE_MANGA_KEY: &str = "o-filho-do-duque-regressado-e-um-assassino";
const SAMPLE_MANGA_URL: &str =
	"https://montetaiscanlator.xyz/manga/o-filho-do-duque-regressado-e-um-assassino/";
const SAMPLE_CHAPTER_KEY: &str = "manga/o-filho-do-duque-regressado-e-um-assassino/capitulo-126";
const SAMPLE_CHAPTER_URL: &str =
	"https://montetaiscanlator.xyz/manga/o-filho-do-duque-regressado-e-um-assassino/capitulo-126/";
const BASKERVILLE_MANGA_KEY: &str = "a-vinganca-do-cao-de-caca-dos-baskerville";
const BASKERVILLE_MANGA_URL: &str =
	"https://montetaiscanlator.xyz/manga/a-vinganca-do-cao-de-caca-dos-baskerville/";
const BASKERVILLE_CHAPTER_KEY: &str =
	"manga/a-vinganca-do-cao-de-caca-dos-baskerville/capitulo-123";
const BASKERVILLE_CHAPTER_URL: &str =
	"https://montetaiscanlator.xyz/manga/a-vinganca-do-cao-de-caca-dos-baskerville/capitulo-123/";

fn make_manga() -> Manga {
	Manga {
		key: SAMPLE_MANGA_KEY.into(),
		url: Some(SAMPLE_MANGA_URL.into()),
		..Default::default()
	}
}

fn make_chapter() -> Chapter {
	Chapter {
		key: SAMPLE_CHAPTER_KEY.into(),
		url: Some(SAMPLE_CHAPTER_URL.into()),
		..Default::default()
	}
}

#[aidoku_test]
fn helper_url_and_key_mapping() {
	assert_eq!(home_url(1), BASE_URL);
	assert_eq!(home_url(3), format!("{BASE_URL}/page/3/"));
	assert_eq!(
		search_url("duque", 1),
		format!("{BASE_URL}/?s=duque&post_type=wp-manga")
	);
	assert_eq!(
		search_url("duque", 2),
		format!("{BASE_URL}/page/2/?s=duque&post_type=wp-manga")
	);
	assert!(search_url("ação e duque", 1).contains("a%C3%A7%C3%A3o+e+duque"));

	assert!(is_manga_url(SAMPLE_MANGA_URL));
	assert!(is_chapter_url(SAMPLE_CHAPTER_URL));
	assert_eq!(
		manga_key_from_url(SAMPLE_MANGA_URL),
		Some(String::from(SAMPLE_MANGA_KEY))
	);
	assert_eq!(
		chapter_key_from_url(SAMPLE_CHAPTER_URL),
		Some(String::from(SAMPLE_CHAPTER_KEY))
	);
	assert_eq!(
		chapter_url(&make_manga(), &make_chapter()),
		String::from(SAMPLE_CHAPTER_URL)
	);
}

#[aidoku_test]
fn helper_parses_numbers_and_dates() {
	assert_eq!(parse_chapter_number("Capitulo 126", ""), Some(126.0));
	assert_eq!(parse_chapter_number("capitulo-09", ""), Some(9.0));
	assert_eq!(
		extract_date_token("Capitulo 120 03/05/2026"),
		Some(String::from("03/05/2026"))
	);
	assert_eq!(parse_pt_br_date("foo"), None);
}

#[aidoku_test]
fn source_maps_home_entries_page_one() {
	let source = MonteTaiScanlator::new();
	let result = source.get_search_manga_list(None, 1, Vec::new()).unwrap();

	assert!(!result.entries.is_empty());
	assert!(result.entries.iter().all(|entry| !entry.key.is_empty()));
	assert!(result.entries.iter().all(|entry| !entry.title.is_empty()));
	assert!(result.entries.iter().any(|entry| {
		entry
			.cover
			.as_ref()
			.map(|cover| cover.starts_with("http"))
			.unwrap_or(false)
	}));
	assert!(result.entries.iter().any(|entry| {
		entry
			.url
			.as_ref()
			.map(|url| url.contains("/manga/"))
			.unwrap_or(false)
	}));
	assert!(result.has_next_page);

	for i in 0..result.entries.len() {
		for j in (i + 1)..result.entries.len() {
			assert_ne!(result.entries[i].key, result.entries[j].key);
		}
	}
}

#[aidoku_test]
fn source_maps_home_entries_page_two() {
	let source = MonteTaiScanlator::new();
	let result = source.get_search_manga_list(None, 2, Vec::new()).unwrap();

	assert!(!result.entries.is_empty());
	assert!(result.entries.iter().all(|entry| !entry.key.is_empty()));
	assert!(result.entries.iter().any(|entry| {
		entry
			.url
			.as_ref()
			.map(|url| url.contains("/manga/"))
			.unwrap_or(false)
	}));
}

#[aidoku_test]
fn source_maps_search_entries() {
	let source = MonteTaiScanlator::new();
	let result = source
		.get_search_manga_list(Some(String::from("duque")), 1, Vec::new())
		.unwrap();

	assert!(!result.entries.is_empty());
	assert!(result.entries.iter().any(|entry| {
		entry.title.to_lowercase().contains("duque") || entry.key.to_lowercase().contains("duque")
	}));
}

#[aidoku_test]
fn source_maps_manga_details() {
	let source = MonteTaiScanlator::new();
	let updated = source.get_manga_update(make_manga(), true, false).unwrap();

	assert!(!updated.title.is_empty());
	assert!(updated.title.to_lowercase().contains("duque"));
	assert!(
		updated
			.cover
			.as_ref()
			.map(|cover| cover.contains("/wp-content/uploads/") && cover.contains("200x300"))
			.unwrap_or(false)
	);
	assert!(
		updated
			.description
			.as_ref()
			.map(|description| description.len() > 80)
			.unwrap_or(false)
	);
	assert!(updated.status != MangaStatus::Unknown);
	assert!(
		updated
			.tags
			.as_ref()
			.map(|tags| !tags.is_empty())
			.unwrap_or(false)
	);
	assert!(
		updated
			.url
			.as_ref()
			.map(|url| url.contains(SAMPLE_MANGA_KEY))
			.unwrap_or(false)
	);
}

#[aidoku_test]
fn source_maps_manga_chapters() {
	let source = MonteTaiScanlator::new();
	let updated = source.get_manga_update(make_manga(), false, true).unwrap();
	let chapters = updated.chapters.unwrap_or_default();

	assert!(chapters.len() > 40);
	assert!(chapters.iter().all(|chapter| {
		chapter
			.key
			.starts_with("manga/o-filho-do-duque-regressado-e-um-assassino/capitulo-")
	}));
	assert!(chapters.iter().all(|chapter| {
		chapter
			.url
			.as_ref()
			.map(|url| url.contains(SAMPLE_MANGA_KEY))
			.unwrap_or(true)
	}));
	assert!(
		chapters
			.iter()
			.any(|chapter| chapter.chapter_number.is_some())
	);
	assert!(chapters.iter().any(|chapter| {
		chapter
			.title
			.as_ref()
			.map(|title| title.to_lowercase().contains("capitulo"))
			.unwrap_or(false)
	}));
}

#[aidoku_test]
fn source_maps_chapter_pages() {
	let source = MonteTaiScanlator::new();
	let pages = source.get_page_list(make_manga(), make_chapter()).unwrap();

	assert!(pages.len() >= 5);
	let mut saw_proxy_image = false;
	let mut saw_context_referer = false;

	for page in pages.iter().take(12) {
		match &page.content {
			PageContent::Url(url, context) => {
				assert!(url.starts_with("http"));
				if url.contains("mt_madara_s3_image") {
					saw_proxy_image = true;
				}
				if let Some(context) = context {
					if let Some(referer) = context.get("referer") {
						assert_eq!(referer, SAMPLE_CHAPTER_URL);
						saw_context_referer = true;
					}
				}
			}
			_ => panic!("Chapter page content must be URL"),
		}
	}

	assert!(saw_proxy_image);
	assert!(saw_context_referer);
}

#[aidoku_test]
fn source_maps_chapter_pages_from_manga_update() {
	let source = MonteTaiScanlator::new();
	let updated = source.get_manga_update(make_manga(), false, true).unwrap();
	let chapters = updated.chapters.unwrap_or_default();
	let first = chapters.first().cloned().unwrap();

	let pages = source.get_page_list(make_manga(), first).unwrap();
	assert!(pages.len() >= 5);
	assert!(pages.iter().any(|page| match &page.content {
		PageContent::Url(url, _) => url.contains("mt_madara_s3_image"),
		_ => false,
	}));
}

#[aidoku_test]
fn source_maps_chapter_pages_with_key_only() {
	let source = MonteTaiScanlator::new();
	let chapter = Chapter {
		key: String::from(SAMPLE_CHAPTER_KEY),
		url: None,
		..Default::default()
	};

	let pages = source.get_page_list(make_manga(), chapter).unwrap();
	assert!(pages.len() >= 5);
}

#[aidoku_test]
fn source_maps_baskerville_jpg_pages() {
	let source = MonteTaiScanlator::new();
	let manga = Manga {
		key: String::from(BASKERVILLE_MANGA_KEY),
		url: Some(String::from(BASKERVILLE_MANGA_URL)),
		..Default::default()
	};
	let chapter = Chapter {
		key: String::from(BASKERVILLE_CHAPTER_KEY),
		url: Some(String::from(BASKERVILLE_CHAPTER_URL)),
		..Default::default()
	};

	let pages = source.get_page_list(manga, chapter).unwrap();
	assert!(pages.len() >= 10);
	assert!(pages.iter().all(|page| match &page.content {
		PageContent::Url(url, _) =>
			!url.contains("/manga/a-vinganca-do-cao-de-caca-dos-baskerville/capitulo-123/"),
		_ => false,
	}));
	assert!(pages.iter().any(|page| match &page.content {
		PageContent::Url(url, _) => {
			url.contains("/wp-content/uploads/wp-manga/data/") || url.ends_with(".jpg")
		}
		_ => false,
	}));
}

#[aidoku_test]
fn source_image_request_returns_image_response() {
	let source = MonteTaiScanlator::new();
	let pages = source.get_page_list(make_manga(), make_chapter()).unwrap();
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
	let content_type = response
		.get_header("content-type")
		.unwrap_or_default()
		.to_lowercase();
	assert!(content_type.contains("image/"));
}

#[aidoku_test]
fn source_handles_deep_links() {
	let source = MonteTaiScanlator::new();

	let manga_result = source
		.handle_deep_link(String::from(SAMPLE_MANGA_URL))
		.unwrap();
	match manga_result {
		Some(DeepLinkResult::Manga { key }) => {
			assert_eq!(key, SAMPLE_MANGA_KEY);
		}
		_ => panic!("Expected manga deep link result"),
	}

	let chapter_result = source
		.handle_deep_link(String::from(SAMPLE_CHAPTER_URL))
		.unwrap();
	match chapter_result {
		Some(DeepLinkResult::Chapter { manga_key, key }) => {
			assert_eq!(manga_key, SAMPLE_MANGA_KEY);
			assert_eq!(key, SAMPLE_CHAPTER_KEY);
		}
		_ => panic!("Expected chapter deep link result"),
	}

	let unsupported = source
		.handle_deep_link(String::from("https://montetaiscanlator.xyz/"))
		.unwrap();
	assert!(unsupported.is_none());
}

#[aidoku_test]
fn parser_maps_manga_document_directly() {
	let document = Request::get(SAMPLE_MANGA_URL).unwrap().html().unwrap();

	let title = parse_manga_title(&document).unwrap_or_default();
	let cover = parse_manga_cover(&document).unwrap_or_default();
	let description = parse_manga_description(&document).unwrap_or_default();
	let tags = parse_manga_tags(&document);
	let status = parse_manga_status(&document);
	let chapters = parse_manga_chapters(&document, Some(SAMPLE_MANGA_KEY));

	assert!(title.to_lowercase().contains("duque"));
	assert!(cover.contains("/wp-content/uploads/"));
	assert!(cover.contains("200x300"));
	assert!(description.len() > 80);
	assert!(!tags.is_empty());
	assert!(status != MangaStatus::Unknown);
	assert!(chapters.len() > 40);
}

#[aidoku_test]
fn parser_maps_chapter_images_directly() {
	let document = Request::get(SAMPLE_CHAPTER_URL).unwrap().html().unwrap();
	let urls = parse_chapter_page_urls(&document);

	assert!(urls.len() >= 5);
	assert!(urls.iter().all(|url| url.starts_with("http")));
	assert!(urls.iter().any(|url| url.contains("mt_madara_s3_image")));
	assert!(urls.iter().all(|url| !is_chapter_url(url)));
}

#[aidoku_test]
fn parser_validates_chapter_page_count() {
	let sample_document = Request::get(SAMPLE_CHAPTER_URL).unwrap().html().unwrap();
	let sample_dom_count = sample_document
		.select("img.wp-manga-chapter-img")
		.map(|images| images.size())
		.unwrap_or(0);
	let sample_urls = parse_chapter_page_urls(&sample_document);
	assert_eq!(sample_urls.len(), sample_dom_count);

	let baskerville_document = Request::get(BASKERVILLE_CHAPTER_URL)
		.unwrap()
		.html()
		.unwrap();
	let baskerville_dom_count = baskerville_document
		.select("img.wp-manga-chapter-img")
		.map(|images| images.size())
		.unwrap_or(0);
	let baskerville_urls = parse_chapter_page_urls(&baskerville_document);
	assert_eq!(baskerville_urls.len(), baskerville_dom_count);
}

#[aidoku_test]
fn parser_prioritizes_json_chapter_data() {
	let document = Request::get(BASKERVILLE_MANGA_URL).unwrap().html().unwrap();
	let chapters_from_json_or_default =
		parse_manga_chapters(&document, Some(BASKERVILLE_MANGA_KEY));
	let chapters_from_links_only =
		parse_manga_chapters_from_links(&document, Some(BASKERVILLE_MANGA_KEY));

	let expected_count = document
		.select_first(".mtx-chapter-list")
		.and_then(|list| list.attr("data-mtx-expected-count"))
		.and_then(|value| value.parse::<usize>().ok())
		.unwrap_or(0);

	assert!(expected_count > 40);
	assert!(chapters_from_links_only.len() <= 40);
	assert_eq!(chapters_from_json_or_default.len(), expected_count);
}

#[aidoku_test]
fn parser_reads_mtx_json_node() {
	let document = Request::get(BASKERVILLE_MANGA_URL).unwrap().html().unwrap();
	let entries = read_mtx_chapter_json_entries(&document).unwrap_or_default();
	assert!(entries.len() > 40);
}

#[aidoku_test]
fn parser_falls_back_to_links_when_json_missing() {
	use aidoku::imports::html::Html;
	let html = r#"
			<div class='mtx-chapter-list'>
				<a class='mtx-chapter-item' href='https://montetaiscanlator.xyz/manga/test/capitulo-2/'>
					<span class='mtx-chapter-title'>Capitulo 2</span>
					<span class='mtx-chapter-meta'>02/01/2026</span>
				</a>
				<a class='mtx-chapter-item' href='https://montetaiscanlator.xyz/manga/test/capitulo-1/'>
					<span class='mtx-chapter-title'>Capitulo 1</span>
					<span class='mtx-chapter-meta'>01/01/2026</span>
				</a>
			</div>
		"#;
	let document = Html::parse(html).unwrap();
	let chapters = parse_manga_chapters(&document, Some("test"));

	assert_eq!(chapters.len(), 2);
	assert_eq!(chapters[0].key, "manga/test/capitulo-2");
	assert_eq!(chapters[1].key, "manga/test/capitulo-1");
}
