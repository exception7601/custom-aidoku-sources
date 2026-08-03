const { test, expect } = require("@playwright/test");

const {
  INSTRUMENTATION_SOURCE,
  createInstrumentedContext,
  openChapter,
  openManga,
  readDebugState,
  readParsedChapterPagesCache,
  readParsedMangaCache,
  waitForChapterPagesCache,
  waitForMangaCache,
} = require("./nexustoons-instrumentation");

test.describe.configure({ mode: "serial" });

async function expectMangaCapture(page, scenario) {
  await expect
    .poll(async () => {
      try {
        await waitForMangaCache(page);
        const { parsed } = await readParsedMangaCache(page);
        return parsed?.manga?.chapters?.length || 0;
      } catch (_) {
        return 0;
      }
    })
    .toBeGreaterThan(0);

  const { raw, parsed } = await readParsedMangaCache(page);
  expect(raw).toBeTruthy();
  expect(parsed).toBeTruthy();
  expect(parsed.manga.title).toBeTruthy();
  expect(parsed.manga.chapters.length).toBeGreaterThan(0);

  const state = await readDebugState(page);
  expect(state.config).toBeTruthy();
  expect(state.debug).toBeTruthy();
  expect(state.debug.fingerprintProfile).toContain("iphone:");
  expect(state.debug.events.length).toBeGreaterThan(0);
  expect(state.debug.events.join("\n")).toContain("manga:cache-saved");
}

async function expectChapterPageCapture(page) {
  await expect
    .poll(async () => {
      try {
        await waitForChapterPagesCache(page);
        const { parsed } = await readParsedChapterPagesCache(page);
        return parsed?.pages?.length || 0;
      } catch (_) {
        return 0;
      }
    })
    .toBeGreaterThan(0);

  const { raw, parsed } = await readParsedChapterPagesCache(page);
  expect(raw).toBeTruthy();
  expect(parsed).toBeTruthy();
  expect(parsed.pages.length).toBeGreaterThan(0);
  expect(parsed.pages[0]).toContain("manga_pages");
}

test("shared instrumentation captures manga details from the WebView flow", async ({ browser }) => {
  const { context, scenario } = await createInstrumentedContext(browser);
  const page = await context.newPage();

  try {
    expect(INSTRUMENTATION_SOURCE).toContain("__nexustoonsAidokuBoot");
    await openManga(page, scenario);
    await expectMangaCapture(page, scenario);
  } finally {
    await context.close();
  }
});

test("shared instrumentation captures chapter page URLs in WebKit", async ({ browser }) => {
  const { context, scenario } = await createInstrumentedContext(browser);
  const page = await context.newPage();

  try {
    await openChapter(page, scenario);
    await expectChapterPageCapture(page);

    const state = await readDebugState(page);
    expect(state.config).toBeTruthy();
    expect(state.debug).toBeTruthy();
    expect(state.debug.events.join("\n")).toContain("chapter:cache-saved");
  } finally {
    await context.close();
  }
});

test("shared instrumentation still captures chapter page URLs with debug disabled", async ({ browser }) => {
  const { context, scenario } = await createInstrumentedContext(browser, { debug: false });
  const page = await context.newPage();

  try {
    await openChapter(page, scenario);
    await expectChapterPageCapture(page);

    const state = await readDebugState(page);
    expect(state.config).toBeTruthy();
    expect(state.config.debug).toBeFalsy();
  } finally {
    await context.close();
  }
});
