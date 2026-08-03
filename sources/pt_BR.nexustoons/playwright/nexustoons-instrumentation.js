const fs = require("node:fs");
const path = require("node:path");

const SOURCE_ROOT = path.resolve(__dirname, "..");
const INSTRUMENTATION_PATH = path.join(
  SOURCE_ROOT,
  "src",
  "webview",
  "instrumentation.js"
);
const INSTRUMENTATION_SOURCE = fs.readFileSync(INSTRUMENTATION_PATH, "utf8");

const WEBVIEW_USER_AGENT =
  "Mozilla/5.0 (iPhone; CPU iPhone OS 18_7 like Mac OS X) " +
  "AppleWebKit/605.1.15 (KHTML, like Gecko) Mobile/15E148";

const BLOCKED_RESOURCE_TYPES = new Set([
  "media",
  "font",
  "stylesheet",
  "texttrack",
]);

const BLOCKED_URL_SNIPPETS = [
  "googletagmanager.com",
  "google-analytics.com",
  "doubleclick.net",
  "connect.facebook.net",
  "clarity.ms",
  "hotjar.com",
];

const DEFAULT_SCENARIO = {
  baseUrl: "https://nexustoons.com/",
  mangaSlug:
    process.env.NEXUSTOONS_MANGA_SLUG ||
    "sono-akuyaku-kizoku-mama-heroine-ga-suki-sugiru-shinshi-na-doryoku-de-saikyou-to-nari-fuguu-na-oshi-chara-tasukemakuru",
  chapterId: process.env.NEXUSTOONS_CHAPTER_ID || "397924",
  debug: true,
  blockNonEssentialResources: true,
};

function buildMangaStorageKey(mangaSlug) {
  return `nexustoons_manga_cache_v1:${String(mangaSlug || "").trim().replace(/^\/+|\/+$/g, "")}`;
}

function buildChapterStorageKey(mangaSlug, chapterId) {
  const slug = String(mangaSlug || "").trim().replace(/^\/+|\/+$/g, "");
  const id = String(chapterId || "").trim().replace(/^\/+|\/+$/g, "");
  return `nexustoons_chapter_pages_cache_v1:${slug}:${id}`;
}

function createScenario(overrides = {}) {
  const merged = { ...DEFAULT_SCENARIO, ...overrides };
  return {
    ...merged,
    mangaUrl: `https://nexustoons.com/manga/${merged.mangaSlug}`,
    chapterUrl: `https://nexustoons.com/ler/${merged.mangaSlug}/${merged.chapterId}`,
    bootConfig: {
      width: 390,
      height: 844,
      dpr: 3,
      userAgent: WEBVIEW_USER_AGENT,
      platform: "iPhone",
      vendor: "Apple Computer, Inc.",
      maxTouchPoints: 5,
      languageHeader: "pt-BR,pt;q=0.9",
      siteHostHints: ["nexustoons.com", "img.nx-toons.xyz"],
      mangaPageUrlHints: ["/manga/"],
      chapterPageUrlHints: ["/ler/"],
      mangaPayloadUrlHints: ["/api/manga/"],
      chapterButtonTextHints: ["Capítulos", "Capitulos"],
      chapterLinkUrlHints: ["/ler/", "/r/"],
      reactRootSelectors: ["div.custom-scrollbar"],
      chapterImageUrlHints: ["manga_pages"],
      mangaCacheGlobalKey: "__nexustoonsAidokuMangaCache",
      mangaStorageKey: buildMangaStorageKey(merged.mangaSlug),
      chapterCacheGlobalKey: "__nexustoonsAidokuChapterPagesCache",
      chapterStorageKey: buildChapterStorageKey(merged.mangaSlug, merged.chapterId),
      debug: merged.debug,
    },
  };
}

function shouldAbortRequest(request, scenario) {
  if (!scenario.blockNonEssentialResources) {
    return false;
  }

  const resourceType = request.resourceType();
  if (BLOCKED_RESOURCE_TYPES.has(resourceType)) {
    return true;
  }

  const url = request.url();
  return BLOCKED_URL_SNIPPETS.some((snippet) => url.includes(snippet));
}

async function installResourceBlocking(context, scenario) {
  if (!scenario.blockNonEssentialResources) {
    return;
  }

  await context.route("**/*", async (route) => {
    if (shouldAbortRequest(route.request(), scenario)) {
      await route.abort();
      return;
    }

    await route.continue();
  });
}

async function createInstrumentedContext(browser, overrides = {}) {
  const scenario = createScenario(overrides);
  const context = await browser.newContext({
    userAgent: scenario.bootConfig.userAgent,
    locale: "pt-BR",
    viewport: {
      width: scenario.bootConfig.width,
      height: scenario.bootConfig.height,
    },
    deviceScaleFactor: scenario.bootConfig.dpr,
    isMobile: true,
    hasTouch: true,
    extraHTTPHeaders: {
      "Accept-Language": scenario.bootConfig.languageHeader,
    },
  });

  await installResourceBlocking(context, scenario);

  await context.addInitScript({ path: INSTRUMENTATION_PATH });
  await context.addInitScript(
    ({ bootConfig }) => {
      if (typeof globalThis.__nexustoonsAidokuBoot === "function") {
        globalThis.__nexustoonsAidokuBoot(bootConfig);
      }
    },
    { bootConfig: scenario.bootConfig }
  );

  return { context, scenario };
}

async function openManga(page, scenario) {
  await page.goto(scenario.mangaUrl, {
    waitUntil: "domcontentloaded",
    referer: scenario.baseUrl,
  });
}

async function openChapter(page, scenario) {
  await page.goto(scenario.chapterUrl, {
    waitUntil: "domcontentloaded",
    referer: scenario.baseUrl,
  });
}

async function waitForMangaCache(page) {
  await page.waitForFunction(() => {
    const storageKey = globalThis.__nexustoonsAidokuConfig?.mangaStorageKey;
    const raw =
      globalThis.__nexustoonsAidokuTestAPI?.readMangaCache?.() ||
      globalThis.__nexustoonsAidokuMangaCache ||
      (storageKey ? sessionStorage.getItem(storageKey) : "") ||
      "";
    return typeof raw === "string" && raw.length > 0;
  });
}

async function waitForChapterPagesCache(page) {
  await page.waitForFunction(() => {
    const storageKey = globalThis.__nexustoonsAidokuConfig?.chapterStorageKey;
    const raw =
      globalThis.__nexustoonsAidokuTestAPI?.readChapterPagesCache?.() ||
      globalThis.__nexustoonsAidokuChapterPagesCache ||
      (storageKey ? sessionStorage.getItem(storageKey) : "") ||
      "";
    return typeof raw === "string" && raw.length > 0;
  });
}

async function readMangaCache(page) {
  return page.evaluate(() => {
    const storageKey = globalThis.__nexustoonsAidokuConfig?.mangaStorageKey;
    return (
      globalThis.__nexustoonsAidokuTestAPI?.readMangaCache?.() ||
      globalThis.__nexustoonsAidokuMangaCache ||
      (storageKey ? sessionStorage.getItem(storageKey) : "") ||
      ""
    );
  });
}

async function readChapterPagesCache(page) {
  return page.evaluate(() => {
    const storageKey = globalThis.__nexustoonsAidokuConfig?.chapterStorageKey;
    return (
      globalThis.__nexustoonsAidokuTestAPI?.readChapterPagesCache?.() ||
      globalThis.__nexustoonsAidokuChapterPagesCache ||
      (storageKey ? sessionStorage.getItem(storageKey) : "") ||
      ""
    );
  });
}

async function readParsedMangaCache(page) {
  const raw = await readMangaCache(page);
  return {
    raw,
    parsed: raw ? JSON.parse(raw) : null,
  };
}

async function readParsedChapterPagesCache(page) {
  const raw = await readChapterPagesCache(page);
  return {
    raw,
    parsed: raw ? JSON.parse(raw) : null,
  };
}

async function readDebugState(page) {
  return page.evaluate(() => ({
    debug: globalThis.__nexustoonsAidokuDebug || null,
    config: globalThis.__nexustoonsAidokuConfig || null,
    mangaCache:
      globalThis.__nexustoonsAidokuTestAPI?.readMangaCache?.() ||
      globalThis.__nexustoonsAidokuMangaCache ||
      "",
    chapterCache:
      globalThis.__nexustoonsAidokuTestAPI?.readChapterPagesCache?.() ||
      globalThis.__nexustoonsAidokuChapterPagesCache ||
      "",
  }));
}

module.exports = {
  INSTRUMENTATION_PATH,
  INSTRUMENTATION_SOURCE,
  WEBVIEW_USER_AGENT,
  buildMangaStorageKey,
  buildChapterStorageKey,
  createInstrumentedContext,
  createScenario,
  openManga,
  openChapter,
  readMangaCache,
  readChapterPagesCache,
  readParsedMangaCache,
  readParsedChapterPagesCache,
  readDebugState,
  waitForMangaCache,
  waitForChapterPagesCache,
};
