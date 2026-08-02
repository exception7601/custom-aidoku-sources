const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");
const { test, expect } = require("@playwright/test");

const INSTRUMENTATION_PATH = path.join(__dirname, "..", "src", "webview", "instrumentation.js");
const INSTRUMENTATION_SOURCE = fs.readFileSync(INSTRUMENTATION_PATH, "utf8");

function loadInternals() {
  const sandbox = {
    globalThis: null,
    navigator: {
      userAgent: "TestAgent/1.0",
      platform: "TestPlatform",
      vendor: "TestVendor",
    },
    Request,
    Headers,
    URL,
    console,
  };
  sandbox.globalThis = sandbox;
  vm.runInNewContext(INSTRUMENTATION_SOURCE, sandbox, {
    filename: INSTRUMENTATION_PATH,
  });
  return sandbox.__toonlivreAidokuInternals;
}

test("extractChapterPayloadCandidate finds nested payloads by shape", async () => {
  const internals = loadInternals();
  const candidate = internals.extractChapterPayloadCandidate({
    ok: true,
    result: [
      1,
      2,
      {
        payload: {
          id: "cap-01",
          pages: [" page-1 ", "page-2"],
        },
      },
    ],
  });

  expect(candidate.id).toBe("cap-01");
  expect(Array.from(candidate.pages)).toEqual([" page-1 ", "page-2"]);
});

test("normalizeChapterPayload accepts aliases and target fallbacks", async () => {
  const internals = loadInternals();
  const chapter = internals.normalizeChapterPayload(
    {
      data: {
        chapter: {
          chapterId: "cap-02",
          chapter_number: "02",
          manga_id: "obra-02",
          images: ["img-1", "", "img-2"],
        },
      },
    },
    {
      targetChapterId: "fallback-cap",
      targetMangaId: "fallback-manga",
      targetChapterNumber: "fallback-number",
    }
  );

  expect(chapter).toEqual({
    id: "cap-02",
    pages: ["img-1", "img-2"],
    title: "",
    number: "02",
    mangaId: "obra-02",
    timestamp: 0,
    releaseDate: "",
  });
});

test("isRuntimeTarget honors configurable URL hints", async () => {
  const internals = loadInternals();

  expect(
    internals.isRuntimeTarget("https://site.test/runtime/task.js", {
      runtimeUrlHints: ["/runtime/"],
    })
  ).toBeTruthy();
  expect(
    internals.isRuntimeTarget("https://site.test/assets/app.js", {
      runtimeUrlHints: ["/runtime/"],
    })
  ).toBeFalsy();
});

test("isPayloadInspectionTarget stays generic but scoped to site hints", async () => {
  const internals = loadInternals();
  const config = {
    payloadUrlHints: ["/api/"],
    siteHostHints: ["toonlivre.net"],
  };

  expect(internals.isPayloadInspectionTarget("/api/chapter/1", config)).toBeTruthy();
  expect(
    internals.isPayloadInspectionTarget("https://toonlivre.net/api/chapter/1", config)
  ).toBeTruthy();
  expect(
    internals.isPayloadInspectionTarget("https://example.com/api/chapter/1", config)
  ).toBeFalsy();
});
