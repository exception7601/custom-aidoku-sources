import { Elysia } from "elysia";
import { accessLogger } from "./logger";
import {
  clearCache,
  fetchChapterDetails,
  fetchMangaById,
  fetchMangaBySlug,
  fetchMangaReader,
  fetchReleases,
  getEncryptionStatus,
  searchMangas,
  setEncryptionMode,
} from "./toonlivre-api";

const app = new Elysia();

// Helper to log requests
function logRequest(
  method: string,
  path: string,
  status: number,
  responseTime: number,
  ip?: string,
  userAgent?: string,
  error?: string,
) {
  accessLogger.log({
    ip: ip || "unknown",
    method,
    path,
    status,
    responseTime,
    userAgent,
    error,
  });
}

app.get("/", () => ({
  name: "Toons Total Proxy",
  version: "2.0.0",
  description:
    "Direct API access with encryption fallback - compatible with token-server",
  endpoints: {
    health: "/health",
    releases: "/api/releases",
    search: "/api/search",
    manga: "/api/manga/:id",
    mangaBySlug: "/api/manga-by-slug/:slug",
    reader: "/api/manga/:id/reader",
    chapters: "/api/manga/:id/chapters/:chapterId",
    encryption: "/api/encryption/status",
    logs: "/api/logs",
    logsStats: "/api/logs/stats",
  },
}));

app.get("/health", ({ headers }) => {
  const startTime = Date.now();
  const encStatus = getEncryptionStatus();
  const response = {
    status: "ok",
    timestamp: new Date().toISOString(),
    encryption: {
      enabled: encStatus.enabled,
      lastCheck: new Date(encStatus.lastCheck).toISOString(),
    },
  };
  logRequest(
    "GET",
    "/health",
    200,
    Date.now() - startTime,
    undefined,
    headers["user-agent"],
  );
  return response;
});

app.get("/api/releases", async ({ query, headers }) => {
  const startTime = Date.now();
  try {
    const page = query.page ? Number.parseInt(query.page as string) : 1;
    const limit = query.limit ? Number.parseInt(query.limit as string) : 48;
    const data = await fetchReleases(page, limit);
    logRequest(
      "GET",
      "/api/releases",
      200,
      Date.now() - startTime,
      undefined,
      headers["user-agent"],
    );
    return {
      success: true,
      data,
      timestamp: new Date().toISOString(),
    };
  } catch (error) {
    console.error("[releases error]", error);
    const errorMsg = error instanceof Error ? error.message : "Unknown error";
    logRequest(
      "GET",
      "/api/releases",
      500,
      Date.now() - startTime,
      undefined,
      headers["user-agent"],
      errorMsg,
    );
    return {
      success: false,
      error: errorMsg,
      timestamp: new Date().toISOString(),
    };
  }
});

app.get("/api/search", async ({ query, headers }) => {
  const startTime = Date.now();
  try {
    const q = query.q as string;
    const page = query.page ? Number.parseInt(query.page as string) : 1;
    const limit = query.limit ? Number.parseInt(query.limit as string) : 24;

    if (!q) {
      logRequest(
        "GET",
        "/api/search",
        400,
        Date.now() - startTime,
        undefined,
        headers["user-agent"],
        "Missing query",
      );
      return {
        success: false,
        error: "Query parameter 'q' is required",
        timestamp: new Date().toISOString(),
      };
    }

    const data = await searchMangas(q, page, limit);
    logRequest(
      "GET",
      "/api/search",
      200,
      Date.now() - startTime,
      undefined,
      headers["user-agent"],
    );
    return {
      success: true,
      data,
      timestamp: new Date().toISOString(),
    };
  } catch (error) {
    console.error("[search error]", error);
    const errorMsg = error instanceof Error ? error.message : "Unknown error";
    logRequest(
      "GET",
      "/api/search",
      500,
      Date.now() - startTime,
      undefined,
      headers["user-agent"],
      errorMsg,
    );
    return {
      success: false,
      error: errorMsg,
      timestamp: new Date().toISOString(),
    };
  }
});

app.get("/api/manga/:id", async ({ params, headers }) => {
  const startTime = Date.now();
  try {
    const data = await fetchMangaById(params.id);
    logRequest(
      "GET",
      `/api/manga/${params.id}`,
      200,
      Date.now() - startTime,
      undefined,
      headers["user-agent"],
    );
    return {
      success: true,
      data,
      timestamp: new Date().toISOString(),
    };
  } catch (error) {
    console.error("[manga by id error]", error);
    const errorMsg = error instanceof Error ? error.message : "Unknown error";
    logRequest(
      "GET",
      `/api/manga/${params.id}`,
      500,
      Date.now() - startTime,
      undefined,
      headers["user-agent"],
      errorMsg,
    );
    return {
      success: false,
      error: errorMsg,
      timestamp: new Date().toISOString(),
    };
  }
});

app.get("/api/manga/:id/reader", async ({ params, headers }) => {
  const startTime = Date.now();
  try {
    const data = await fetchMangaReader(params.id);
    logRequest(
      "GET",
      `/api/manga/${params.id}/reader`,
      200,
      Date.now() - startTime,
      undefined,
      headers["user-agent"],
    );
    return {
      success: true,
      data,
      timestamp: new Date().toISOString(),
    };
  } catch (error) {
    console.error("[manga reader error]", error);
    const errorMsg = error instanceof Error ? error.message : "Unknown error";
    logRequest(
      "GET",
      `/api/manga/${params.id}/reader`,
      500,
      Date.now() - startTime,
      undefined,
      headers["user-agent"],
      errorMsg,
    );
    return {
      success: false,
      error: errorMsg,
      timestamp: new Date().toISOString(),
    };
  }
});

app.get("/api/manga-by-slug/:slug", async ({ params, headers }) => {
  const startTime = Date.now();
  try {
    const data = await fetchMangaBySlug(params.slug);
    logRequest(
      "GET",
      `/api/manga-by-slug/${params.slug}`,
      200,
      Date.now() - startTime,
      undefined,
      headers["user-agent"],
    );
    return {
      success: true,
      data,
      timestamp: new Date().toISOString(),
    };
  } catch (error) {
    console.error("[manga by slug error]", error);
    const errorMsg = error instanceof Error ? error.message : "Unknown error";
    logRequest(
      "GET",
      `/api/manga-by-slug/${params.slug}`,
      500,
      Date.now() - startTime,
      undefined,
      headers["user-agent"],
      errorMsg,
    );
    return {
      success: false,
      error: errorMsg,
      timestamp: new Date().toISOString(),
    };
  }
});

app.get("/api/manga/:id/chapters/:chapterId", async ({ params, headers }) => {
  const startTime = Date.now();
  try {
    const data = await fetchChapterDetails(params.id, params.chapterId);
    logRequest(
      "GET",
      `/api/manga/${params.id}/chapters/${params.chapterId}`,
      200,
      Date.now() - startTime,
      undefined,
      headers["user-agent"],
    );
    return {
      success: true,
      data,
      timestamp: new Date().toISOString(),
    };
  } catch (error) {
    console.error("[chapter details error]", error);
    const errorMsg = error instanceof Error ? error.message : "Unknown error";
    logRequest(
      "GET",
      `/api/manga/${params.id}/chapters/${params.chapterId}`,
      500,
      Date.now() - startTime,
      undefined,
      headers["user-agent"],
      errorMsg,
    );
    return {
      success: false,
      error: errorMsg,
      timestamp: new Date().toISOString(),
    };
  }
});

app.get("/api/encryption/status", ({ headers }) => {
  const startTime = Date.now();
  const status = getEncryptionStatus();
  logRequest(
    "GET",
    "/api/encryption/status",
    200,
    Date.now() - startTime,
    undefined,
    headers["user-agent"],
  );
  return {
    success: true,
    data: {
      enabled: status.enabled,
      lastCheck: new Date(status.lastCheck).toISOString(),
      mode: status.enabled ? "encrypted" : "direct",
    },
    timestamp: new Date().toISOString(),
  };
});

app.post("/api/encryption/toggle", async ({ body, headers }) => {
  const startTime = Date.now();
  try {
    const { enabled } = body as { enabled: boolean };
    setEncryptionMode(enabled);
    logRequest(
      "POST",
      "/api/encryption/toggle",
      200,
      Date.now() - startTime,
      undefined,
      headers["user-agent"],
    );
    return {
      success: true,
      data: {
        enabled,
        mode: enabled ? "encrypted" : "direct",
      },
      timestamp: new Date().toISOString(),
    };
  } catch (error) {
    console.error("[encryption toggle error]", error);
    const errorMsg = error instanceof Error ? error.message : "Unknown error";
    logRequest(
      "POST",
      "/api/encryption/toggle",
      500,
      Date.now() - startTime,
      undefined,
      headers["user-agent"],
      errorMsg,
    );
    return {
      success: false,
      error: errorMsg,
      timestamp: new Date().toISOString(),
    };
  }
});

app.post("/api/cache/clear", ({ headers }) => {
  const startTime = Date.now();
  clearCache();
  logRequest(
    "POST",
    "/api/cache/clear",
    200,
    Date.now() - startTime,
    undefined,
    headers["user-agent"],
  );
  return {
    success: true,
    message: "Cache cleared",
    timestamp: new Date().toISOString(),
  };
});

app.get("/api/logs", ({ query, headers }) => {
  const startTime = Date.now();
  const limit = query.limit ? Number.parseInt(query.limit as string) : 100;
  const logs = accessLogger.getLogs(limit);
  logRequest(
    "GET",
    "/api/logs",
    200,
    Date.now() - startTime,
    undefined,
    headers["user-agent"],
  );
  return {
    success: true,
    data: logs,
    count: logs.length,
    timestamp: new Date().toISOString(),
  };
});

app.get("/api/logs/stats", ({ headers }) => {
  const startTime = Date.now();
  const stats = accessLogger.getStats();
  logRequest(
    "GET",
    "/api/logs/stats",
    200,
    Date.now() - startTime,
    undefined,
    headers["user-agent"],
  );
  return {
    success: true,
    data: stats,
    timestamp: new Date().toISOString(),
  };
});

app.delete("/api/logs", ({ headers }) => {
  const startTime = Date.now();
  accessLogger.clear();
  logRequest(
    "DELETE",
    "/api/logs",
    200,
    Date.now() - startTime,
    undefined,
    headers["user-agent"],
  );
  return {
    success: true,
    message: "Logs cleared",
    timestamp: new Date().toISOString(),
  };
});

app.listen({ port: process.env.PORT || 4000, hostname: "0.0.0.0" }, () => {
  console.log(
    `[proxy] listening on http://0.0.0.0:${process.env.PORT || 4000}`,
  );
});

export default app;
