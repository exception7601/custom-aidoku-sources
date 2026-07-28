import { describe, expect, it, beforeAll, afterAll } from "bun:test";

describe("Toons Total Proxy - Encryption Fallback Tests", () => {
  const BASE_URL = process.env.TEST_BASE_URL || "http://localhost:4001";

  describe("Encryption Status Endpoints", () => {
    it("should return encryption status in health endpoint", async () => {
      const response = await fetch(`${BASE_URL}/health`);
      const data = await response.json();

      expect(response.status).toBe(200);
      expect(data.status).toBe("ok");
      expect(data.encryption).toBeDefined();
      expect(typeof data.encryption.enabled).toBe("boolean");
      expect(data.encryption.lastCheck).toBeDefined();

      console.log(
        `[health] Encryption mode: ${data.encryption.enabled ? "ENABLED" : "DISABLED"}`,
      );
    });

    it("should get encryption status from dedicated endpoint", async () => {
      const response = await fetch(`${BASE_URL}/api/encryption/status`);
      const data = await response.json();

      expect(response.status).toBe(200);
      expect(data.success).toBe(true);
      expect(data.data.enabled).toBeDefined();
      expect(data.data.mode).toMatch(/^(direct|encrypted)$/);

      console.log(`[encryption] Current mode: ${data.data.mode}`);
    });
  });

  describe("Encryption Toggle", () => {
    it("should toggle encryption mode on", async () => {
      const response = await fetch(`${BASE_URL}/api/encryption/toggle`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ enabled: true }),
      });

      const data = await response.json();

      expect(response.status).toBe(200);
      expect(data.success).toBe(true);
      expect(data.data.enabled).toBe(true);
      expect(data.data.mode).toBe("encrypted");

      console.log("[encryption] Mode set to: encrypted");
    });

    it("should toggle encryption mode off", async () => {
      const response = await fetch(`${BASE_URL}/api/encryption/toggle`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ enabled: false }),
      });

      const data = await response.json();

      expect(response.status).toBe(200);
      expect(data.success).toBe(true);
      expect(data.data.enabled).toBe(false);
      expect(data.data.mode).toBe("direct");

      console.log("[encryption] Mode set to: direct");
    });
  });

  describe("Cache Management", () => {
    it("should clear cache successfully", async () => {
      const response = await fetch(`${BASE_URL}/api/cache/clear`, {
        method: "POST",
      });

      const data = await response.json();

      expect(response.status).toBe(200);
      expect(data.success).toBe(true);
      expect(data.message).toContain("cleared");

      console.log("[cache] Cache cleared successfully");
    });
  });

  describe("Direct Mode - All Endpoints", () => {
    beforeAll(async () => {
      // Ensure direct mode is enabled
      await fetch(`${BASE_URL}/api/encryption/toggle`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ enabled: false }),
      });

      // Clear cache for fresh tests
      await fetch(`${BASE_URL}/api/cache/clear`, { method: "POST" });

      console.log("\n[test] Starting direct mode tests...\n");
    });

    it("should fetch releases in direct mode", async () => {
      const response = await fetch(`${BASE_URL}/api/releases?page=1&limit=3`);
      const data = await response.json();

      expect(response.status).toBe(200);
      expect(data.success).toBe(true);
      expect(data.data.mangas).toBeDefined();
      expect(Array.isArray(data.data.mangas)).toBe(true);

      console.log(
        `[direct] Releases: ${data.data.mangas.length} mangas fetched`,
      );
    });

    it("should search mangas in direct mode", async () => {
      const response = await fetch(
        `${BASE_URL}/api/search?q=demon&page=1&limit=3`,
      );
      const data = await response.json();

      expect(response.status).toBe(200);
      expect(data.success).toBe(true);
      expect(data.data.mangas).toBeDefined();

      console.log(
        `[direct] Search: ${data.data.mangas.length} results for "demon"`,
      );
    });

    it("should fetch manga by slug in direct mode", async () => {
      const response = await fetch(
        `${BASE_URL}/api/manga-by-slug/contos-de-demonios-e-deuses`,
      );
      const data = await response.json();

      expect(response.status).toBe(200);
      expect(data.success).toBe(true);
      expect(data.data.id).toBeDefined();
      expect(data.data.title).toBeDefined();

      console.log(`[direct] Manga by slug: ${data.data.title}`);
    });

    it("should fetch manga by id in direct mode", async () => {
      const mangaResponse = await fetch(
        `${BASE_URL}/api/manga-by-slug/contos-de-demonios-e-deuses`,
      );
      const mangaData = await mangaResponse.json();
      const mangaId = mangaData.data.id;

      const response = await fetch(`${BASE_URL}/api/manga/${mangaId}`);
      const data = await response.json();

      expect(response.status).toBe(200);
      expect(data.success).toBe(true);
      expect(data.data.id).toBe(mangaId);

      console.log(`[direct] Manga by ID: ${data.data.title}`);
    });

    it("should fetch reader in direct mode", async () => {
      const mangaResponse = await fetch(
        `${BASE_URL}/api/manga-by-slug/contos-de-demonios-e-deuses`,
      );
      const mangaData = await mangaResponse.json();
      const mangaId = mangaData.data.id;

      const response = await fetch(`${BASE_URL}/api/manga/${mangaId}/reader`);
      const data = await response.json();

      expect(response.status).toBe(200);
      expect(data.success).toBe(true);
      expect(data.data.chapters).toBeDefined();

      console.log(
        `[direct] Reader: ${data.data.chapters.length} chapters available`,
      );
    });

    it("should handle chapter not found gracefully in direct mode", async () => {
      const response = await fetch(
        `${BASE_URL}/api/manga/obra-test/chapters/cap-test`,
      );
      const data = await response.json();

      expect(data.success).toBe(false);
      expect(data.error).toBeDefined();

      console.log(`[direct] Chapter not found handled correctly`);
    });
  });

  describe("Fallback Behavior", () => {
    it("should report current mode after multiple requests", async () => {
      // Make multiple requests
      await fetch(`${BASE_URL}/api/releases?page=1&limit=1`);
      await fetch(`${BASE_URL}/api/search?q=test&page=1&limit=1`);

      // Check if mode changed
      const statusResponse = await fetch(`${BASE_URL}/api/encryption/status`);
      const statusData = await statusResponse.json();

      console.log(
        `\n[fallback] Final mode: ${statusData.data.mode} (enabled: ${statusData.data.enabled})`,
      );
      console.log(
        `[fallback] Last check: ${statusData.data.lastCheck}\n`,
      );

      expect(statusData.success).toBe(true);
      expect(statusData.data.mode).toBeDefined();
    });
  });

  afterAll(async () => {
    // Reset to direct mode
    await fetch(`${BASE_URL}/api/encryption/toggle`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ enabled: false }),
    });

    console.log("\n[test] All fallback tests completed\n");
  });
});
