const nl = (name) => (name === 'toon_v' ? 'cookie' : '');
const nv = () => {
  const n = new Date();
  const s = `${n.getUTCFullYear()}-${String(n.getUTCMonth() + 1).padStart(2, '0')}-${String(
    n.getUTCDate()
  ).padStart(2, '0')}`;
  const i = 'toonlivre.tv::v8'.split('');
  const l = 't17_4v19_b2'.split('');
  const u = 'Dealer-Critter-Catnip4'.split('');
  const f = s + i.join('') + l.join('');
  const g = Wi.MD5(f).toString(Wi.enc.Hex).substring(0, 8);
  return u.join('') + g;
};
let vc = false;
const wc = [];
const bc = () => {
  if (vc) {
    return;
  }

  vc = true;
  let n = nl('toon_v');
  n ||
    ((n =
      Math.random().toString(36).substring(2, 15) + Math.random().toString(36).substring(2, 15)),
    (document.cookie = `toon_v=${n}; path=/; max-age=31536000; SameSite=Lax`));
  wc.forEach((resolve) => resolve());
};
const av = async () => {
  if (!(vc || nl('toon_v'))) {
    return new Promise((resolve) => wc.push(resolve));
  }
};
const ov = () => nl('toon_v') || '';
const He = async (input, init = {}) => {
  const i = typeof input === 'string' ? input : input.url;
  await av();
  const buildHeaders = async (requestInit) => {
    const method = String(requestInit.method || 'GET').toUpperCase();
    const headers = new Headers(requestInit.headers || {});
    const decode = (value) => value.reduce((acc, item) => acc + String.fromCharCode(item), '');
    const appendMethod = decode([97, 112, 112, 101, 110, 100]);
    const signatureHeader = decode([
      120, 45, 116, 111, 111, 110, 45, 115, 105, 103, 110, 97, 116, 117, 114, 101,
    ]);
    const chapterSignature = decode([116, 56, 118, 95, 97, 117, 116, 104, 88, 57]);
    const defaultSignature = decode([116, 56, 118, 95, 100, 101, 99, 111, 121, 57]);
    const isChapter = i.includes('/chapters');

    if (
      (headers[appendMethod](signatureHeader, isChapter ? chapterSignature : defaultSignature),
      headers[appendMethod]('x-toon-verify', ov()),
      method !== 'GET' && method !== 'HEAD' && method !== 'OPTIONS')
    ) {
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
      const passphrase = nv();
      const decrypted = Wi.Rabbit.decrypt(json[dataKey], passphrase).toString(Wi.enc.Utf8);
      response = new Response(JSON.stringify(JSON.parse(decrypted)), {
        status: response.status,
        statusText: response.statusText,
        headers: response.headers,
      });
    }
  }

  return response;
};

export { He, bc, nv };
