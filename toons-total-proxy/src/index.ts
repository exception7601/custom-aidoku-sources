import axios, { type AxiosInstance } from "axios";
import { Elysia, t } from "elysia";
import {
  fetchChapterDetails,
  fetchMangaById,
  fetchMangaBySlug,
  fetchMangaReader,
  fetchReleases,
  searchMangas,
} from "./toonlivre-api";

// Internal token server
interface TokenResponse {
  session?: string;
  passphrase?: string;
  headers?: {
    "x-toon-signature": string;
    "x-toon-verify": string;
  };
  strategy?: string;
  expiresAt?: number;
}

interface CacheEntry<T> {
  data: T;
  expiresAt: number;
}

class InternalTokenServer {
  private cache: Map<string, CacheEntry<unknown>> = new Map();
  private client: AxiosInstance;
  private ttl = 20; // 20 seconds

  constructor() {
    this.client = axios.create({
      timeout: 30000,
      headers: {
        "Content-Type": "application/json",
      },
    });
  }

  async getTokens(url: string): Promise<TokenResponse> {
    const cacheKey = `tokens:${url}`;

    // Check cache
    const cached = this.cache.get(cacheKey) as
      | CacheEntry<TokenResponse>
      | undefined;
    if (cached && cached.expiresAt > Date.now()) {
      console.log(`[token-cache-hit] ${cacheKey}`);
      return cached.data;
    }

    // Fetch new token from real server
    try {
      const response = await this.client.post<TokenResponse>(
        "https://toons.4nd.xyz/api/tokens",
        { url },
      );

      // Store in cache
      this.cache.set(cacheKey, {
        data: response.data,
        expiresAt: Date.now() + this.ttl * 1000,
      });

      console.log(`[token-fetched] ${cacheKey}`);
      return response.data;
    } catch (error) {
      console.error(`[token-error] Failed to get tokens for ${url}`, error);
      throw error;
    }
  }

  async getHealth(): Promise<boolean> {
    try {
      await this.client.get("https://toons.4nd.xyz/health");
      return true;
    } catch (error) {
      console.error("[token-health-error]", error);
      return false;
    }
  }
}

const tokenServer = new InternalTokenServer();

const app = new Elysia();

app.get("/", () => ({
  name: "Toons Total Proxy",
  version: "1.0.0",
  endpoints: {
    health: "/health",
    releases: "/api/releases",
    search: "/api/search",
    manga: "/api/manga/:id",
    mangaBySlug: "/api/manga-by-slug/:slug",
    chapters: "/api/manga/:id/chapters/:chapterId",
  },
}));

app.get("/health", async () => {
  const isHealthy = await tokenServer.getHealth();
  return {
    status: isHealthy ? "ok" : "unhealthy",
    timestamp: new Date().toISOString(),
  };
});

app.get(
  "/api/releases",
  async ({ query }) => {
    try {
      const page = Number.parseInt(query.page as string) || 1;
      const limit = Number.parseInt(query.limit as string) || 48;

      console.log(`[releases] page=${page} limit=${limit}`);

      const result = await fetchReleases(page, limit);
      return {
        success: true,
        data: result,
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
  },
  {
    query: t.Object({
      page: t.Optional(t.String()),
      limit: t.Optional(t.String()),
    }),
  },
);

app.get(
  "/api/search",
  async ({ query }) => {
    try {
      const q = query.q as string;
      if (!q || q.trim().length === 0) {
        return {
          success: false,
          error: "Query parameter 'q' is required",
          timestamp: new Date().toISOString(),
        };
      }

      const page = Number.parseInt(query.page as string) || 1;
      const limit = Number.parseInt(query.limit as string) || 24;

      console.log(`[search] q=${q} page=${page} limit=${limit}`);

      const result = await searchMangas(q, page, limit);
      return {
        success: true,
        data: result,
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
  },
  {
    query: t.Object({
      q: t.Optional(t.String()),
      page: t.Optional(t.String()),
      limit: t.Optional(t.String()),
    }),
  },
);

app.get("/api/manga/:id", async ({ params }) => {
  try {
    const { id } = params;
    console.log(`[manga] id=${id}`);

    const result = await fetchMangaById(id);
    return {
      success: true,
      data: result,
      timestamp: new Date().toISOString(),
    };
  } catch (error) {
    console.error("[manga error]", error);
    return {
      success: false,
      error: error instanceof Error ? error.message : "Unknown error",
      timestamp: new Date().toISOString(),
    };
  }
});

app.get("/api/manga/:id/reader", async ({ params }) => {
  try {
    const { id } = params;
    console.log(`[manga-reader] id=${id}`);

    const result = await fetchMangaReader(id);
    return {
      success: true,
      data: result,
      timestamp: new Date().toISOString(),
    };
  } catch (error) {
    console.error("[manga-reader error]", error);
    return {
      success: false,
      error: error instanceof Error ? error.message : "Unknown error",
      timestamp: new Date().toISOString(),
    };
  }
});

app.get("/api/manga-by-slug/:slug", async ({ params }) => {
  try {
    const { slug } = params;
    console.log(`[manga-by-slug] slug=${slug}`);

    const result = await fetchMangaBySlug(slug);
    return {
      success: true,
      data: result,
      timestamp: new Date().toISOString(),
    };
  } catch (error) {
    console.error("[manga-by-slug error]", error);
    return {
      success: false,
      error: error instanceof Error ? error.message : "Unknown error",
      timestamp: new Date().toISOString(),
    };
  }
});

app.get("/api/manga/:id/chapters/:chapterId", async ({ params }) => {
  try {
    const { id, chapterId } = params;
    console.log(`[chapter] mangaId=${id} chapterId=${chapterId}`);

    const result = await fetchChapterDetails(id, chapterId);
    return {
      success: true,
      data: result,
      timestamp: new Date().toISOString(),
    };
  } catch (error) {
    console.error("[chapter error]", error);
    return {
      success: false,
      error: error instanceof Error ? error.message : "Unknown error",
      timestamp: new Date().toISOString(),
    };
  }
});

const PORT = Number.parseInt(process.env.PORT || "3000");
app.listen(PORT, () => {
  console.log(`[server] listening on http://localhost:${PORT}`);
});
