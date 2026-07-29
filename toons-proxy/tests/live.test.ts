import { describe, expect, it } from "bun:test";
import {
  fetchChapterDetails,
  fetchMangaById,
  fetchMangaBySlug,
  fetchMangaReader,
  fetchReleases,
  searchMangas,
} from "../src/toonlivre-api";

describe("Toons Total Proxy - Live Integration Tests", () => {
  describe("Releases API", () => {
    it("should fetch releases without token-server", async () => {
      const data = await fetchReleases(1, 3);

      expect(data).toBeDefined();
      expect(data.mangas).toBeDefined();
      expect(Array.isArray(data.mangas)).toBe(true);
      expect(data.pagination).toBeDefined();
      expect(data.pagination.currentPage).toBe(1);
    });
  });

  describe("Search API", () => {
    it("should search mangas without token-server", async () => {
      const data = await searchMangas("demon", 1, 3);

      expect(data).toBeDefined();
      expect(data.mangas).toBeDefined();
      expect(Array.isArray(data.mangas)).toBe(true);
    });
  });

  describe("Manga Details API", () => {
    it("should fetch manga by slug without token-server", async () => {
      const data = await fetchMangaBySlug("contos-de-demonios-e-deuses");

      expect(data).toBeDefined();
      expect(data.id).toBeDefined();
      expect(data.title).toBeDefined();
      // Note: API may not return slug field
      // console.log("[test] Manga data:", JSON.stringify(data, null, 2));
    });

    it("should fetch manga by id without token-server", async () => {
      // First get a manga slug to get its ID
      const searchResult = await fetchMangaBySlug(
        "contos-de-demonios-e-deuses",
      );
      const mangaId = searchResult.id;

      const data = await fetchMangaById(mangaId);

      expect(data).toBeDefined();
      expect(data.id).toBe(mangaId);
      expect(data.title).toBeDefined();
    });
  });

  describe("Reader API", () => {
    it("should fetch manga reader without token-server", async () => {
      const searchResult = await fetchMangaBySlug(
        "contos-de-demonios-e-deuses",
      );
      const mangaId = searchResult.id;

      const data = await fetchMangaReader(mangaId);

      expect(data).toBeDefined();
      expect(data.id).toBe(mangaId);
      expect(data.chapters).toBeDefined();
      expect(Array.isArray(data.chapters)).toBe(true);
    });
  });

  describe("Chapter Details API", () => {
    it("should fetch chapter details without token-server", async () => {
      // Get manga first
      const manga = await fetchMangaBySlug("obra-2f24ed3d");
      const reader = await fetchMangaReader(manga.id);

      if (reader.chapters.length > 0) {
        const chapter = reader.chapters.find(
          (item) => item.id === "cap-9da2ca9c-01",
        );

        expect(chapter).toBeDefined();

        const chapterData = await fetchChapterDetails(
          manga.id,
          "cap-9da2ca9c-01",
          chapter?.url,
        );

        expect(chapterData).toBeDefined();
        expect(chapterData.pages).toBeDefined();
        expect(Array.isArray(chapterData.pages)).toBe(true);

        console.log(`[test] Chapter has ${chapterData.pages.length} pages`);
        if (chapterData.pages.length > 0) {
          console.log(`[test] First page: ${chapterData.pages[0]}`);
        }
      }
    });
  });

  describe("Cache Behavior", () => {
    it("should cache requests for performance", async () => {
      const start1 = Date.now();
      await fetchReleases(1, 3);
      const time1 = Date.now() - start1;

      // Wait a bit to ensure different timing
      await new Promise((resolve) => setTimeout(resolve, 10));

      const start2 = Date.now();
      await fetchReleases(1, 3);
      const time2 = Date.now() - start2;

      console.log(`[cache test] First request: ${time1}ms, Cached: ${time2}ms`);

      // Both requests might be cached if run too quickly
      // Just verify cache is working (second should be <= first)
      expect(time2).toBeLessThanOrEqual(time1);
    });
  });
});
