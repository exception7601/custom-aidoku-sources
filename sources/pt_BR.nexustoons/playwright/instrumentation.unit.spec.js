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
  return sandbox.__nexustoonsAidokuInternals;
}

test("extractMangaPayloadCandidate finds nested manga payloads by shape", async () => {
  const internals = loadInternals();
  const candidate = internals.extractMangaPayloadCandidate({
    ok: true,
    data: {
      payload: {
        manga: {
          title: "Test Manga",
          chapters: [
            {
              id: 397924,
              number: "45",
            },
          ],
        },
      },
    },
  });

  expect(candidate.title).toBe("Test Manga");
  expect(candidate.chapters).toHaveLength(1);
});

test("normalizeMangaDetailsPayload accepts aliases and chapter metadata", async () => {
  const internals = loadInternals();
  const manga = internals.normalizeMangaDetailsPayload({
    result: {
      title: "Test Manga",
      coverImage: "https://example.com/cover.jpg",
      description: "Test description",
      status: "ongoing",
      chapters: [
        {
          id: "397924",
          number: "45",
          title: "",
          createdAt: "2026-07-29T07:11:46.030197+02:00",
        },
      ],
    },
  });

  expect(manga).toEqual({
    title: "Test Manga",
    coverUrl: "https://example.com/cover.jpg",
    description: "Test description",
    status: "ongoing",
    chapters: [
      {
        id: 397924,
        number: "45",
        title: "",
        dateUploaded: "2026-07-29T07:11:46.030197+02:00",
      },
    ],
  });
});

test("filterImageUrls keeps unique reader image URLs", async () => {
  const internals = loadInternals();
  expect(
    internals.filterImageUrls(
      [
        "https://img.nx-toons.xyz/manga_pages/758/73801/page_001.webp",
        "https://img.nx-toons.xyz/manga_pages/758/73801/page_001.webp",
        "https://img.nx-toons.xyz/covers/test.jpg",
        "",
      ],
      {
        chapterImageUrlHints: ["manga_pages"],
      }
    )
  ).toEqual([
    "https://img.nx-toons.xyz/manga_pages/758/73801/page_001.webp",
  ]);
});

test("isMangaPayloadTarget stays scoped to site and endpoint hints", async () => {
  const internals = loadInternals();

  expect(
    internals.isMangaPayloadTarget("https://nexustoons.com/api/manga/758", {
      siteHostHints: ["nexustoons.com"],
      mangaPayloadUrlHints: ["/api/manga/"],
    })
  ).toBeTruthy();
  expect(
    internals.isMangaPayloadTarget("https://example.com/api/manga/758", {
      siteHostHints: ["nexustoons.com"],
      mangaPayloadUrlHints: ["/api/manga/"],
    })
  ).toBeFalsy();
});
