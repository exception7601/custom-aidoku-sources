import { describe, expect, it } from "bun:test";

describe("Toons Total Proxy - End-to-End Integration", () => {
  const BASE_URL = process.env.TEST_BASE_URL || "http://localhost:4001";

  it("should complete full workflow: search -> manga -> reader -> image", async () => {
    console.log("\n=== Starting E2E Test ===\n");

    // Step 1: Search for a manga
    console.log("[1/4] Searching for manga...");
    const searchResponse = await fetch(
      `${BASE_URL}/api/search?q=demon&page=1&limit=1`,
    );
    const searchData = await searchResponse.json();

    expect(searchData.success).toBe(true);
    expect(searchData.data.mangas.length).toBeGreaterThan(0);

    const manga = searchData.data.mangas[0];
    console.log(`✓ Found manga: ${manga.title} (${manga.id})`);

    // Step 2: Get manga details
    console.log("[2/4] Fetching manga details...");
    const mangaResponse = await fetch(`${BASE_URL}/api/manga/${manga.id}`);
    const mangaData = await mangaResponse.json();

    expect(mangaData.success).toBe(true);
    expect(mangaData.data.id).toBe(manga.id);
    console.log(`✓ Manga details loaded: ${mangaData.data.title}`);

    // Step 3: Get cover image through proxy
    if (manga.coverUrl) {
      console.log("[3/4] Proxying cover image...");
      const encodedUrl = encodeURIComponent(manga.coverUrl);
      const imageResponse = await fetch(
        `${BASE_URL}/api/image?url=${encodedUrl}`,
      );

      expect(imageResponse.status).toBe(200);
      expect(imageResponse.headers.get("content-type")).toMatch(/^image\//);

      const imageBuffer = await imageResponse.arrayBuffer();
      expect(imageBuffer.byteLength).toBeGreaterThan(0);

      console.log(
        `✓ Image proxied successfully: ${imageBuffer.byteLength} bytes, type: ${imageResponse.headers.get("content-type")}`,
      );
    } else {
      console.log("⚠ No cover URL available for this manga");
    }

    // Step 4: Get reader data
    console.log("[4/4] Fetching reader data...");
    const readerResponse = await fetch(
      `${BASE_URL}/api/manga/${manga.id}/reader`,
    );
    const readerData = await readerResponse.json();

    expect(readerData.success).toBe(true);
    expect(readerData.data.id).toBe(manga.id);

    if (readerData.data.chapters && readerData.data.chapters.length > 0) {
      console.log(
        `✓ Reader loaded with ${readerData.data.chapters.length} chapters`,
      );
    } else {
      console.log("⚠ No chapters available for this manga");
    }

    console.log("\n=== E2E Test Complete ===\n");
  });

  it("should verify no encryption/token-server required", async () => {
    console.log("\n=== Verifying No Token Server Dependency ===\n");

    // Multiple requests without any token management
    const requests = [
      fetch(`${BASE_URL}/api/releases?page=1&limit=2`),
      fetch(`${BASE_URL}/api/search?q=test&page=1&limit=2`),
      fetch(`${BASE_URL}/health`),
    ];

    const responses = await Promise.all(requests);

    for (const response of responses) {
      expect(response.ok).toBe(true);
      const data = await response.json();
      console.log(`✓ Request successful: ${response.url}`);

      // Verify no token-related fields in response
      expect(JSON.stringify(data)).not.toContain("token");
      expect(JSON.stringify(data)).not.toContain("passphrase");
      expect(JSON.stringify(data)).not.toContain("x-toon-signature");
    }

    console.log("\n=== No Token Server Dependency Confirmed ===\n");
  });

  it("should handle cache correctly", async () => {
    console.log("\n=== Testing Cache Behavior ===\n");

    const url = `${BASE_URL}/api/releases?page=1&limit=1`;

    // First request
    const start1 = Date.now();
    const response1 = await fetch(url);
    const time1 = Date.now() - start1;
    const data1 = await response1.json();

    console.log(`First request: ${time1}ms`);
    expect(data1.success).toBe(true);

    // Second request (should be cached)
    const start2 = Date.now();
    const response2 = await fetch(url);
    const time2 = Date.now() - start2;
    const data2 = await response2.json();

    console.log(`Cached request: ${time2}ms`);
    expect(data2.success).toBe(true);

    // Data should be identical
    expect(JSON.stringify(data1.data)).toBe(JSON.stringify(data2.data));
    console.log("✓ Cache working: both requests returned identical data");

    console.log("\n=== Cache Test Complete ===\n");
  });
});
