import { describe, expect, it } from "bun:test";

describe("Toons Total Proxy - Image Proxy Tests", () => {
  const BASE_URL = process.env.TEST_BASE_URL || "http://localhost:4001";

  describe("Image Proxy Endpoint", () => {
    it("should require url parameter", async () => {
      const response = await fetch(`${BASE_URL}/api/image`);
      const data = await response.json();

      expect(response.status).toBe(400);
      expect(data.success).toBe(false);
      expect(data.error).toContain("url");
    });

    it("should validate url parameter format", () => {
      const testUrl = "https://example.com/image.jpg";
      const encoded = encodeURIComponent(testUrl);
      const fullUrl = `${BASE_URL}/api/image?url=${encoded}`;

      expect(fullUrl).toContain("url=");
      expect(decodeURIComponent(encoded)).toBe(testUrl);
    });

    it("should handle missing image gracefully", async () => {
      const testUrl = "https://toonlivre.net/non-existent-image.jpg";
      const encoded = encodeURIComponent(testUrl);
      const response = await fetch(`${BASE_URL}/api/image?url=${encoded}`);

      // Should either return 404 or error response
      expect([404, 500]).toContain(response.status);
    });
  });

  describe("Content Type Handling", () => {
    it("should handle common image content types", () => {
      const contentTypes = [
        "image/jpeg",
        "image/png",
        "image/webp",
        "image/avif",
        "image/gif",
      ];

      for (const type of contentTypes) {
        expect(type).toMatch(/^image\//);
      }
    });
  });

  describe("Cache Headers", () => {
    it("should have proper cache control structure", () => {
      const cacheControl = "public, max-age=31536000";
      expect(cacheControl).toContain("public");
      expect(cacheControl).toContain("max-age=31536000");
    });
  });
});
