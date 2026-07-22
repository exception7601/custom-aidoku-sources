const sv = () => {
  const n = new Date();
  const i =
    `${n.getUTCFullYear()}-${String(n.getUTCMonth() + 1).padStart(2, '0')}-${String(
      n.getUTCDate()
    ).padStart(2, '0')}` + 'toonlivre.net::v9p6_2x8_j';
  return 'Celestial-Raven-Invoke9' + Gi.SHA256(i).toString(Gi.enc.Hex).slice(0, 8);
};

const He = async (response, json) => {
  const dataKey = response.headers.get('x-toon-datakey');

  if (dataKey && response.ok && json && json[dataKey]) {
    const passphrase = sv();
    return Gi.Rabbit.decrypt(json[dataKey], passphrase).toString(Gi.enc.Utf8);
  }

  return null;
};

export { He, sv };
