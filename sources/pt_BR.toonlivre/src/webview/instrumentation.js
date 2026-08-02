(function initToonLivreAidoku(global) {
  const DEFAULT_LANGUAGE_HEADER = "pt-BR,pt;q=0.9";
  const DEBUG_MESSAGE_KEY = "__toonlivreAidokuWorkerDebug";

  function clip(value, maxChars = 180) {
    return String(value || "").replace(/\s+/g, " ").trim().slice(0, maxChars);
  }

  function pushDebug(debug, message) {
    try {
      debug.events.push(String(message));
      if (debug.events.length > 60) {
        debug.events = debug.events.slice(-60);
      }
    } catch (_) {}
  }

  function pushWorkerUrl(debug, value) {
    try {
      debug.workerUrls.push(String(value || ""));
      if (debug.workerUrls.length > 16) {
        debug.workerUrls = debug.workerUrls.slice(-16);
      }
    } catch (_) {}
  }

  function createDebugStore(globalObject) {
    const debug = globalObject.__toonlivreAidokuDebug =
      globalObject.__toonlivreAidokuDebug || {
        events: [],
        workerUrls: [],
        fingerprintProfile: "",
      };
    return debug;
  }

  function ensureState(globalObject) {
    const state = globalObject.__toonlivreAidokuState =
      globalObject.__toonlivreAidokuState || {
        booted: false,
        debug: createDebugStore(globalObject),
        config: null,
      };
    return state;
  }

  function patchProperty(target, key, descriptor) {
    try {
      Object.defineProperty(target, key, descriptor);
      return true;
    } catch (_) {
      return false;
    }
  }

  function normalizeUrl(value) {
    if (!value) return "";
    if (typeof value === "string") return value;
    if (typeof Request !== "undefined" && value instanceof Request) return value.url || "";
    if (typeof value.url === "string") return value.url;
    try {
      return String(value);
    } catch (_) {
      return "";
    }
  }

  function toAbsoluteUrl(value, baseUrl) {
    const normalized = normalizeUrl(value);
    if (!normalized) return normalized;
    try {
      return new URL(normalized, baseUrl || global.location.href).toString();
    } catch (_) {
      return normalized;
    }
  }

  function describeData(value) {
    try {
      if (typeof value === "string") return clip(value);
      if (value === null || typeof value === "undefined") return String(value);
      if (typeof value === "number" || typeof value === "boolean") return String(value);
      if (Array.isArray(value)) return `array(${value.length}) ${clip(JSON.stringify(value))}`;
      if (typeof value === "object") {
        const keys = Object.keys(value).slice(0, 8).join(",");
        return `object keys=${keys} payload=${clip(JSON.stringify(value))}`;
      }
      return clip(String(value));
    } catch (error) {
      return `unserializable error=${clip(error)}`;
    }
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
      toonVCookie: String((config && config.toonVCookie) || ""),
      chapterCacheGlobalKey: String(
        (config && config.chapterCacheGlobalKey) || "__toonlivreAidokuChapterCache"
      ),
      chapterStorageKey: String((config && config.chapterStorageKey) || ""),
      targetMangaId: String((config && config.targetMangaId) || ""),
      targetChapterId: String((config && config.targetChapterId) || ""),
      targetChapterNumber: String((config && config.targetChapterNumber) || ""),
      debug: !!(config && config.debug),
    };
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

    const patched = {
      innerWidth: patchProperty(globalObject, "innerWidth", { configurable: true, get: () => width }),
      innerHeight: patchProperty(globalObject, "innerHeight", {
        configurable: true,
        get: () => height,
      }),
      outerWidth: patchProperty(globalObject, "outerWidth", { configurable: true, get: () => width }),
      outerHeight: patchProperty(globalObject, "outerHeight", {
        configurable: true,
        get: () => height,
      }),
      devicePixelRatio: patchProperty(globalObject, "devicePixelRatio", {
        configurable: true,
        get: () => dpr,
      }),
      orientation: patchProperty(globalObject, "orientation", {
        configurable: true,
        get: () => 0,
      }),
      visibilityState: patchProperty(globalObject.document, "visibilityState", {
        configurable: true,
        get: () => "visible",
      }),
      hidden: patchProperty(globalObject.document, "hidden", {
        configurable: true,
        get: () => false,
      }),
      hasFocus: patchProperty(globalObject.document, "hasFocus", {
        configurable: true,
        value: () => true,
      }),
      userAgent: patchProperty(globalObject.navigator, "userAgent", {
        configurable: true,
        get: () => userAgent,
      }),
      appVersion: patchProperty(globalObject.navigator, "appVersion", {
        configurable: true,
        get: () => userAgent,
      }),
      platform: patchProperty(globalObject.navigator, "platform", {
        configurable: true,
        get: () => platform,
      }),
      vendor: patchProperty(globalObject.navigator, "vendor", {
        configurable: true,
        get: () => vendor,
      }),
      language: patchProperty(globalObject.navigator, "language", {
        configurable: true,
        get: () => "pt-BR",
      }),
      languages: patchProperty(globalObject.navigator, "languages", {
        configurable: true,
        get: () => ["pt-BR", "pt"],
      }),
      maxTouchPoints: patchProperty(globalObject.navigator, "maxTouchPoints", {
        configurable: true,
        get: () => maxTouchPoints,
      }),
      webdriver: patchProperty(globalObject.navigator, "webdriver", {
        configurable: true,
        get: () => false,
      }),
      isTrusted: patchProperty(globalObject.Event.prototype, "isTrusted", {
        configurable: true,
        get: () => true,
      }),
      screenWidth: globalObject.screen
        ? patchProperty(globalObject.screen, "width", { configurable: true, get: () => width })
        : false,
      screenHeight: globalObject.screen
        ? patchProperty(globalObject.screen, "height", { configurable: true, get: () => height })
        : false,
      screenAvailWidth: globalObject.screen
        ? patchProperty(globalObject.screen, "availWidth", {
            configurable: true,
            get: () => width,
          })
        : false,
      screenAvailHeight: globalObject.screen
        ? patchProperty(globalObject.screen, "availHeight", {
            configurable: true,
            get: () => height,
          })
        : false,
      screenColorDepth: globalObject.screen
        ? patchProperty(globalObject.screen, "colorDepth", {
            configurable: true,
            get: () => 32,
          })
        : false,
      screenPixelDepth: globalObject.screen
        ? patchProperty(globalObject.screen, "pixelDepth", {
            configurable: true,
            get: () => 32,
          })
        : false,
      matchMedia: patchProperty(globalObject, "matchMedia", {
        configurable: true,
        writable: true,
        value: buildMatchMedia(width),
      }),
    };

    globalObject.document.dispatchEvent(new Event("visibilitychange"));
    globalObject.dispatchEvent(new Event("resize"));
    globalObject.dispatchEvent(new Event("focus"));
    globalObject.dispatchEvent(new Event("orientationchange"));

    return {
      patched,
      innerWidth: globalObject.innerWidth,
      innerHeight: globalObject.innerHeight,
      outerWidth: globalObject.outerWidth,
      outerHeight: globalObject.outerHeight,
      screenWidth: globalObject.screen ? globalObject.screen.width : 0,
      screenHeight: globalObject.screen ? globalObject.screen.height : 0,
      devicePixelRatio: globalObject.devicePixelRatio,
      userAgent: globalObject.navigator.userAgent,
      platform: globalObject.navigator.platform,
      vendor: globalObject.navigator.vendor,
      visibilityState: globalObject.document.visibilityState,
      hidden: globalObject.document.hidden,
      matchMedia: globalObject.matchMedia("(max-width: 767px)").matches,
    };
  }

  function setToonVCookie(globalObject, config, debug) {
    if (!config.toonVCookie) return;
    try {
      globalObject.document.cookie =
        `toon_v=${config.toonVCookie}; Path=/; Max-Age=31536000; SameSite=Lax`;
      pushDebug(debug, `cookie:set toon_v=${clip(config.toonVCookie)}`);
    } catch (error) {
      pushDebug(debug, `cookie:set-error error=${clip(error)}`);
    }
  }

  function isRuntimeTarget(url) {
    return (
      String(url || "").includes("/api/reader/bootstrap") ||
      String(url || "").includes("/api/reader/runtime") ||
      String(url || "").includes("/api/reader/proof/verify")
    );
  }

  function isChapterTarget(url) {
    const value = String(url || "");
    return value.includes("/api/mangas/") && value.includes("/chapters/");
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

  function normalizeChapterPayload(value, config) {
    if (!value || typeof value !== "object" || !Array.isArray(value.pages)) return null;
    const pages = value.pages.map((page) => String(page || "").trim()).filter(Boolean);
    if (!pages.length) return null;
    return {
      id: String(value.id || config.targetChapterId),
      pages,
      title: typeof value.title === "string" ? value.title : "",
      number: String(
        value.number || value.chapterNumber || config.targetChapterNumber || config.targetChapterId
      ),
      mangaId: String(value.mangaId || value.manga_id || config.targetMangaId),
      timestamp: Number(value.timestamp || 0) || 0,
      releaseDate: typeof value.releaseDate === "string" ? value.releaseDate : "",
    };
  }

  function persistChapterCache(globalObject, config, debug, value, source) {
    try {
      const chapter = normalizeChapterPayload(value, config);
      if (!chapter) return false;
      const payload = JSON.stringify({ chapter });
      globalObject[config.chapterCacheGlobalKey] = payload;
      globalObject.sessionStorage.setItem(config.chapterStorageKey, payload);
      pushDebug(
        debug,
        `chapter:cache-saved source=${clip(source)} key=${clip(config.chapterStorageKey)} pages=${chapter.pages.length}`
      );
      return true;
    } catch (error) {
      pushDebug(debug, `chapter:cache-save-error source=${clip(source)} error=${clip(error)}`);
      return false;
    }
  }

  function captureChapterMessage(globalObject, config, debug, value, source) {
    try {
      if (persistChapterCache(globalObject, config, debug, value, `${source}:direct`)) return true;
      if (Array.isArray(value) && value.length >= 3) {
        return persistChapterCache(globalObject, config, debug, value[2], `${source}:array[2]`);
      }
    } catch (_) {}
    return false;
  }

  function readChapterCache(globalObject, config) {
    return (
      globalObject[config.chapterCacheGlobalKey] ||
      globalObject.sessionStorage.getItem(config.chapterStorageKey) ||
      ""
    );
  }

  function buildWorkerShimSource(workerSourceUrl, languageHeader) {
    return `
(() => {
  const DEBUG_MESSAGE_KEY = ${JSON.stringify(DEBUG_MESSAGE_KEY)};
  const ORIGINAL_URL = ${JSON.stringify(workerSourceUrl)};
  const LANGUAGE_HEADER = ${JSON.stringify(languageHeader)};
  const clip = ${clip.toString()};
  const normalizeUrl = ${normalizeUrl.toString()};
  const describeData = ${describeData.toString()};
  const isRuntimeTarget = ${isRuntimeTarget.toString()};
  const appendAcceptLanguage = ${appendAcceptLanguage.toString()};
  const resolveUrl = (value) => {
    const normalized = normalizeUrl(value);
    if (!normalized) return normalized;
    try {
      return new URL(normalized, ORIGINAL_URL).toString();
    } catch (_) {
      return normalized;
    }
  };
  const originalPostMessage = self.postMessage.bind(self);
  const sendDebug = (message) => {
    try {
      originalPostMessage({ [DEBUG_MESSAGE_KEY]: true, message });
    } catch (_) {}
  };

  sendDebug(\`worker:proxy-start url=\${clip(ORIGINAL_URL)}\`);

  self.addEventListener("message", (event) => {
    const data = event && event.data;
    if (data && typeof data === "object" && data[DEBUG_MESSAGE_KEY]) return;
    sendDebug(\`worker:message data=\${describeData(data)}\`);
  });
  self.addEventListener("messageerror", () => {
    sendDebug("worker:messageerror");
  });
  self.addEventListener("error", (event) => {
    sendDebug(
      \`worker:error message=\${clip(event && event.message)} filename=\${clip(
        event && event.filename
      )} lineno=\${(event && event.lineno) || 0} colno=\${(event && event.colno) || 0}\`
    );
  });

  if (typeof importScripts === "function") {
    const originalImportScripts = self.importScripts.bind(self);
    self.importScripts = function importScriptsPatched() {
      const resolvedUrls = Array.prototype.slice.call(arguments).map((value) => resolveUrl(value) || value);
      sendDebug(\`worker:importScripts urls=\${clip(resolvedUrls.join(" | "))}\`);
      return originalImportScripts.apply(this, resolvedUrls);
    };
  }

  if (typeof Worker !== "undefined") {
    const NestedOriginalWorker = Worker;
    self.Worker = function nestedWorker(url, options) {
      const nestedUrl = normalizeUrl(url);
      const resolvedUrl = resolveUrl(url);
      sendDebug(
        \`worker:nested-worker:create url=\${clip(nestedUrl)} resolved=\${clip(resolvedUrl)}\`
      );
      return new NestedOriginalWorker(resolvedUrl || url, options);
    };
    self.Worker.prototype = NestedOriginalWorker.prototype;
  }

  if (self.fetch) {
    const originalFetch = self.fetch.bind(self);
    self.fetch = async function fetchPatched(resource, init) {
      const requestInit = init || {};
      requestInit.headers = requestInit.headers || {};
      appendAcceptLanguage(requestInit.headers, LANGUAGE_HEADER);
      const url = resolveUrl(resource);
      const method =
        requestInit.method ||
        (typeof Request !== "undefined" && resource instanceof Request ? resource.method : "GET");
      const input = typeof Request !== "undefined" && resource instanceof Request ? resource : (url || resource);
      if (isRuntimeTarget(url)) {
        sendDebug(\`worker:fetch:start method=\${clip(method)} url=\${clip(url)}\`);
      }
      try {
        const response = await originalFetch(input, requestInit);
        if (isRuntimeTarget(url)) {
          response.clone().text()
            .then((body) => sendDebug(\`worker:fetch:done status=\${response.status} url=\${clip(url)} body=\${clip(body)}\`))
            .catch((error) => sendDebug(\`worker:fetch:done status=\${response.status} url=\${clip(url)} body_error=\${clip(error)}\`));
        }
        return response;
      } catch (error) {
        if (isRuntimeTarget(url)) {
          sendDebug(\`worker:fetch:error method=\${clip(method)} url=\${clip(url)} error=\${clip(error)}\`);
        }
        throw error;
      }
    };
  }

  if (typeof XMLHttpRequest !== "undefined") {
    const originalOpen = XMLHttpRequest.prototype.open;
    XMLHttpRequest.prototype.open = function openPatched(method, url) {
      this.__toonlivreMethod = method;
      this.__toonlivreUrl = resolveUrl(url);
      const args = Array.prototype.slice.call(arguments);
      args[1] = this.__toonlivreUrl || url;
      if (isRuntimeTarget(this.__toonlivreUrl)) {
        sendDebug(\`worker:xhr:open method=\${clip(method)} url=\${clip(this.__toonlivreUrl)}\`);
      }
      return originalOpen.apply(this, args);
    };

    const originalSend = XMLHttpRequest.prototype.send;
    XMLHttpRequest.prototype.send = function sendPatched(body) {
      try {
        if (
          this.__toonlivreUrl &&
          (this.__toonlivreUrl.startsWith("/") || this.__toonlivreUrl.includes("toonlivre.net"))
        ) {
          this.setRequestHeader("Accept-Language", LANGUAGE_HEADER);
        }
        if (isRuntimeTarget(this.__toonlivreUrl)) {
          const method = this.__toonlivreMethod || "GET";
          sendDebug(\`worker:xhr:send method=\${clip(method)} url=\${clip(this.__toonlivreUrl)} body=\${clip(body)}\`);
          this.addEventListener("load", () => {
            sendDebug(\`worker:xhr:load status=\${this.status} url=\${clip(this.__toonlivreUrl)} body=\${clip(this.responseText)}\`);
          }, { once: true });
          this.addEventListener("error", () => {
            sendDebug(\`worker:xhr:error url=\${clip(this.__toonlivreUrl)}\`);
          }, { once: true });
        }
      } catch (_) {}
      return originalSend.apply(this, arguments);
    };
  }

  self.postMessage = function postMessagePatched(message, transfer) {
    if (!(message && typeof message === "object" && message[DEBUG_MESSAGE_KEY])) {
      sendDebug(\`worker:postMessage data=\${describeData(message)}\`);
    }
    return originalPostMessage.apply(this, arguments);
  };

  sendDebug(\`worker:proxy-source-ready url=\${clip(ORIGINAL_URL)}\`);
})();
`;
  }

  function buildWorkerProxyUrl(globalObject, config, debug, absoluteUrl) {
    const workerSourceUrl = toAbsoluteUrl(absoluteUrl);
    if (!workerSourceUrl) {
      throw new Error("Missing worker runtime URL");
    }

    pushDebug(debug, `worker:proxy-source-fetch-start url=${clip(workerSourceUrl)}`);
    const xhr = new XMLHttpRequest();
    xhr.open("GET", workerSourceUrl, false);

    try {
      xhr.withCredentials = true;
    } catch (_) {}

    try {
      xhr.setRequestHeader("Accept-Language", config.languageHeader);
    } catch (_) {}

    let runtimeSource = "";
    try {
      xhr.send(null);
      runtimeSource = typeof xhr.responseText === "string" ? xhr.responseText : "";
      pushDebug(
        debug,
        `worker:proxy-source-fetched url=${clip(workerSourceUrl)} status=${xhr.status || 0} bytes=${runtimeSource.length}`
      );
    } catch (error) {
      pushDebug(debug, `worker:proxy-source-fetch-error url=${clip(workerSourceUrl)} error=${clip(error)}`);
      throw error;
    }

    if ((xhr.status || 0) < 200 || (xhr.status || 0) >= 300 || !runtimeSource.trim()) {
      throw new Error(`Runtime fetch failed status=${xhr.status || 0} bytes=${runtimeSource.length}`);
    }

    const source = [
      buildWorkerShimSource(workerSourceUrl, config.languageHeader),
      runtimeSource,
      `\n//# sourceURL=${workerSourceUrl}`,
    ].join("\n");

    return URL.createObjectURL(new Blob([source], { type: "application/javascript" }));
  }

  function installWorkerHooks(globalObject, config, debug, helpers) {
    if (typeof globalObject.Worker === "undefined") return;
    if (globalObject.Worker.__toonlivreAidokuWrapped) return;

    const OriginalWorker = globalObject.Worker;

    function WrappedWorker(url, options) {
      const normalizedUrl = normalizeUrl(url);
      const absoluteUrl = toAbsoluteUrl(url);
      const workerLabel = normalizedUrl || absoluteUrl || clip(url);
      const isModuleWorker = !!(options && options.type === "module");

      pushWorkerUrl(debug, workerLabel);
      if (isRuntimeTarget(workerLabel)) {
        pushDebug(debug, `worker:create url=${clip(workerLabel)}`);
      }

      let workerUrl = url;
      let proxyUrl = null;
      if (isRuntimeTarget(workerLabel) && !isModuleWorker) {
        try {
          proxyUrl = buildWorkerProxyUrl(globalObject, config, debug, absoluteUrl || workerLabel);
          workerUrl = proxyUrl;
          pushDebug(debug, `worker:proxy url=${clip(workerLabel)} proxy=${clip(proxyUrl)}`);
        } catch (error) {
          pushDebug(debug, `worker:proxy-error url=${clip(workerLabel)} error=${clip(error)}`);
        }
      } else if (isRuntimeTarget(workerLabel) && isModuleWorker) {
        pushDebug(debug, `worker:proxy-skip-module url=${clip(workerLabel)}`);
      }

      let worker;
      try {
        worker = new OriginalWorker(workerUrl, options);
      } catch (error) {
        if (proxyUrl) {
          try {
            URL.revokeObjectURL(proxyUrl);
          } catch (_) {}
        }
        pushDebug(debug, `worker:error url=${clip(workerLabel)} error=${clip(error)}`);
        throw error;
      }

      const originalPostMessage = worker.postMessage;
      worker.postMessage = function postMessagePatched(message, transfer) {
        pushDebug(debug, `worker:main-postMessage url=${clip(workerLabel)} data=${describeData(message)}`);
        return originalPostMessage.apply(this, arguments);
      };

      worker.addEventListener("message", (event) => {
        const data = event && event.data;
        if (data && typeof data === "object" && data[DEBUG_MESSAGE_KEY]) {
          pushDebug(debug, String(data.message || "worker:debug"));
          if (typeof event.stopImmediatePropagation === "function") {
            event.stopImmediatePropagation();
          }
          if (typeof event.preventDefault === "function") {
            event.preventDefault();
          }
          return;
        }
        helpers.captureChapterMessage(data, `worker-message url=${workerLabel}`);
        pushDebug(debug, `worker:message url=${clip(workerLabel)} data=${describeData(data)}`);
      });

      worker.addEventListener("error", (event) => {
        pushDebug(
          debug,
          `worker:error-event url=${clip(workerLabel)} message=${clip(event && event.message)} filename=${clip(event && event.filename)} lineno=${(event && event.lineno) || 0} colno=${(event && event.colno) || 0}`
        );
      });

      worker.addEventListener("messageerror", () => {
        pushDebug(debug, `worker:messageerror-event url=${clip(workerLabel)}`);
      });

      const originalTerminate = worker.terminate;
      worker.terminate = function terminatePatched() {
        pushDebug(debug, `worker:terminate url=${clip(workerLabel)}`);
        if (proxyUrl) {
          try {
            URL.revokeObjectURL(proxyUrl);
          } catch (_) {}
        }
        return originalTerminate.apply(this, arguments);
      };

      return worker;
    }

    WrappedWorker.prototype = OriginalWorker.prototype;
    WrappedWorker.__toonlivreAidokuWrapped = true;
    globalObject.Worker = WrappedWorker;
  }

  function installFetchHooks(globalObject, config, debug, helpers) {
    if (!globalObject.fetch || globalObject.fetch.__toonlivreAidokuWrapped) return;

    const originalFetch = globalObject.fetch;
    async function fetchPatched(resource, init) {
      const requestInit = init || {};
      requestInit.headers = requestInit.headers || {};
      appendAcceptLanguage(requestInit.headers, config.languageHeader);
      const url = normalizeUrl(resource);
      const method =
        requestInit.method ||
        (typeof Request !== "undefined" && resource instanceof Request ? resource.method : "GET");

      if (isRuntimeTarget(url)) {
        pushDebug(debug, `fetch:start method=${clip(method)} url=${clip(url)}`);
      }

      try {
        const response = await originalFetch.call(this, resource, requestInit);
        if (isRuntimeTarget(url)) {
          response.clone().text()
            .then((body) => pushDebug(debug, `fetch:done status=${response.status} url=${clip(url)} body=${clip(body)}`))
            .catch((error) => pushDebug(debug, `fetch:done status=${response.status} url=${clip(url)} body_error=${clip(error)}`));
        }
        if (isChapterTarget(url)) {
          response.clone().text()
            .then((body) => {
              try {
                helpers.persistChapterCache(JSON.parse(body), `fetch url=${url}`);
              } catch (error) {
                pushDebug(debug, `chapter:fetch-parse-error url=${clip(url)} error=${clip(error)}`);
              }
            })
            .catch((error) => pushDebug(debug, `chapter:fetch-read-error url=${clip(url)} error=${clip(error)}`));
        }
        return response;
      } catch (error) {
        if (isRuntimeTarget(url)) {
          pushDebug(debug, `fetch:error method=${clip(method)} url=${clip(url)} error=${clip(error)}`);
        }
        throw error;
      }
    }

    fetchPatched.__toonlivreAidokuWrapped = true;
    globalObject.fetch = fetchPatched;
  }

  function installXhrHooks(globalObject, config, debug, helpers) {
    if (typeof globalObject.XMLHttpRequest === "undefined") return;
    const proto = globalObject.XMLHttpRequest.prototype;
    if (proto.open.__toonlivreAidokuWrapped || proto.send.__toonlivreAidokuWrapped) return;

    const originalOpen = proto.open;
    proto.open = function openPatched(method, url) {
      this._toonlivreMethod = method;
      this._toonlivreUrl = normalizeUrl(url);
      if (isRuntimeTarget(this._toonlivreUrl)) {
        pushDebug(debug, `xhr:open method=${clip(method)} url=${clip(this._toonlivreUrl)}`);
      }
      return originalOpen.apply(this, arguments);
    };
    proto.open.__toonlivreAidokuWrapped = true;

    const originalSend = proto.send;
    proto.send = function sendPatched(body) {
      try {
        if (
          this._toonlivreUrl &&
          (this._toonlivreUrl.startsWith("/") || this._toonlivreUrl.includes("toonlivre.net"))
        ) {
          this.setRequestHeader("Accept-Language", config.languageHeader);
        }
        if (isRuntimeTarget(this._toonlivreUrl)) {
          const method = this._toonlivreMethod || "GET";
          pushDebug(debug, `xhr:send method=${clip(method)} url=${clip(this._toonlivreUrl)} body=${clip(body)}`);
          this.addEventListener("load", () => {
            pushDebug(debug, `xhr:load status=${this.status} url=${clip(this._toonlivreUrl)} body=${clip(this.responseText)}`);
          }, { once: true });
          this.addEventListener("error", () => {
            pushDebug(debug, `xhr:error url=${clip(this._toonlivreUrl)}`);
          }, { once: true });
        }
        if (isChapterTarget(this._toonlivreUrl)) {
          this.addEventListener("load", () => {
            try {
              helpers.persistChapterCache(JSON.parse(this.responseText || "{}"), `xhr url=${this._toonlivreUrl}`);
            } catch (error) {
              pushDebug(debug, `chapter:xhr-parse-error url=${clip(this._toonlivreUrl)} error=${clip(error)}`);
            }
          }, { once: true });
        }
      } catch (_) {}
      return originalSend.apply(this, arguments);
    };
    proto.send.__toonlivreAidokuWrapped = true;
  }

  function triggerSyntheticEvents(globalObject) {
    const trigger = () => {
      globalObject.dispatchEvent(new Event("scroll"));
      globalObject.dispatchEvent(new MouseEvent("mousemove"));
      globalObject.dispatchEvent(new Event("focus"));
      globalObject.dispatchEvent(new Event("orientationchange"));
    };

    globalObject.setTimeout(trigger, 500);
    globalObject.setTimeout(trigger, 1500);
    globalObject.setTimeout(trigger, 3000);
  }

  function exposeTestApi(globalObject, config, debug, helpers) {
    if (!config.debug) return;
    globalObject.__toonlivreAidokuTestAPI = {
      applyLayoutPatch: () => applyLayoutPatch(globalObject, config),
      captureChapterMessage: (value, source) => helpers.captureChapterMessage(value, source || "test"),
      normalizeChapterPayload: (value) => normalizeChapterPayload(value, config),
      persistChapterCache: (value, source) => helpers.persistChapterCache(value, source || "test"),
      readChapterCache: () => readChapterCache(globalObject, config),
    };
  }

  function boot(config) {
    const state = ensureState(global);
    const normalized = normalizeConfig(config || state.config || {});
    const debug = state.debug;

    state.config = normalized;
    global.__toonlivreAidokuConfig = normalized;
    global.__toonlivreAidokuApplyLayoutPatch = (nextConfig) =>
      applyLayoutPatch(global, nextConfig || state.config || normalized);

    const helpers = {
      captureChapterMessage: (value, source) =>
        captureChapterMessage(global, normalized, debug, value, source),
      persistChapterCache: (value, source) =>
        persistChapterCache(global, normalized, debug, value, source),
      readChapterCache: () => readChapterCache(global, normalized),
    };

    setToonVCookie(global, normalized, debug);
    applyLayoutPatch(global, normalized);
    pushDebug(debug, `fingerprint profile=${debug.fingerprintProfile} ua=${normalized.userAgent}`);

    if (!state.booted) {
      installWorkerHooks(global, normalized, debug, helpers);
      installFetchHooks(global, normalized, debug, helpers);
      installXhrHooks(global, normalized, debug, helpers);
      triggerSyntheticEvents(global);
      state.booted = true;
    }

    exposeTestApi(global, normalized, debug, helpers);
    return { booted: state.booted, fingerprintProfile: debug.fingerprintProfile };
  }

  global.__toonlivreAidokuBoot = boot;
  global.__toonlivreAidokuApplyLayoutPatch = (config) => applyLayoutPatch(global, config || {});
})(globalThis);
