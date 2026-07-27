import axios from "axios";
import { tokenManager } from "./token-manager";

const API_BASE = "https://toonlivre.net/api";

export interface ApiChapter {
  id: string;
  number: string;
  title?: string;
  releaseDate?: string;
  timestamp?: number;
  pageCount?: number;
}

export interface ApiMangaCard {
  id: string;
  title: string;
  coverUrl?: string;
  slug?: string;
  alternativeTitle?: string;
  recent_chapters: ApiChapter[];
  registered_users_only: boolean;
}

export interface ApiPagination {
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

async function requestDirect<T>(url: string): Promise<T> {
  console.log(`[api] requesting ${url}`);

  // Check cache first
  const cacheKey = `api:${url}`;
  const cached = tokenManager.getFromCache(cacheKey);
  if (cached) {
    return cached as T;
  }

  try {
    const startTime = Date.now();

    // Get token for request
    const tokenData = await tokenManager.getToken();

    const response = await axios.get<T>(url, {
      headers: {
        Accept: "application/json, text/plain, */*",
        "User-Agent":
          "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36",
        "Accept-Language": "pt-BR,pt;q=0.9",
        Referer: "https://toonlivre.net/",
        Authorization: `Bearer ${tokenData.token}`,
      },
      timeout: 30000,
    });

    const requestTime = Date.now() - startTime;
    console.log(`[api] request completed in ${requestTime}ms for ${url}`);

    // Store in cache with request time
    tokenManager.setCache(
      cacheKey,
      response.data,
      requestTime,
      tokenManager.REQUEST_CACHE_TTL,
    );
    return response.data;
  } catch (error) {
    console.error(`[api] request failed for ${url}:`, error);
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
