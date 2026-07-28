import axios from "axios";
import { clearTokenCache, decryptData, getAuthTokens } from "./crypto";

const API_BASE = "https://toonlivre.net/api";

// Feature flag for encryption fallback
let USE_ENCRYPTION = false;
let ENCRYPTION_LAST_CHECK = 0;
const ENCRYPTION_CHECK_INTERVAL = 5 * 60 * 1000; // 5 minutes

// Simple cache with TTL
interface CacheEntry<T> {
  data: T;
  expiresAt: number;
  cachedAt: number;
}

class SimpleCache {
  private cache: Map<string, CacheEntry<unknown>> = new Map();
  private readonly TTL = 20 * 1000; // 20 seconds

  get<T>(key: string): T | null {
    const entry = this.cache.get(key) as CacheEntry<T> | undefined;
    if (!entry) return null;

    if (Date.now() > entry.expiresAt) {
      this.cache.delete(key);
      return null;
    }

    return entry.data;
  }

  set<T>(key: string, data: T): void {
    const now = Date.now();
    this.cache.set(key, {
      data,
      expiresAt: now + this.TTL,
      cachedAt: now,
    });
  }

  clear(): void {
    this.cache.clear();
  }
}

const cache = new SimpleCache();

export interface ApiChapter {
  id: string;
  number: string;
  title?: string;
  releaseDate?: string;
  timestamp?: number;
  pageCount?: number;
}

interface ApiMangaCard {
  id: string;
  title: string;
  coverUrl?: string;
  slug?: string;
  alternativeTitle?: string;
  recent_chapters: ApiChapter[];
  registered_users_only: boolean;
}

interface ApiPagination {
  currentPage: number;
  hasNextPage: boolean;
}

export interface ApiListResponse {
  mangas: ApiMangaCard[];
  pagination: ApiPagination;
}

export interface ApiMangaById {
  id: string;
  slug: string;
  title: string;
  coverUrl?: string;
  authors: string[];
  artists: string[];
  genres: string[];
  description?: string;
  status?: string;
  alternativeTitle?: string;
  recent_chapters: ApiChapter[];
  registered_users_only: boolean;
}

export interface ApiReaderManga {
  id: string;
  title: string;
  slug?: string;
  coverUrl?: string;
  authors: string[];
  artists: string[];
  genres: string[];
  description?: string;
  status?: string;
  alternativeTitle?: string;
  chapters: ApiChapter[];
  registered_users_only: boolean;
}

export interface ApiChapterDetails {
  id: string;
  pages: string[];
  title?: string;
  number: string;
  mangaId: string;
  timestamp?: number;
  releaseDate?: string;
}

/**
 * Generate session ID (toon_i cookie)
 */
function generateSession(): string {
  return Math.random().toString(36).substring(2, 15);
}

/**
 * Request with encryption fallback
 * Tries direct access first, falls back to token-server if it fails
 */
async function requestDirect<T>(url: string): Promise<T> {
  console.log(`[api] requesting ${url}`);

  // Check cache first
  const cacheKey = `api:${url}`;
  const cached = cache.get<T>(cacheKey);
  if (cached) {
    console.log(`[cache] hit for ${url}`);
    return cached;
  }

  // Try direct access (no encryption)
  try {
    const startTime = Date.now();
    const session = generateSession();

    const response = await axios.get<T>(url, {
      headers: {
        Accept: "application/json, text/plain, */*",
        "User-Agent":
          "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36",
        "Accept-Language": "pt-BR,pt;q=0.9",
        Referer: "https://toonlivre.net/",
        Origin: "https://toonlivre.net",
        Cookie: `toon_i=${session}`,
      },
      timeout: 30000,
      validateStatus: (status) => status < 500, // Don't throw on 4xx
    });

    // Check if response indicates encryption is needed
    const responseData = response.data as Record<string, unknown>;
    const errorMessage =
      typeof responseData?.error === "string" ? responseData.error : "";
    const needsEncryption =
      response.status === 401 ||
      response.status === 403 ||
      (responseData?.error &&
        (errorMessage.includes("token") ||
          errorMessage.includes("signature") ||
          errorMessage.includes("unauthorized")));

    if (needsEncryption && !USE_ENCRYPTION) {
      console.log(
        `[api] Direct access failed for ${url}, enabling encryption fallback`,
      );
      USE_ENCRYPTION = true;
      ENCRYPTION_LAST_CHECK = Date.now();

      // Retry with encryption
      return requestWithEncryption<T>(url);
    }

    if (response.status >= 400) {
      throw new Error(`Request failed with status code ${response.status}`);
    }

    const requestTime = Date.now() - startTime;
    console.log(`[api] request completed in ${requestTime}ms for ${url}`);

    // Store in cache
    cache.set(cacheKey, response.data);
    return response.data;
  } catch (error) {
    // Check if we should try encryption fallback
    const now = Date.now();
    if (
      !USE_ENCRYPTION &&
      now - ENCRYPTION_LAST_CHECK > ENCRYPTION_CHECK_INTERVAL
    ) {
      console.log(
        `[api] Direct access error for ${url}, trying encryption fallback`,
      );
      ENCRYPTION_LAST_CHECK = now;

      try {
        return await requestWithEncryption<T>(url);
      } catch (encryptionError) {
        console.error(
          "[api] Encryption fallback also failed:",
          encryptionError,
        );
        throw error; // Throw original error
      }
    }

    console.error(`[api] request failed for ${url}:`, error);
    throw error;
  }
}

/**
 * Request with encryption (using built-in crypto with bundle execution)
 */
async function requestWithEncryption<T>(url: string): Promise<T> {
  console.log(`[api] using encryption for ${url}`);

  try {
    // Get authentication tokens and passphrase from bundle execution
    const { signature, verify, session } = await getAuthTokens();

    // Make request with encryption headers
    const response = await axios.get(url, {
      headers: {
        Accept: "application/json, text/plain, */*",
        "User-Agent":
          "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36",
        "Accept-Language": "pt-BR,pt;q=0.9",
        Referer: "https://toonlivre.net/",
        Origin: "https://toonlivre.net",
        Cookie: `toon_i=${session}`,
        "x-toon-signature": signature,
        "x-toon-verify": verify,
      },
      timeout: 30000,
    });

    // Check if response is encrypted (string instead of JSON)
    const data = response.data;
    if (typeof data === "string" && data.length > 0) {
      console.log(`[api] decrypting response from ${url}`);

      try {
        const decrypted = await decryptData(data);
        return JSON.parse(decrypted) as T;
      } catch (decryptError) {
        console.log("[api] response not encrypted, returning as-is");
        return data as T;
      }
    }

    console.log("[api] encryption successful, switching to encrypted mode");
    USE_ENCRYPTION = true;

    return response.data;
  } catch (error) {
    console.error("[api] encryption request failed:", error);
    throw error;
  }
}

export async function fetchReleases(
  page = 1,
  limit = 48,
): Promise<ApiListResponse> {
  const url = `${API_BASE}/mangas/releases?page=${page}&limit=${limit}`;
  return requestDirect<ApiListResponse>(url);
}

export async function searchMangas(
  query: string,
  page = 1,
  limit = 24,
): Promise<ApiListResponse> {
  const encoded = encodeURIComponent(query.trim());
  const url = `${API_BASE}/mangas/search?q=${encoded}&page=${page}&limit=${limit}&sortBy=updated&sortOrder=desc`;
  return requestDirect<ApiListResponse>(url);
}

export async function fetchMangaById(id: string): Promise<ApiMangaById> {
  const url = `${API_BASE}/mangas/${id}`;
  return requestDirect<ApiMangaById>(url);
}

export async function fetchMangaReader(id: string): Promise<ApiReaderManga> {
  const url = `${API_BASE}/mangas/${id}/reader`;
  return requestDirect<ApiReaderManga>(url);
}

export async function fetchMangaBySlug(slug: string): Promise<ApiMangaById> {
  const encoded = encodeURIComponent(slug.trim().replace(/^\/|\/$/g, ""));
  const url = `${API_BASE}/manga-by-slug/${encoded}`;
  return requestDirect<ApiMangaById>(url);
}

export async function fetchChapterDetails(
  mangaId: string,
  chapterId: string,
): Promise<ApiChapterDetails> {
  const url = `${API_BASE}/mangas/${mangaId}/chapters/${chapterId}`;
  return requestDirect<ApiChapterDetails>(url);
}

/**
 * Get current encryption status
 */
export function getEncryptionStatus(): {
  enabled: boolean;
  lastCheck: number;
} {
  return {
    enabled: USE_ENCRYPTION,
    lastCheck: ENCRYPTION_LAST_CHECK,
  };
}

/**
 * Manually enable/disable encryption mode
 */
export function setEncryptionMode(enabled: boolean): void {
  USE_ENCRYPTION = enabled;
  ENCRYPTION_LAST_CHECK = Date.now();
  console.log(`[api] Encryption mode ${enabled ? "enabled" : "disabled"}`);
}

/**
 * Clear API cache and token cache
 */
export function clearCache(): void {
  cache.clear();
  clearTokenCache();
  console.log("[api] Cache and tokens cleared");
}
