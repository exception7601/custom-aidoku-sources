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
	assert_eq!(
		manga_key_from_url("https://montetaiscanlator.xyz/a-divina-loja-de-mascotes-estelares/"),
		Some(String::from("a-divina-loja-de-mascotes-estelares"))
	);
	assert_eq!(
		chapter_key_from_url(
			"https://montetaiscanlator.xyz/a-divina-loja-de-mascotes-estelares/capitulo-213/",
		),
		Some(String::from(
			"a-divina-loja-de-mascotes-estelares/capitulo-213"
		))
	);
	assert!(!is_manga_url("https://montetaiscanlator.xyz/page/2/"));
	assert!(!is_chapter_url("https://montetaiscanlator.xyz/page/2/"));
	assert_eq!(manga_key_from_url("https://discord.gg/cj3CVFqT6E"), None);
	assert_eq!(
		manga_key_from_url("https://montetaiscanlator.xyz/politica-de-privacidade/"),
		None
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
fn helper_detects_structural_pagination_next_page() {
	let html = r#"
		<nav class='navigation paging-navigation'>
			<div class='wp-pagenavi'>
				<span aria-current='page' class='page-numbers current'>1</span>
				<a class='page-numbers' href='https://montetaiscanlator.xyz/page/2/?s=a&post_type=wp-manga'>2</a>
				<a class='next page-numbers' href='https://montetaiscanlator.xyz/page/2/?s=a&post_type=wp-manga'>Proximo</a>
			</div>
		</nav>
	"#;
	let document = Html::parse(html).unwrap();

	assert!(has_next_page(&document));
}

#[aidoku_test]
fn helper_detects_structural_pagination_last_page() {
	let html = r#"
		<nav class='navigation paging-navigation'>
			<div class='wp-pagenavi'>
				<a class='prev page-numbers' href='https://montetaiscanlator.xyz/page/11/?s=a&post_type=wp-manga'>Anterior</a>
				<a class='page-numbers' href='https://montetaiscanlator.xyz/page/10/?s=a&post_type=wp-manga'>10</a>
				<a class='page-numbers' href='https://montetaiscanlator.xyz/page/11/?s=a&post_type=wp-manga'>11</a>
				<span aria-current='page' class='page-numbers current'>12</span>
			</div>
		</nav>
	"#;
	let document = Html::parse(html).unwrap();

	assert!(!has_next_page(&document));
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
	assert!(date_uploaded >= now - (3 * 24 * 60 * 60));
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
fn parser_ignores_non_manga_links_on_home_page() {
	let html = r#"
		<header>
			<a href='https://discord.gg/cj3CVFqT6E'>Discord</a>
			<a href='https://montetaiscanlator.xyz/politica-de-privacidade/'>Política de privacidade</a>
		</header>
		<article class='mt-popular-home__card'>
			<a class='mt-popular-home__thumb' href='https://montetaiscanlator.xyz/manga/o-deus-do-caos-todo-poderoso/'>
				<img width='506' height='740' src='https://montetaiscanlator.xyz/wp-content/uploads/2024/12/gif-myst.gif' class='img-responsive' alt='gif-myst'>
			</a>
			<h3 class='mt-popular-home__title'>
				<a href='https://montetaiscanlator.xyz/manga/o-deus-do-caos-todo-poderoso/'>Deus do Caos Todo-Poderoso</a>
			</h3>
		</article>
		<nav class='mt-home-lab-catalog__pagination'>
			<a class='next page-numbers' href='https://montetaiscanlator.xyz/page/2/'>Proximo</a>
		</nav>
	"#;
	let document = Html::parse(html).unwrap();
	let entries = parse_entries(&document, false);

	assert_eq!(entries.len(), 1);
	assert_eq!(entries[0].key, "o-deus-do-caos-todo-poderoso");
	assert_eq!(entries[0].title, "Deus do Caos Todo-Poderoso");
}

#[aidoku_test]
fn parser_ignores_non_manga_links_on_search_page() {
	let html = r#"
		<div class='row c-tabs-item__content'>
			<div class='col-4 col-md-2'>
				<div class='tab-thumb c-image-hover'>
					<a href='https://montetaiscanlator.xyz/manga/o-filho-do-duque-regressado-e-um-assassino/' title='O Filho do Duque Regressado é um Assassino'>
						<img width='350' height='476' src='https://montetaiscanlator.xyz/wp-content/uploads/2026/03/001-67b185a156ba45770d7b96e14d5f8f97-350x476.png' alt='001 – 67b185a156ba45770d7b96e14d5f8f97'>
					</a>
				</div>
			</div>
			<div class='col-8 col-md-10'>
				<div class='tab-summary'>
					<div class='post-title'>
						<h3 class='h4'><a href='https://montetaiscanlator.xyz/manga/o-filho-do-duque-regressado-e-um-assassino/'>O Filho do Duque Regressado é um Assassino</a></h3>
					</div>
				</div>
			</div>
		</div>
		<footer>
			<a href='https://discord.gg/cj3CVFqT6E'>Discord</a>
			<a href='https://montetaiscanlator.xyz/politica-de-privacidade/'>Política de privacidade</a>
			<a href='https://montetaiscanlator.xyz/page/2/?s=duque&post_type=wp-manga'>Próximo</a>
		</footer>
	"#;
	let document = Html::parse(html).unwrap();
	let entries = parse_entries(&document, true);

	assert_eq!(entries.len(), 1);
	assert_eq!(entries[0].key, "o-filho-do-duque-regressado-e-um-assassino");
	assert_eq!(
		entries[0].title,
		"O Filho do Duque Regressado é um Assassino"
	);
}

#[aidoku_test]
fn parser_maps_a_divina_detail_document() {
	let html = r#"
		<meta property='og:title' content='A Divina Loja de Mascotes Estelares'>
		<meta property='og:image' content='https://montetaiscanlator.xyz/wp-content/uploads/2024/12/A-Divina-Loja-de-Mascotes-Estelares-v2.gif'>
		<meta name='twitter:image' content='https://montetaiscanlator.xyz/wp-content/uploads/2024/12/A-Divina-Loja-de-Mascotes-Estelares-v2.gif'>
		<div class='mtx-layout' style='--mtx-cover-image:url(https://montetaiscanlator.xyz/wp-content/uploads/2024/12/A-Divina-Loja-de-Mascotes-Estelares-v2.gif);'>
			<div class='mtx-cover'>
				<img class='img-responsive mtx-cover-gif' src='https://montetaiscanlator.xyz/wp-content/uploads/2024/12/A-Divina-Loja-de-Mascotes-Estelares-v2.gif' alt='A Divina Loja de Mascotes Estelares' loading='eager' decoding='async' width='1024' height='1536'>
			</div>
			<div class='mtx-body'>
				<div class='mtx-body-inner'>
					<div class='mtx-main'>
						<section class='mtx-hero'>
							<div class='mtx-hero-badges'>
								<span class='mtx-pill mtx-pill-type'>Manhwa</span>
								<span class='mtx-pill mtx-pill-status'><span class='mtx-dot'></span>Em progresso</span>
							</div>
							<h2 class='mtx-hero-title'>A Divina Loja de Mascotes Estelares</h2>
							<p class='mtx-hero-subtitle'>The Divine Pet Shop, A Divina Loja de Mascotes Estelares, A Loja Divina de Mascotes</p>
							<div class='mtx-chip-list'>
								<a class='mtx-chip' href='https://montetaiscanlator.xyz/genero/academica/'>Acadêmica</a>
								<a class='mtx-chip' href='https://montetaiscanlator.xyz/genero/acao/'>Ação</a>
								<a class='mtx-chip' href='https://montetaiscanlator.xyz/genero/aventura/'>Aventura</a>
								<a class='mtx-chip' href='https://montetaiscanlator.xyz/genero/genio/'>Gênio</a>
								<a class='mtx-chip' href='https://montetaiscanlator.xyz/genero/manhwa/'>Manhwa</a>
								<a class='mtx-chip' href='https://montetaiscanlator.xyz/genero/regressao/'>Regressão</a>
							</div>
						</section>
					</div>
				</div>
			</div>
			<div class='mtx-reading-zone' data-mtx-ready='1' data-mtx-reviews-watch='1' data-mtx-chapter-poll='1'>
				<div class='mtx-tabs' role='tablist' aria-label='Abas da obra'>
					<button type='button' class='mtx-tab is-active' data-mtx-tab='chapters' aria-selected='true'>Capitulos (213)</button>
					<button type='button' class='mtx-tab' data-mtx-tab='synopsis' aria-selected='false'>Sinopse</button>
					<button type='button' class='mtx-tab' data-mtx-tab='reviews' aria-selected='false'>Comentarios<span class='mtx-tab-count' data-mtx-reviews-count=''> (0)</span></button>
				</div>
				<div class='mtx-panel is-active' data-mtx-panel='chapters'>
					<div class='mtx-chapter-list' data-mtx-page-size='40' data-mtx-show-thumbs='0' data-mtx-cover='https://montetaiscanlator.xyz/wp-content/uploads/2024/12/A-Divina-Loja-de-Mascotes-Estelares-v2-683x1024.gif' data-mtx-manga-id='1890' data-mtx-latest-update='1776453128' data-mtx-expected-count='213'>
						<script class='mtx-chapter-data' type='application/json'>
							[
								{"url":"https://montetaiscanlator.xyz/manga/a-divina-loja-de-mascotes-estelares/capitulo-213/","title":"Capitulo 213","meta":"17/04/2026","search":"capitulo 213"},
								{"url":"https://montetaiscanlator.xyz/manga/a-divina-loja-de-mascotes-estelares/capitulo-212/","title":"Capitulo 212","meta":"17/04/2026","search":"capitulo 212"}
							]
						</script>
					</div>
				</div>
				<div class='mtx-panel' data-mtx-panel='synopsis' id='manga-desc'>
					<div class='mtx-synopsis'>
						<p>Cyan Vert, o melhor assassino do continente, enfrentou uma morte lamentável após ser traido pelo seu próprio irmão, a quem ele confiava a sua vida. Se eu tivesse a chance de viver mais uma vez, eu viveria ela de um jeito diferente. Eu só confiaria em mim mesmo, e conquistaria todas coisas que eu queria por conta própria, sem servir a ninguém além de mim mesmo. E foi assim que me deram uma segunda chance na vida. O Cyan vert, a sombra que viveu pelo outros, não existe mais. Agora eu irei abrir o meu próprio caminho, por mim mesmo.</p>
						<p>&nbsp;</p>
					</div>
				</div>
			</div>
		</div>
	"#;
	let document = Html::parse(html).unwrap();

	assert_eq!(
		parse_manga_title(&document).as_deref(),
		Some("A Divina Loja de Mascotes Estelares")
	);
	assert_eq!(
		parse_manga_cover(&document).as_deref(),
		Some(
			"https://montetaiscanlator.xyz/wp-content/uploads/2024/12/A-Divina-Loja-de-Mascotes-Estelares-v2-683x1024.gif"
		)
	);
	assert_eq!(parse_manga_status(&document), MangaStatus::Ongoing);
	assert_eq!(
		parse_manga_tags(&document),
		Vec::from([
			String::from("Acadêmica"),
			String::from("Ação"),
			String::from("Aventura"),
			String::from("Gênio"),
			String::from("Manhwa"),
			String::from("Regressão"),
		])
	);
	assert_eq!(
		parse_manga_description(&document).as_deref(),
		Some(
			"Cyan Vert, o melhor assassino do continente, enfrentou uma morte lamentável após ser traido pelo seu próprio irmão, a quem ele confiava a sua vida. Se eu tivesse a chance de viver mais uma vez, eu viveria ela de um jeito diferente. Eu só confiaria em mim mesmo, e conquistaria todas coisas que eu queria por conta própria, sem servir a ninguém além de mim mesmo. E foi assim que me deram uma segunda chance na vida. O Cyan vert, a sombra que viveu pelo outros, não existe mais. Agora eu irei abrir o meu próprio caminho, por mim mesmo."
		)
	);

	let chapters = parse_manga_chapters(&document, Some("a-divina-loja-de-mascotes-estelares"));
	assert_eq!(chapters.len(), 2);
	assert_eq!(
		chapters[0].key,
		"manga/a-divina-loja-de-mascotes-estelares/capitulo-213"
	);
	assert_eq!(chapters[0].date_uploaded, parse_pt_br_date("17/04/2026"));
	assert_eq!(
		chapters[1].key,
		"manga/a-divina-loja-de-mascotes-estelares/capitulo-212"
	);
	assert_eq!(chapters[1].date_uploaded, parse_pt_br_date("17/04/2026"));
}

#[aidoku_test]
fn source_prefers_gif_detail_cover_over_parent() {
	let html = r#"
		<meta property='og:title' content='A Divina Loja de Mascotes Estelares'>
		<meta property='og:image' content='https://montetaiscanlator.xyz/wp-content/uploads/2024/12/A-Divina-Loja-de-Mascotes-Estelares-v2.gif'>
		<meta name='twitter:image' content='https://montetaiscanlator.xyz/wp-content/uploads/2024/12/A-Divina-Loja-de-Mascotes-Estelares-v2.gif'>
		<div class='mtx-layout'>
			<div class='mtx-cover'>
				<img class='img-responsive mtx-cover-gif' src='https://montetaiscanlator.xyz/wp-content/uploads/2024/12/A-Divina-Loja-de-Mascotes-Estelares-v2.gif' alt='A Divina Loja de Mascotes Estelares' loading='eager' decoding='async' width='1024' height='1536'>
			</div>
			<div class='mtx-chapter-list' data-mtx-page-size='40' data-mtx-show-thumbs='0' data-mtx-cover='https://montetaiscanlator.xyz/wp-content/uploads/2024/12/A-Divina-Loja-de-Mascotes-Estelares-v2-683x1024.gif' data-mtx-manga-id='1890' data-mtx-latest-update='1776453128' data-mtx-expected-count='213'>
				<script class='mtx-chapter-data' type='application/json'>
					[
						{"url":"https://montetaiscanlator.xyz/manga/a-divina-loja-de-mascotes-estelares/capitulo-213/","title":"Capitulo 213","meta":"17/04/2026","search":"capitulo 213"},
						{"url":"https://montetaiscanlator.xyz/manga/a-divina-loja-de-mascotes-estelares/capitulo-212/","title":"Capitulo 212","meta":"17/04/2026","search":"capitulo 212"}
					]
				</script>
			</div>
		</div>
	"#;
	let document = Html::parse(html).unwrap();
	let manga = Manga {
		key: String::from("a-divina-loja-de-mascotes-estelares"),
		url: Some(String::from(
			"https://montetaiscanlator.xyz/manga/a-divina-loja-de-mascotes-estelares/",
		)),
		cover: Some(String::from(
			"https://montetaiscanlator.xyz/wp-content/uploads/2024/12/A-Divina-Loja-de-Mascotes-Estelares-v2.gif",
		)),
		..Default::default()
	};

	let updated = update_manga_from_document(manga, &document, true, true);

	assert_eq!(updated.title, "A Divina Loja de Mascotes Estelares");
	assert_eq!(
		updated.cover.as_deref(),
		Some(
			"https://montetaiscanlator.xyz/wp-content/uploads/2024/12/A-Divina-Loja-de-Mascotes-Estelares-v2-683x1024.gif"
		)
	);
	let chapters = updated.chapters.unwrap_or_default();
	assert_eq!(chapters.len(), 2);
	assert_eq!(
		chapters[0].key,
		"manga/a-divina-loja-de-mascotes-estelares/capitulo-213"
	);
	assert_eq!(chapters[0].date_uploaded, parse_pt_br_date("17/04/2026"));
	assert_eq!(
		chapters[1].key,
		"manga/a-divina-loja-de-mascotes-estelares/capitulo-212"
	);
	assert_eq!(chapters[1].date_uploaded, parse_pt_br_date("17/04/2026"));
}

#[aidoku_test]
fn parser_maps_a_divina_loja_home_card() {
	let html = r#"
		<article class='mt-popular-home__card'>
			<a class='mt-popular-home__thumb' href='https://montetaiscanlator.xyz/manga/a-divina-loja-de-mascotes-estelares/'>
				<span class='mt-popular-home__rank'>#2</span>
				<img width='1024' height='1536' src='https://montetaiscanlator.xyz/wp-content/uploads/2024/12/A-Divina-Loja-de-Mascotes-Estelares-v2.gif' class='img-responsive' style='width:auto; ' alt='A Divina Loja de Mascotes Estelares v2'>
			</a>
			<h3 class='mt-popular-home__title'>
				<a href='https://montetaiscanlator.xyz/manga/a-divina-loja-de-mascotes-estelares/'>A Divina Loja de Mascotes Estelares</a>
			</h3>
			<p class='mt-popular-home__views'>399.6K views</p>
		</article>
	"#;
	let document = Html::parse(html).unwrap();
	let entries = parse_entries(&document, false);

	assert_eq!(entries.len(), 1);
	assert_eq!(entries[0].key, "a-divina-loja-de-mascotes-estelares");
	assert_eq!(entries[0].title, "A Divina Loja de Mascotes Estelares");
	assert_eq!(
		entries[0].cover.as_deref(),
		Some(
			"https://montetaiscanlator.xyz/wp-content/uploads/2024/12/A-Divina-Loja-de-Mascotes-Estelares-v2.gif"
		)
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
fn source_maps_manga_details() {
	let source = MonteTaiScanlator::new();
	let updated = source.get_manga_update(make_manga(), true, false).unwrap();

	assert!(!updated.title.is_empty());
	assert!(updated.title.to_lowercase().contains("duque"));
	assert!(
		updated
			.cover
			.as_ref()
			.map(|cover| cover.contains("/wp-content/uploads/") && cover.ends_with(".png"))
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
	assert!(cover.ends_with(".png"));
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
	assert!(cover.ends_with(".png"));
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
