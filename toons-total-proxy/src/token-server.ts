import axios, { type AxiosInstance } from "axios";

interface TokenServerConfig {
  host: string;
  endpoints: {
    tokens: string;
    health: string;
  };
  timeout: {
    connect: number;
    request: number;
  };
  cache: {
    enabled: boolean;
    ttl: number;
  };
}

interface TokenServerResponse {
  session?: string;
  passphrase?: string;
  headers?: {
    "x-toon-signature": string;
    "x-toon-verify": string;
  };
  strategy?: string;
  expiresAt?: number;
  cached?: boolean;
  expiresIn?: number;
}

interface CacheEntry<T> {
  data: T;
  expiresAt: number;
}

class TokenServerClient {
  private client: AxiosInstance;
  private config: TokenServerConfig;
  private cache: Map<string, CacheEntry<unknown>> = new Map();

  constructor() {
    this.config = {
      host: process.env.TOKEN_SERVER_HOST || "http://localhost:3001",
      endpoints: {
        tokens: "/api/tokens",
        health: "/health",
      },
      timeout: {
        connect: 10000,
        request: 30000,
      },
      cache: {
        enabled: true,
        ttl: 20,
      },
    };

    this.client = axios.create({
      baseURL: this.config.host,
      timeout: this.config.timeout.request,
      headers: {
        "Content-Type": "application/json",
      },
    });
  }

  async getTokens(url: string): Promise<TokenServerResponse> {
    const cacheKey = `tokens:${url}`;

    if (this.config.cache.enabled) {
      const cached = this.cache.get(cacheKey) as
        | CacheEntry<TokenServerResponse>
        | undefined;
      if (cached && cached.expiresAt > Date.now()) {
        console.log(`[cache hit] ${cacheKey}`);
        return cached.data;
      }
    }

    try {
      const response = await this.client.post<TokenServerResponse>(
        this.config.endpoints.tokens,
        { url },
      );

      if (this.config.cache.enabled) {
        this.cache.set(cacheKey, {
          data: response.data,
          expiresAt: Date.now() + this.config.cache.ttl * 1000,
        });
      }

      return response.data;
    } catch (error) {
      console.error(
        `[token-server error] Failed to get tokens for ${url}`,
        error,
      );
      throw error;
    }
  }

  async getHealth(): Promise<boolean> {
    try {
      await this.client.get(this.config.endpoints.health);
      return true;
    } catch (error) {
      console.error("[token-server error] Health check failed", error);
      return false;
    }
  }
}

export const tokenServer = new TokenServerClient();
