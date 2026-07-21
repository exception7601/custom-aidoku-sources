const $c = (name) => (name === 'toon_v' ? 'cookie' : '');
const sv = () => {
  const n = new Date();
  const s = `${n.getUTCFullYear()}-${String(n.getUTCMonth() + 1).padStart(2, '0')}-${String(
    n.getUTCDate()
  ).padStart(2, '0')}`;
  const i = 'toonlivre.com::v8'.split('');
  const l = 't8_4v2_b'.split('');
  const u = 'Magnesium-Strike-Astonish3'.split('');
  const f = s + i.join('') + l.join('');
  const g = Gi.SHA256(f).toString(Gi.enc.Hex).substring(0, 8);
  return u.join('') + g;
};
let bc = false;
const kc = [];
const Ec = (event) => {
  if ((event && !event.isTrusted) || bc) {
    return;
  }

  bc = true;
  let token = $c('toon_v');
  token ||
    ((token =
      Math.random().toString(36).substring(2, 15) +
      Math.random().toString(36).substring(2, 15)),
    (document.cookie = `toon_v=${token}; path=/; max-age=31536000; SameSite=Lax`));
  kc.forEach((resolve) => resolve());
};
const iv = async () => {
  if (!(bc || $c('toon_v'))) {
    return new Promise((resolve) => kc.push(resolve));
  }
};
let la = '';
let Ks = 0;
let jo = null;
const lv = async () => {
  if (typeof document < 'u') {
    const s = document.querySelector('meta[name="t-seed"]')?.getAttribute('content') || '';
    if (s)
      try {
        const parts = s.split('.');
        if (parts.length === 3) {
          const u = JSON.parse(atob(parts[1].replace(/-/g, '+').replace(/_/g, '/'))).exp * 1000;
          if (u > Date.now() + 12e4) {
            la = s;
            Ks = u;
            return s;
          }
        }
      } catch {}
  }

  return la && Ks > Date.now() + 12e4
    ? la
    : jo ||
        (jo = fetch('/api/seed', {
          credentials: 'include',
          cache: 'no-store',
        })
          .then((response) => (response.ok ? response.json() : null))
          .then((payload) => {
            if (payload?.token) {
              la = payload.token;
              return la;
            }
            return la || '';
          })
          .catch(() => la || '')
          .finally(() => {
            jo = null;
          }));
};
const He = async (input, init = {}) => {
  const i = typeof input === 'string' ? input : input.url;
  await iv();
  const buildHeaders = async (requestInit) => {
    const method = String(requestInit.method || 'GET').toUpperCase();
    const headers = new Headers(requestInit.headers || {});
    headers.append('x-toon-signature', await lv());
    if (method !== 'GET' && method !== 'HEAD' && method !== 'OPTIONS') {
      headers.set('x-csrf-token', 'csrf');
    }
    return headers;
  };

  let response = await fetch(i, {
    ...init,
    headers: await buildHeaders(init),
    credentials: 'include',
  });
  const dataKey = response.headers.get('x-toon-datakey');

  if (dataKey && response.ok) {
    const json = await response.clone().json();
    if (json && json[dataKey]) {
      const passphrase = sv();
      const decrypted = Gi.Rabbit.decrypt(json[dataKey], passphrase).toString(Gi.enc.Utf8);
      response = new Response(JSON.stringify(JSON.parse(decrypted)), {
        status: response.status,
        statusText: response.statusText,
        headers: response.headers,
      });
    }
  }

  return response;
};

export { He, Ec, lv, sv };
