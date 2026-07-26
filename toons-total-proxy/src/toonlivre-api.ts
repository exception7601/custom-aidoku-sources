import axios from "axios";
import { tokenServer } from "./token-server";

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

async function requestWithTokens<T>(url: string): Promise<T> {
  console.log(`[api] requesting ${url}`);

  const tokens = await tokenServer.getTokens(url);

  if (!tokens.headers) {
    throw new Error("Failed to get required headers from token server");
  }

  const response = await axios.get<T>(url, {
    headers: {
      Accept: "application/json, text/plain, */*",
      "User-Agent":
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36",
      "Accept-Language": "pt-BR,pt;q=0.9",
      Referer: "https://toonlivre.net/",
      "x-toon-signature": tokens.headers["x-toon-signature"],
      "x-toon-verify": tokens.headers["x-toon-verify"],
    },
  });

  return response.data;
}

export async function fetchReleases(
  page = 1,
  limit = 48,
): Promise<ApiListResponse> {
  const url = `${API_BASE}/mangas/releases?page=${page}&limit=${limit}`;
  return requestWithTokens<ApiListResponse>(url);
}

export async function searchMangas(
  query: string,
  page = 1,
  limit = 24,
): Promise<ApiListResponse> {
  const encoded = encodeURIComponent(query.trim());
  const url = `${API_BASE}/mangas/search?q=${encoded}&page=${page}&limit=${limit}&sortBy=updated&sortOrder=desc`;
  return requestWithTokens<ApiListResponse>(url);
}

export async function fetchMangaById(id: string): Promise<ApiMangaById> {
  const url = `${API_BASE}/mangas/${id}`;
  return requestWithTokens<ApiMangaById>(url);
}

export async function fetchMangaReader(id: string): Promise<ApiReaderManga> {
  const url = `${API_BASE}/mangas/${id}/reader`;
  return requestWithTokens<ApiReaderManga>(url);
}

export async function fetchMangaBySlug(slug: string): Promise<ApiMangaById> {
  const encoded = encodeURIComponent(slug.trim().replace(/^\/|\/$/g, ""));
  const url = `${API_BASE}/manga-by-slug/${encoded}`;
  return requestWithTokens<ApiMangaById>(url);
}

export async function fetchChapterDetails(
  mangaId: string,
  chapterId: string,
): Promise<ApiChapterDetails> {
  const url = `${API_BASE}/mangas/${mangaId}/chapters/${chapterId}`;
  return requestWithTokens<ApiChapterDetails>(url);
}
