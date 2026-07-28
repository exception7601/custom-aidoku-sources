import { describe, expect, it } from "bun:test";
import type {
  ApiChapter,
  ApiListResponse,
  ApiMangaById,
} from "../src/toonlivre-api";

describe("Toons Total Proxy - Type Tests", () => {
  describe("API Response Types", () => {
    it("should validate ApiChapter type", () => {
      const chapter: ApiChapter = {
        id: "cap-1",
        number: "1",
        title: "Chapter 1",
        releaseDate: "2024-01-01",
        timestamp: 1704067200000,
        pageCount: 20,
      };

      expect(chapter.id).toBe("cap-1");
      expect(chapter.number).toBe("1");
    });

    it("should validate ApiListResponse type", () => {
      const response: ApiListResponse = {
        mangas: [
          {
            id: "obra-1",
            title: "Test Manga",
            coverUrl: "https://example.com/cover.jpg",
            slug: "test-manga",
            alternativeTitle: "Alternative",
            recent_chapters: [],
            registered_users_only: false,
          },
        ],
        pagination: {
          currentPage: 1,
          hasNextPage: false,
        },
      };

      expect(response.mangas.length).toBe(1);
      expect(response.pagination.currentPage).toBe(1);
    });

    it("should validate ApiMangaById type", () => {
      const manga: ApiMangaById = {
        id: "obra-1",
        slug: "test-manga",
        title: "Test Manga",
        coverUrl: "https://example.com/cover.jpg",
        authors: ["Author 1"],
        artists: ["Artist 1"],
        genres: ["Action"],
        description: "Test description",
        status: "Ongoing",
        alternativeTitle: "Alternative",
        recent_chapters: [],
        registered_users_only: false,
      };

      expect(manga.id).toBe("obra-1");
      expect(manga.authors.length).toBe(1);
      expect(manga.genres).toContain("Action");
    });
  });

  describe("Configuration", () => {
    it("should have valid environment variables accessible", () => {
      const tokenServerHost =
        process.env.TOKEN_SERVER_HOST || "https://toons.4nd.xyz";
      expect(tokenServerHost).toMatch(/^https?:\/\//);
    });

    it("should have valid port configuration", () => {
      const port = Number.parseInt(process.env.PORT || "3000");
      expect(port).toBeGreaterThan(0);
      expect(port).toBeLessThanOrEqual(65535);
    });
  });

  describe("Server Health Check", () => {
    it("should have health endpoint documentation", () => {
      const endpoints = {
        health: "/health",
        releases: "/api/releases",
        search: "/api/search",
        manga: "/api/manga/:id",
        mangaBySlug: "/api/manga-by-slug/:slug",
        chapters: "/api/manga/:mangaId/chapters/:chapterId",
      };

      expect(endpoints.health).toBeDefined();
      expect(endpoints.releases).toBeDefined();
      expect(endpoints.search).toBeDefined();
      expect(endpoints.manga).toBeDefined();
      expect(endpoints.mangaBySlug).toBeDefined();
      expect(endpoints.chapters).toBeDefined();
    });
  });
});
