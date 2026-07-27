import { Elysia } from "elysia";
import {
  fetchChapterDetails,
  fetchMangaById,
  fetchMangaBySlug,
  fetchMangaReader,
  fetchReleases,
  searchMangas,
} from "./toonlivre-api";

const app = new Elysia();

app.get("/", () => ({
  name: "Toons Total Proxy",
  version: "1.1.0",
  endpoints: {
    health: "/health",
    releases: "/api/releases",
    search: "/api/search",
    manga: "/api/manga/:id",
    mangaBySlug: "/api/manga-by-slug/:slug",
    chapters: "/api/manga/:id/chapters/:chapterId",
  },
}));

app.get("/health", () => ({
  status: "ok",
  timestamp: new Date().toISOString(),
}));

app.get("/api/releases", async ({ query }) => {
  try {
    const page = query.page ? Number.parseInt(query.page as string) : 1;
    const limit = query.limit ? Number.parseInt(query.limit as string) : 48;
    const data = await fetchReleases(page, limit);
    return {
      success: true,
      data,
      timestamp: new Date().toISOString(),
    };
  } catch (error) {
    console.error("[releases error]", error);
    return {
      success: false,
      error: error instanceof Error ? error.message : "Unknown error",
      timestamp: new Date().toISOString(),
    };
  }
});

app.get("/api/search", async ({ query }) => {
  try {
    const q = query.q as string;
    const page = query.page ? Number.parseInt(query.page as string) : 1;
    const limit = query.limit ? Number.parseInt(query.limit as string) : 24;

    if (!q) {
      return {
        success: false,
        error: "Query parameter 'q' is required",
        timestamp: new Date().toISOString(),
      };
    }

    const data = await searchMangas(q, page, limit);
    return {
      success: true,
      data,
      timestamp: new Date().toISOString(),
    };
  } catch (error) {
    console.error("[search error]", error);
    return {
      success: false,
      error: error instanceof Error ? error.message : "Unknown error",
      timestamp: new Date().toISOString(),
    };
  }
});

app.get("/api/manga/:id", async ({ params }) => {
  try {
    const data = await fetchMangaById(params.id);
    return {
      success: true,
      data,
      timestamp: new Date().toISOString(),
    };
  } catch (error) {
    console.error("[manga by id error]", error);
    return {
      success: false,
      error: error instanceof Error ? error.message : "Unknown error",
      timestamp: new Date().toISOString(),
    };
  }
});

app.get("/api/manga/:id/reader", async ({ params }) => {
  try {
    const data = await fetchMangaReader(params.id);
    return {
      success: true,
      data,
      timestamp: new Date().toISOString(),
    };
  } catch (error) {
    console.error("[manga reader error]", error);
    return {
      success: false,
      error: error instanceof Error ? error.message : "Unknown error",
      timestamp: new Date().toISOString(),
    };
  }
});

app.get("/api/manga-by-slug/:slug", async ({ params }) => {
  try {
    const data = await fetchMangaBySlug(params.slug);
    return {
      success: true,
      data,
      timestamp: new Date().toISOString(),
    };
  } catch (error) {
    console.error("[manga by slug error]", error);
    return {
      success: false,
      error: error instanceof Error ? error.message : "Unknown error",
      timestamp: new Date().toISOString(),
    };
  }
});

app.get("/api/manga/:id/chapters/:chapterId", async ({ params }) => {
  try {
    const data = await fetchChapterDetails(params.id, params.chapterId);
    return {
      success: true,
      data,
      timestamp: new Date().toISOString(),
    };
  } catch (error) {
    console.error("[chapter details error]", error);
    return {
      success: false,
      error: error instanceof Error ? error.message : "Unknown error",
      timestamp: new Date().toISOString(),
    };
  }
});

app.listen(3000, () => {
  console.log("[proxy] listening on http://localhost:3000");
});

export default app;
