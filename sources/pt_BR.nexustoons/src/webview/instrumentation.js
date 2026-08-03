((global) => {
  const DEFAULT_LANGUAGE_HEADER = "pt-BR,pt;q=0.9";

  function ensureState(globalObject) {
    if (!globalObject.__nexustoonsAidokuState) {
      globalObject.__nexustoonsAidokuState = {
        booted: false,
        config: null,
        debug: createDebugStore(globalObject),
        mangaChapterCount: 0,
        chapterPageCount: 0,
        chapterPageSeen: typeof Set !== "undefined" ? new Set() : [],
        scheduledMangaCapture: false,
        scheduledChapterCapture: false,
        chapterObserverInstalled: false,
      };
    }
    return globalObject.__nexustoonsAidokuState;
  }

  function createDebugStore(globalObject) {
    const debug = globalObject.__nexustoonsAidokuDebug =
      globalObject.__nexustoonsAidokuDebug || {
        enabled: false,
        events: [],
        fingerprintProfile: "",
      };
    return debug;
  }

  function pushDebug(debug, message) {
    if (!debug || !debug.enabled) return;
    try {
      debug.events.push(String(message));
      if (debug.events.length > 80) {
        debug.events.splice(0, debug.events.length - 80);
      }
    } catch (_) {}
  }

  function clip(value, max = 240) {
    const normalized = String(value == null ? "" : value);
    return normalized.length > max ? `${normalized.slice(0, max)}...` : normalized;
  }

  function describeData(value) {
    try {
      if (value == null) return String(value);
      if (typeof value === "string") return clip(value);
      if (typeof value !== "object") return String(value);
      if (Array.isArray(value)) return `array(${value.length})`;
      return `object(${Object.keys(value).slice(0, 8).join(",")})`;
    } catch (error) {
      return `unserializable error=${clip(error)}`;
    }
  }

  function patchProperty(target, key, descriptor) {
    try {
      Object.defineProperty(target, key, descriptor);
      return true;
    } catch (_) {
      return false;
    }
  }

  function normalizeStringList(value, fallback) {
    if (!Array.isArray(value)) return fallback.slice();
    const normalized = value.map((entry) => String(entry || "").trim()).filter(Boolean);
    return normalized.length ? normalized : fallback.slice();
  }

  function normalizeText(value) {
    return String(value || "")
      .toLowerCase()
      .normalize("NFD")
      .replace(/[\u0300-\u036f]/g, "")
      .trim();
  }

  function normalizeConfig(config) {
    return {
      width: Number(config && config.width) || 390,
      height: Number(config && config.height) || 844,
      dpr: Number(config && config.dpr) || 3,
      userAgent: String((config && config.userAgent) || global.navigator.userAgent || ""),
      platform: String((config && config.platform) || global.navigator.platform || ""),
      vendor: String((config && config.vendor) || global.navigator.vendor || ""),
      maxTouchPoints: Number(config && config.maxTouchPoints) || 5,
      languageHeader: String((config && config.languageHeader) || DEFAULT_LANGUAGE_HEADER),
      siteHostHints: normalizeStringList(
        config && config.siteHostHints,
        ["nexustoons.com", "img.nx-toons.xyz"]
      ),
      mangaPageUrlHints: normalizeStringList(
        config && config.mangaPageUrlHints,
        ["/manga/"]
      ),
      chapterPageUrlHints: normalizeStringList(
        config && config.chapterPageUrlHints,
        ["/ler/"]
      ),
      mangaPayloadUrlHints: normalizeStringList(
        config && config.mangaPayloadUrlHints,
        ["/api/manga/"]
      ),
      chapterButtonTextHints: normalizeStringList(
        config && config.chapterButtonTextHints,
        ["Capítulos", "Capitulos"]
      ),
      chapterLinkUrlHints: normalizeStringList(
        config && config.chapterLinkUrlHints,
        ["/ler/", "/r/"]
      ),
      reactRootSelectors: normalizeStringList(
        config && config.reactRootSelectors,
        ["div.custom-scrollbar"]
      ),
      chapterImageUrlHints: normalizeStringList(
        config && config.chapterImageUrlHints,
        ["manga_pages"]
      ),
      mangaCacheGlobalKey: String(
        (config && config.mangaCacheGlobalKey) || "__nexustoonsAidokuMangaCache"
      ),
      mangaStorageKey: String((config && config.mangaStorageKey) || ""),
      chapterCacheGlobalKey: String(
        (config && config.chapterCacheGlobalKey) || "__nexustoonsAidokuChapterPagesCache"
      ),
      chapterStorageKey: String((config && config.chapterStorageKey) || ""),
      debug: !!(config && config.debug),
    };
  }

  function matchesAnyHint(value, hints) {
    const normalized = String(value || "");
    if (!normalized) return false;
    return hints.some((hint) => normalized.includes(String(hint || "")));
  }

  function isSiteTarget(url, config) {
    const normalized = String(url || "");
    if (!normalized) return false;
    if (normalized.startsWith("/") || normalized.startsWith("blob:") || normalized.startsWith("data:")) {
      return true;
    }
    return matchesAnyHint(normalized, config.siteHostHints);
  }

  function isMangaPage(url, config) {
    return matchesAnyHint(url, config.mangaPageUrlHints);
  }

  function isChapterPage(url, config) {
    return matchesAnyHint(url, config.chapterPageUrlHints);
  }

  function isMangaPayloadTarget(url, config) {
    return isSiteTarget(url, config) && matchesAnyHint(url, config.mangaPayloadUrlHints);
  }

  function appendAcceptLanguage(headers, languageHeader) {
    if (headers instanceof Headers) {
      headers.set("Accept-Language", languageHeader);
      return;
    }
    if (Array.isArray(headers)) {
      let found = false;
      for (let index = 0; index < headers.length; index += 1) {
        if (String(headers[index][0] || "").toLowerCase() === "accept-language") {
          headers[index][1] = languageHeader;
          found = true;
          break;
        }
      }
      if (!found) {
        headers.push(["Accept-Language", languageHeader]);
      }
      return;
    }
    headers["Accept-Language"] = languageHeader;
  }

  function buildMatchMedia(width) {
    return function matchMedia(query) {
      const minWidth = /min-width:\s*(\d+)px/.exec(query);
      const maxWidth = /max-width:\s*(\d+)px/.exec(query);
      const min = minWidth ? Number(minWidth[1]) : null;
      const max = maxWidth ? Number(maxWidth[1]) : null;
      const matches = (min === null || width >= min) && (max === null || width <= max);
      return {
        matches,
        media: query,
        onchange: null,
        addListener() {},
        removeListener() {},
        addEventListener() {},
        removeEventListener() {},
        dispatchEvent() {
          return false;
        },
      };
    };
  }

  function applyLayoutPatch(globalObject, config) {
    const normalized = normalizeConfig(config);
    const debug = createDebugStore(globalObject);
    const { width, height, dpr, userAgent, platform, vendor, maxTouchPoints } = normalized;

    debug.fingerprintProfile = `iphone:${width}x${height}@${dpr}`;

    if (globalObject.screen && globalObject.screen.orientation) {
      patchProperty(globalObject.screen.orientation, "type", {
        configurable: true,
        get: () => "portrait-primary",
      });
      patchProperty(globalObject.screen.orientation, "angle", {
        configurable: true,
        get: () => 0,
      });
    }

    patchProperty(globalObject, "innerWidth", { configurable: true, get: () => width });
    patchProperty(globalObject, "innerHeight", { configurable: true, get: () => height });
    patchProperty(globalObject, "outerWidth", { configurable: true, get: () => width });
    patchProperty(globalObject, "outerHeight", { configurable: true, get: () => height });
    patchProperty(globalObject, "devicePixelRatio", { configurable: true, get: () => dpr });
    patchProperty(globalObject, "orientation", { configurable: true, get: () => 0 });
    patchProperty(globalObject.document, "visibilityState", {
      configurable: true,
      get: () => "visible",
    });
    patchProperty(globalObject.document, "hidden", {
      configurable: true,
      get: () => false,
    });
    patchProperty(globalObject.document, "hasFocus", {
      configurable: true,
      value: () => true,
    });
    patchProperty(globalObject.navigator, "userAgent", {
      configurable: true,
      get: () => userAgent,
    });
    patchProperty(globalObject.navigator, "appVersion", {
      configurable: true,
      get: () => userAgent,
    });
    patchProperty(globalObject.navigator, "platform", {
      configurable: true,
      get: () => platform,
    });
    patchProperty(globalObject.navigator, "vendor", {
      configurable: true,
      get: () => vendor,
    });
    patchProperty(globalObject.navigator, "language", {
      configurable: true,
      get: () => "pt-BR",
    });
    patchProperty(globalObject.navigator, "languages", {
      configurable: true,
      get: () => ["pt-BR", "pt"],
    });
    patchProperty(globalObject.navigator, "maxTouchPoints", {
      configurable: true,
      get: () => maxTouchPoints,
    });
    patchProperty(globalObject.navigator, "webdriver", {
      configurable: true,
      get: () => false,
    });
    patchProperty(globalObject.Event.prototype, "isTrusted", {
      configurable: true,
      get: () => true,
    });
    if (globalObject.screen) {
      patchProperty(globalObject.screen, "width", { configurable: true, get: () => width });
      patchProperty(globalObject.screen, "height", { configurable: true, get: () => height });
      patchProperty(globalObject.screen, "availWidth", {
        configurable: true,
        get: () => width,
      });
      patchProperty(globalObject.screen, "availHeight", {
        configurable: true,
        get: () => height,
      });
      patchProperty(globalObject.screen, "colorDepth", {
        configurable: true,
        get: () => 32,
      });
      patchProperty(globalObject.screen, "pixelDepth", {
        configurable: true,
        get: () => 32,
      });
    }
    patchProperty(globalObject, "matchMedia", {
      configurable: true,
      writable: true,
      value: buildMatchMedia(width),
    });

    const OriginalIntersectionObserver = globalObject.IntersectionObserver;
    if (
      OriginalIntersectionObserver &&
      !OriginalIntersectionObserver.__nexustoonsAidokuWrapped
    ) {
      globalObject.IntersectionObserver = class ForcedIntersectionObserver extends OriginalIntersectionObserver {
        constructor(callback, options) {
          super((entries, observer) => {
            const forced = entries.map((entry) => ({
              ...entry,
              isIntersecting: true,
              intersectionRatio: 1,
            }));
            callback(forced, observer);
          }, options);
        }
      };
      globalObject.IntersectionObserver.__nexustoonsAidokuWrapped = true;
    }

    try {
      globalObject.document.dispatchEvent(new Event("visibilitychange"));
      globalObject.dispatchEvent(new Event("resize"));
      globalObject.dispatchEvent(new Event("focus"));
      globalObject.dispatchEvent(new Event("orientationchange"));
    } catch (_) {}

    return {
      innerWidth: globalObject.innerWidth,
      innerHeight: globalObject.innerHeight,
      devicePixelRatio: globalObject.devicePixelRatio,
      visibilityState: globalObject.document.visibilityState,
      hidden: globalObject.document.hidden,
      userAgent: globalObject.navigator.userAgent,
      fingerprintProfile: debug.fingerprintProfile,
    };
  }

  function triggerSyntheticEvents(globalObject) {
    const trigger = () => {
      try {
        globalObject.dispatchEvent(new Event("scroll"));
        globalObject.dispatchEvent(new MouseEvent("mousemove"));
        globalObject.dispatchEvent(new Event("focus"));
        globalObject.dispatchEvent(new Event("orientationchange"));
      } catch (_) {}
    };

    globalObject.setTimeout(trigger, 100);
    globalObject.setTimeout(trigger, 500);
    globalObject.setTimeout(trigger, 1500);
    globalObject.setTimeout(trigger, 3000);
  }

  function createVisitedTracker() {
    return typeof WeakSet !== "undefined" ? new WeakSet() : [];
  }

  function hasVisited(tracker, value) {
    if (!tracker || !value || typeof value !== "object") return false;
    if (typeof tracker.has === "function") return tracker.has(value);
    return tracker.includes(value);
  }

  function markVisited(tracker, value) {
    if (!tracker || !value || typeof value !== "object") return;
    if (typeof tracker.add === "function") {
      tracker.add(value);
      return;
    }
    tracker.push(value);
  }

  function normalizeChapterId(value) {
    const numeric = Number(value);
    return Number.isFinite(numeric) && numeric > 0 ? Math.trunc(numeric) : 0;
  }

  function normalizeChapterNumber(value) {
    const normalized = String(value == null ? "" : value).trim();
    return normalized;
  }

  function isLikelyChapterEntry(value) {
    if (!value || typeof value !== "object") return false;
    return normalizeChapterId(value.id || value.chapterId) > 0 &&
      normalizeChapterNumber(value.number || value.chapterNumber || value.chapter_number)
        .length > 0;
  }

  function normalizeChapterEntries(value) {
    if (!Array.isArray(value)) return [];
    return value
      .filter(isLikelyChapterEntry)
      .map((chapter) => ({
        id: normalizeChapterId(chapter.id || chapter.chapterId),
        number: normalizeChapterNumber(
          chapter.number || chapter.chapterNumber || chapter.chapter_number
        ),
        title:
          typeof chapter.title === "string"
            ? chapter.title.trim()
            : typeof chapter.name === "string"
              ? chapter.name.trim()
              : "",
        dateUploaded:
          typeof chapter.createdAt === "string"
            ? chapter.createdAt
            : typeof chapter.dateUploaded === "string"
              ? chapter.dateUploaded
              : typeof chapter.date_uploaded === "string"
                ? chapter.date_uploaded
                : null,
      }))
      .filter((chapter) => chapter.id > 0 && chapter.number.length > 0);
  }

  function chapterListFromValue(value) {
    if (!value || typeof value !== "object") return [];
    return normalizeChapterEntries(
      value.chapters || value.recentChapters || value.recent_chapters || []
    );
  }

  function isLikelyMangaPayload(value) {
    if (!value || typeof value !== "object") return false;
    if (typeof value.title !== "string") return false;
    return chapterListFromValue(value).length > 0;
  }

  function extractMangaPayloadCandidate(value, maxDepth = 6, tracker) {
    if (!value || maxDepth < 0) return null;
    if (isLikelyMangaPayload(value)) return value;
    if (typeof value !== "object") return null;

    const visited = tracker || createVisitedTracker();
    if (hasVisited(visited, value)) return null;
    markVisited(visited, value);

    if (Array.isArray(value)) {
      for (const entry of value) {
        const candidate = extractMangaPayloadCandidate(entry, maxDepth - 1, visited);
        if (candidate) return candidate;
      }
      return null;
    }

    const priorityKeys = ["manga", "data", "payload", "result", "response", "props"];
    for (const key of priorityKeys) {
      if (key in value) {
        const candidate = extractMangaPayloadCandidate(value[key], maxDepth - 1, visited);
        if (candidate) return candidate;
      }
    }

    for (const key of Object.keys(value)) {
      const candidate = extractMangaPayloadCandidate(value[key], maxDepth - 1, visited);
      if (candidate) return candidate;
    }

    return null;
  }

  function normalizeMangaDetailsPayload(value) {
    const candidate = extractMangaPayloadCandidate(value);
    if (!candidate) return null;

    const chapters = chapterListFromValue(candidate);
    if (!chapters.length) return null;

    return {
      title: typeof candidate.title === "string" ? candidate.title.trim() : "",
      coverUrl:
        typeof candidate.coverUrl === "string"
          ? candidate.coverUrl
          : typeof candidate.coverImage === "string"
            ? candidate.coverImage
            : typeof candidate.cover_url === "string"
              ? candidate.cover_url
              : typeof candidate.bannerImage === "string"
                ? candidate.bannerImage
                : null,
      description:
        typeof candidate.description === "string" ? candidate.description : null,
      status: typeof candidate.status === "string" ? candidate.status : null,
      chapters,
    };
  }

  function persistRawCache(globalObject, globalKey, storageKey, rawValue) {
    globalObject[globalKey] = rawValue;
    if (!storageKey) return;
    try {
      globalObject.sessionStorage.setItem(storageKey, rawValue);
    } catch (_) {}
  }

  function readRawCache(globalObject, globalKey, storageKey) {
    return (
      globalObject[globalKey] ||
      (storageKey ? globalObject.sessionStorage.getItem(storageKey) : "") ||
      ""
    );
  }

  function persistMangaCache(globalObject, config, debug, value, source) {
    try {
      const manga = normalizeMangaDetailsPayload(value);
      if (!manga || !manga.title || !manga.chapters.length) return false;
      const state = ensureState(globalObject);
      if (manga.chapters.length < state.mangaChapterCount) {
        return false;
      }
      state.mangaChapterCount = manga.chapters.length;
      const payload = JSON.stringify({ manga });
      persistRawCache(
        globalObject,
        config.mangaCacheGlobalKey,
        config.mangaStorageKey,
        payload
      );
      pushDebug(
        debug,
        `manga:cache-saved source=${clip(source)} chapters=${manga.chapters.length}`
      );
      return true;
    } catch (error) {
      pushDebug(debug, `manga:cache-save-error source=${clip(source)} error=${clip(error)}`);
      return false;
    }
  }

  function normalizeImageUrl(value) {
    return String(value || "").trim();
  }

  function filterImageUrls(urls, config) {
    const seen = new Set();
    const out = [];
    for (const entry of urls || []) {
      const normalized = normalizeImageUrl(entry);
      if (!normalized) continue;
      if (!matchesAnyHint(normalized, config.chapterImageUrlHints)) continue;
      if (seen.has(normalized)) continue;
      seen.add(normalized);
      out.push(normalized);
    }
    return out;
  }

  function collectDocumentImageUrls(globalObject, config) {
    const urls = [];
    const images = globalObject.document.querySelectorAll("img");
    for (const image of images) {
      urls.push(
        image.currentSrc ||
          image.dataset?.src ||
          image.dataset?.lazySrc ||
          image.getAttribute("data-src") ||
          image.getAttribute("src") ||
          image.src ||
          ""
      );
    }
    return filterImageUrls(urls, config);
  }

  function persistChapterPagesCache(globalObject, config, debug, pages, source) {
    try {
      const normalized = filterImageUrls(pages, config);
      if (!normalized.length) return false;
      const state = ensureState(globalObject);
      if (normalized.length < state.chapterPageCount) {
        return false;
      }
      state.chapterPageCount = normalized.length;
      const payload = JSON.stringify({ pages: normalized });
      persistRawCache(
        globalObject,
        config.chapterCacheGlobalKey,
        config.chapterStorageKey,
        payload
      );
      pushDebug(
        debug,
        `chapter:cache-saved source=${clip(source)} pages=${normalized.length}`
      );
      return true;
    } catch (error) {
      pushDebug(
        debug,
        `chapter:cache-save-error source=${clip(source)} error=${clip(error)}`
      );
      return false;
    }
  }

  function collectChapterPagesNow(globalObject, config, debug) {
    try {
      const state = ensureState(globalObject);
      const collected = collectDocumentImageUrls(globalObject, config);
      for (const value of collected) {
        if (typeof state.chapterPageSeen.add === "function") {
          state.chapterPageSeen.add(value);
        } else if (!state.chapterPageSeen.includes(value)) {
          state.chapterPageSeen.push(value);
        }
      }
      const values =
        typeof state.chapterPageSeen.values === "function"
          ? Array.from(state.chapterPageSeen.values())
          : state.chapterPageSeen.slice();
      persistChapterPagesCache(globalObject, config, debug, values, "collect-now");
      return values.length;
    } catch (error) {
      pushDebug(debug, `chapter:collect-error error=${clip(error)}`);
      return 0;
    }
  }

  function collectChapterPagesWithScroll(globalObject, config, debug, ratio, label) {
    try {
      const max = Math.max(
        0,
        (globalObject.document.documentElement.scrollHeight || 0) - globalObject.innerHeight
      );
      const target = Math.round(max * ratio);
      globalObject.scrollTo(0, target);
      globalObject.dispatchEvent(new Event("scroll"));
    } catch (_) {}
    return collectChapterPagesNow(globalObject, config, debug, label);
  }

  function textMatchesHint(text, hints) {
    const normalized = normalizeText(text);
    if (!normalized) return false;
    return hints.some((hint) => normalized.includes(normalizeText(hint)));
  }

  function openChapterList(globalObject, config, debug) {
    let clicked = 0;
    const candidates = globalObject.document.querySelectorAll("button,[role='button']");
    for (const candidate of candidates) {
      if (!textMatchesHint(candidate.textContent, config.chapterButtonTextHints)) continue;
      try {
        candidate.click();
        clicked += 1;
      } catch (_) {}
    }
    if (clicked > 0) {
      pushDebug(debug, `manga:chapter-list-open clicked=${clicked}`);
    }
    return clicked;
  }

  function findReactFiberSeedElements(globalObject, config) {
    const elements = [];
    for (const selector of config.reactRootSelectors) {
      for (const element of globalObject.document.querySelectorAll(selector)) {
        elements.push(element);
      }
    }
    for (const anchor of globalObject.document.querySelectorAll("a[href]")) {
      const href = String(anchor.getAttribute("href") || "");
      if (matchesAnyHint(href, config.chapterLinkUrlHints)) {
        const container = anchor.closest("div") || anchor.parentElement || anchor;
        elements.push(container);
      }
    }
    return elements;
  }

  function reactFiberRootsFromElement(element) {
    const roots = [];
    if (!element || typeof element !== "object") return roots;
    for (const key of Object.getOwnPropertyNames(element)) {
      if (key.startsWith("__reactFiber") || key.startsWith("__reactContainer")) {
        try {
          roots.push(element[key]);
        } catch (_) {}
      }
    }
    return roots;
  }

  function captureMangaFromReactRoots(globalObject, config, debug, source) {
    const seeds = findReactFiberSeedElements(globalObject, config);
    for (const seed of seeds) {
      for (const root of reactFiberRootsFromElement(seed)) {
        if (persistMangaCache(globalObject, config, debug, root, source)) {
          return true;
        }
      }
    }
    return false;
  }

  function captureMangaNow(globalObject, config, debug) {
    const locationHref = String(globalObject.location && globalObject.location.href || "");
    if (!isMangaPage(locationHref, config)) return false;
    const opened = openChapterList(globalObject, config, debug);
    const captured = captureMangaFromReactRoots(
      globalObject,
      config,
      debug,
      opened > 0 ? "react-root-after-open" : "react-root"
    );
    return captured;
  }

  function scheduleMangaCapture(globalObject, config, debug) {
    const state = ensureState(globalObject);
    if (state.scheduledMangaCapture) return;
    state.scheduledMangaCapture = true;

    const run = (label) => {
      try {
        openChapterList(globalObject, config, debug);
        if (!captureMangaFromReactRoots(globalObject, config, debug, label)) {
          captureMangaNow(globalObject, config, debug);
        }
      } catch (_) {}
    };

    const delays = [0, 150, 500, 1000, 2000, 3000];
    delays.forEach((delay) => {
      globalObject.setTimeout(() => run(`scheduled-manga delay=${delay}`), delay);
    });
  }

  function scheduleChapterCapture(globalObject, config, debug) {
    const state = ensureState(globalObject);
    if (state.scheduledChapterCapture) return;
    state.scheduledChapterCapture = true;

    const steps = [0, 0.05, 0.15, 0.3, 0.5, 0.75, 1];
    steps.forEach((ratio, index) => {
      globalObject.setTimeout(() => {
        collectChapterPagesWithScroll(
          globalObject,
          config,
          debug,
          ratio,
          `scheduled-chapter ratio=${ratio}`
        );
      }, index * 250);
    });
    globalObject.setTimeout(() => {
      try {
        globalObject.scrollTo(0, globalObject.document.documentElement.scrollHeight || 0);
      } catch (_) {}
      collectChapterPagesNow(globalObject, config, debug);
    }, 2500);

    if (!state.chapterObserverInstalled && typeof MutationObserver !== "undefined") {
      state.chapterObserverInstalled = true;
      try {
        const observer = new MutationObserver(() => {
          collectChapterPagesNow(globalObject, config, debug);
        });
        observer.observe(globalObject.document.documentElement || globalObject.document.body, {
          childList: true,
          subtree: true,
          attributes: true,
          attributeFilter: ["src", "data-src"],
        });
      } catch (_) {}
    }
  }

  function installJsonParseHook(globalObject, config, debug) {
    if (!globalObject.JSON || globalObject.JSON.parse.__nexustoonsAidokuWrapped) return;

    const originalParse = globalObject.JSON.parse;
    globalObject.JSON.parse = function parsePatched(text, reviver) {
      const value = originalParse.call(this, text, reviver);
      try {
        if (persistMangaCache(globalObject, config, debug, value, "JSON.parse")) {
          pushDebug(debug, `manga:captured data=${describeData(value)}`);
        }
      } catch (_) {}
      return value;
    };
    globalObject.JSON.parse.__nexustoonsAidokuWrapped = true;
  }

  function installFetchHook(globalObject, config, debug) {
    if (!globalObject.fetch || globalObject.fetch.__nexustoonsAidokuWrapped) return;

    const originalFetch = globalObject.fetch;
    async function fetchPatched(resource, init) {
      const requestInit = init || {};
      requestInit.headers = requestInit.headers || {};
      const url =
        typeof Request !== "undefined" && resource instanceof Request
          ? resource.url
          : String(resource || "");
      if (isSiteTarget(url, config)) {
        appendAcceptLanguage(requestInit.headers, config.languageHeader);
      }
      const shouldDebug = config.debug && isMangaPayloadTarget(url, config);
      if (shouldDebug) {
        pushDebug(debug, `fetch:start url=${clip(url)}`);
      }
      try {
        const response = await originalFetch.call(this, resource, requestInit);
        if (shouldDebug) {
          response.clone().text()
            .then((body) => pushDebug(debug, `fetch:done url=${clip(url)} body=${clip(body)}`))
            .catch((error) => pushDebug(debug, `fetch:read-error url=${clip(url)} error=${clip(error)}`));
        }
        if (isMangaPayloadTarget(url, config)) {
          response.clone().text()
            .then((body) => {
              try {
                persistMangaCache(globalObject, config, debug, JSON.parse(body), `fetch url=${url}`);
              } catch (_) {}
            })
            .catch(() => {});
        }
        return response;
      } catch (error) {
        if (shouldDebug) {
          pushDebug(debug, `fetch:error url=${clip(url)} error=${clip(error)}`);
        }
        throw error;
      }
    }

    fetchPatched.__nexustoonsAidokuWrapped = true;
    globalObject.fetch = fetchPatched;
  }

  function installXhrHook(globalObject, config, debug) {
    if (typeof globalObject.XMLHttpRequest === "undefined") return;
    const proto = globalObject.XMLHttpRequest.prototype;
    if (proto.open.__nexustoonsAidokuWrapped || proto.send.__nexustoonsAidokuWrapped) return;

    const originalOpen = proto.open;
    proto.open = function openPatched(method, url) {
      this.__nexustoonsAidokuUrl = String(url || "");
      return originalOpen.apply(this, arguments);
    };
    proto.open.__nexustoonsAidokuWrapped = true;

    const originalSend = proto.send;
    proto.send = function sendPatched(body) {
      try {
        if (isSiteTarget(this.__nexustoonsAidokuUrl, config)) {
          this.setRequestHeader("Accept-Language", config.languageHeader);
        }
        this.addEventListener("load", () => {
          try {
            if (!isMangaPayloadTarget(this.__nexustoonsAidokuUrl, config)) return;
            const text = this.responseText || "";
            if (!text) return;
            try {
              persistMangaCache(
                globalObject,
                config,
                debug,
                JSON.parse(text),
                `xhr url=${this.__nexustoonsAidokuUrl}`
              );
            } catch (_) {}
          } catch (_) {}
        }, { once: true });
      } catch (_) {}
      return originalSend.call(this, body);
    };
    proto.send.__nexustoonsAidokuWrapped = true;
  }

  function readMangaCache(globalObject, config) {
    return readRawCache(globalObject, config.mangaCacheGlobalKey, config.mangaStorageKey);
  }

  function readChapterPagesCache(globalObject, config) {
    return readRawCache(globalObject, config.chapterCacheGlobalKey, config.chapterStorageKey);
  }

  function exposeApis(globalObject, config, debug) {
    globalObject.__nexustoonsAidokuOpenChapterList = () =>
      openChapterList(globalObject, config, debug);
    globalObject.__nexustoonsAidokuCaptureMangaNow = () =>
      captureMangaNow(globalObject, config, debug);
    globalObject.__nexustoonsAidokuCollectChapterPagesNow = () =>
      collectChapterPagesNow(globalObject, config, debug);
    globalObject.__nexustoonsAidokuTestAPI = {
      applyLayoutPatch: () => applyLayoutPatch(globalObject, config),
      openChapterList: () => openChapterList(globalObject, config, debug),
      captureMangaNow: () => captureMangaNow(globalObject, config, debug),
      collectChapterPagesNow: () => collectChapterPagesNow(globalObject, config, debug),
      readMangaCache: () => readMangaCache(globalObject, config),
      readChapterPagesCache: () => readChapterPagesCache(globalObject, config),
    };
  }

  function boot(config) {
    const state = ensureState(global);
    const normalized = normalizeConfig(config || state.config || {});
    const debug = state.debug;

    debug.enabled = normalized.debug;
    if (!debug.enabled) {
      debug.events = [];
    }

    state.config = normalized;
    global.__nexustoonsAidokuConfig = normalized;
    global.__nexustoonsAidokuApplyLayoutPatch = (nextConfig) =>
      applyLayoutPatch(global, nextConfig || state.config || normalized);

    applyLayoutPatch(global, normalized);
    triggerSyntheticEvents(global);
    exposeApis(global, normalized, debug);

    if (normalized.debug) {
      pushDebug(debug, `fingerprint profile=${debug.fingerprintProfile} ua=${normalized.userAgent}`);
    }

    if (!state.booted) {
      installJsonParseHook(global, normalized, debug);
      installFetchHook(global, normalized, debug);
      installXhrHook(global, normalized, debug);
      state.booted = true;
    }

    const href = String(global.location && global.location.href || "");
    if (isMangaPage(href, normalized)) {
      scheduleMangaCapture(global, normalized, debug);
    }
    if (isChapterPage(href, normalized)) {
      scheduleChapterCapture(global, normalized, debug);
    }

    return {
      booted: state.booted,
      fingerprintProfile: debug.fingerprintProfile,
    };
  }

  global.__nexustoonsAidokuBoot = boot;
  global.__nexustoonsAidokuApplyLayoutPatch = (config) => applyLayoutPatch(global, config || {});
  global.__nexustoonsAidokuInternals = {
    normalizeConfig,
    matchesAnyHint,
    filterImageUrls,
    extractMangaPayloadCandidate,
    normalizeMangaDetailsPayload,
    isMangaPayloadTarget: (url, config) =>
      isMangaPayloadTarget(url, normalizeConfig(config || {})),
  };
})(globalThis);
