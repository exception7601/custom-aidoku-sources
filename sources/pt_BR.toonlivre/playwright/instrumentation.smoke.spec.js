const { test, expect } = require("@playwright/test");

const {
  INSTRUMENTATION_SOURCE,
  createInstrumentedContext,
  openChapter,
  readDebugState,
  readParsedChapterCache,
  waitForChapterCache,
} = require("./toonlivre-instrumentation");

test.describe.configure({ mode: "serial" });

async function expectChapterCapture(page, scenario) {
  await expect
    .poll(async () => {
      try {
        await waitForChapterCache(page);
        const { parsed } = await readParsedChapterCache(page);
        return parsed?.chapter?.pages?.length || 0;
      } catch (_) {
        return 0;
      }
    })
    .toBeGreaterThan(0);

  const { raw, parsed } = await readParsedChapterCache(page);
  expect(raw).toBeTruthy();
  expect(parsed).toBeTruthy();
  expect(parsed.chapter.id).toBe(scenario.bootConfig.targetChapterId);
  expect(parsed.chapter.mangaId).toBe(scenario.bootConfig.targetMangaId);
  expect(parsed.chapter.number).toBe(scenario.bootConfig.targetChapterNumber);
  expect(parsed.chapter.pages.length).toBeGreaterThan(0);

  const state = await readDebugState(page);
  expect(state.config).toBeTruthy();
  expect(state.debug).toBeTruthy();
  expect(state.debug.fingerprintProfile).toContain("iphone:");
  expect(state.debug.events.length).toBeGreaterThan(0);

  const joinedEvents = state.debug.events.join("\n");
  expect(joinedEvents).toContain("chapter:cache-saved");
  expect(
    joinedEvents.includes("/api/reader/proof/verify") ||
      joinedEvents.includes("/api/reader/runtime") ||
      joinedEvents.includes("worker:proxy-source-ready")
  ).toBeTruthy();
  expect(state.debug.workerUrls.length).toBeGreaterThan(0);
}

test("shared instrumentation captures chapter pages and runtime signals", async ({ browser }) => {
  const { context, scenario } = await createInstrumentedContext(browser);
  const page = await context.newPage();

  try {
    expect(INSTRUMENTATION_SOURCE).toContain("__toonlivreAidokuBoot");
    await openChapter(page, scenario);
    await expectChapterCapture(page, scenario);
  } finally {
    await context.close();
  }
});

test("shared instrumentation still captures chapter pages without opening the homepage first", async ({ browser }) => {
  const { context, scenario } = await createInstrumentedContext(browser);
  const page = await context.newPage();

  try {
    await openChapter(page, scenario, { loadBasePage: false });
    await expectChapterCapture(page, scenario);
  } finally {
    await context.close();
  }
});

test("shared instrumentation still works with debug disabled", async ({ browser }) => {
  const { context, scenario } = await createInstrumentedContext(browser, { debug: false });
  const page = await context.newPage();

  try {
    await openChapter(page, scenario, { loadBasePage: false });

    await expect
      .poll(async () => {
        try {
          await waitForChapterCache(page);
          const { parsed } = await readParsedChapterCache(page);
          return parsed?.chapter?.pages?.length || 0;
        } catch (_) {
          return 0;
        }
      })
      .toBeGreaterThan(0);

    const { raw, parsed } = await readParsedChapterCache(page);
    expect(raw).toBeTruthy();
    expect(parsed).toBeTruthy();
    expect(parsed.chapter.pages.length).toBeGreaterThan(0);

    const state = await readDebugState(page);
    expect(state.config).toBeTruthy();
    expect(state.config.debug).toBeFalsy();
  } finally {
    await context.close();
  }
});
