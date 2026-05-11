use super::*;
use aidoku::{
	Chapter, DeepLinkHandler, DeepLinkResult, ImageRequestProvider, Manga, MangaStatus,
	PageContent, Source,
	alloc::{String, Vec},
	imports::{html::Html, net::Request, std::current_date},
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

fn find_sample_manga_with_cover(entries: Vec<Manga>) -> Manga {
	entries
		.into_iter()
		.find(|entry| entry.key == SAMPLE_MANGA_KEY && entry.cover.is_some())
		.expect("Expected sample manga cover in list")
}

fn assert_update_keeps_cover(source: &MonteTaiScanlator, manga: Manga) {
	let expected_cover = manga.cover.clone();
	let updated = source.get_manga_update(manga, true, false).unwrap();
	assert_eq!(updated.cover, expected_cover);
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
fn helper_parses_relative_chapter_dates() {
	let fallback_date = 1_234_567_890;

	assert_eq!(
		chapter_date_from_text("03/05/2026", fallback_date),
		parse_pt_br_date("03/05/2026").unwrap()
	);
	assert_eq!(
		chapter_date_from_text("Capitulo 126 18 horas atrás", fallback_date),
		1_234_503_090
	);
	assert_eq!(
		chapter_date_from_text("5 minutos atrás", fallback_date),
		1_234_567_590
	);
	assert_eq!(
		chapter_date_from_text("2 dias atrás", fallback_date),
		1_234_395_090
	);
	assert_eq!(
		chapter_date_from_text("texto sem data", fallback_date),
		fallback_date
	);
}

#[aidoku_test]
fn source_parses_relative_chapter_dates_from_live_page() {
	let source = MonteTaiScanlator::new();
	let updated = source.get_manga_update(make_manga(), false, true).unwrap();
	let chapters = updated.chapters.unwrap_or_default();
	let now = current_date();
	let chapter = chapters
		.iter()
		.find(|chapter| chapter.key.ends_with("/capitulo-126"))
		.expect("Expected chapter 126 in live chapter list");
	let date_uploaded = chapter
		.date_uploaded
		.expect("Expected relative chapter date");

	assert!(date_uploaded < now);
	assert!(date_uploaded >= now - (20 * 60 * 60));
	assert!(date_uploaded <= now - (17 * 60 * 60));
}

#[aidoku_test]
fn parser_prefers_smaller_cover_from_srcset() {
	let html = r#"
		<div class='mt-manga-catalog-card__poster'>
			<a href='https://montetaiscanlator.xyz/manga/test/'>
				<img
					src='https://montetaiscanlator.xyz/wp-content/uploads/test-175x238.png'
					srcset='https://montetaiscanlator.xyz/wp-content/uploads/test-110x150.png 110w, https://montetaiscanlator.xyz/wp-content/uploads/test-175x238.png 175w, https://montetaiscanlator.xyz/wp-content/uploads/test-350x476.png 350w'
					alt='Test Manga'
				/>
			</a>
		</div>
	"#;
	let document = Html::parse(html).unwrap();
	let container = document
		.select_first(".mt-manga-catalog-card__poster")
		.unwrap();
	let entries = parse_entries(&document, false);

	assert_eq!(
		extract_cover_from_container(&container).as_deref(),
		Some("https://montetaiscanlator.xyz/wp-content/uploads/test-110x150.png")
	);
	assert_eq!(entries.len(), 1);
	assert_eq!(
		entries[0].cover.as_deref(),
		Some("https://montetaiscanlator.xyz/wp-content/uploads/test-110x150.png")
	);
}

#[aidoku_test]
fn parser_uses_popular_home_title_over_gif_alt() {
	let html = r#"
		<article class='mt-popular-home__card'>
			<a class='mt-popular-home__thumb' href='https://montetaiscanlator.xyz/manga/o-deus-do-caos-todo-poderoso/'>
				<span class='mt-popular-home__rank'>#5</span>
				<img width='506' height='740' src='https://montetaiscanlator.xyz/wp-content/uploads/2024/12/gif-myst.gif' class='img-responsive' style='width:auto; ' alt='gif-myst'>
			</a>
			<h3 class='mt-popular-home__title'>
				<a href='https://montetaiscanlator.xyz/manga/o-deus-do-caos-todo-poderoso/'>Deus do Caos Todo-Poderoso</a>
			</h3>
			<p class='mt-popular-home__views'>173.8K views</p>
		</article>
	"#;
	let document = Html::parse(html).unwrap();
	let entries = parse_entries(&document, false);

	assert_eq!(entries.len(), 1);
	assert_eq!(entries[0].key, "o-deus-do-caos-todo-poderoso");
	assert_eq!(entries[0].title, "Deus do Caos Todo-Poderoso");
	assert_eq!(
		entries[0].cover.as_deref(),
		Some("https://montetaiscanlator.xyz/wp-content/uploads/2024/12/gif-myst.gif")
	);
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
fn source_keeps_home_list_cover_on_manga_update() {
	let source = MonteTaiScanlator::new();
	let result = source.get_search_manga_list(None, 1, Vec::new()).unwrap();
	let manga = find_sample_manga_with_cover(result.entries);

	assert_update_keeps_cover(&source, manga);
}

#[aidoku_test]
fn source_keeps_search_list_cover_on_manga_update() {
	let source = MonteTaiScanlator::new();
	let result = source
		.get_search_manga_list(Some(String::from("duque")), 1, Vec::new())
		.unwrap();
	let manga = find_sample_manga_with_cover(result.entries);

	assert_update_keeps_cover(&source, manga);
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
	assert!(chapters.iter().any(|chapter| {
		chapter.key.ends_with("/capitulo-126") && chapter.date_uploaded.is_some()
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
fn parser_prefers_smaller_manga_cover_from_srcset() {
	let document = Request::get(SAMPLE_MANGA_URL).unwrap().html().unwrap();
	let cover = parse_manga_cover(&document).unwrap();

	assert!(cover.starts_with("https://montetaiscanlator.xyz/wp-content/uploads/"));
	assert!(cover.ends_with("-200x300.png"));
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
