const sv = () => {
  const s = [new Date().toISOString().slice(0, 10), 'toonlivre.net::w3', 'r7_5m2_k'].join('');
  return 'Phantom-Tide-Harvest8' + Gi.SHA256(s).toString(Gi.enc.Hex).slice(0, 8);
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
