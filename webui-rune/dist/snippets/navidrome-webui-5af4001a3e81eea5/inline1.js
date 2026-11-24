
export function ndFetch(url, method, body) {
  const opts = {method: method || 'GET', headers: {Accept: 'application/json'}};
  if (body) { opts.headers['Content-Type'] = 'application/json'; opts.body = body; }
  return fetch(url, opts).then(async r => {
    const text = await r.text();
    return JSON.stringify({ok: r.ok, status: r.status, body: text});
  });
}
