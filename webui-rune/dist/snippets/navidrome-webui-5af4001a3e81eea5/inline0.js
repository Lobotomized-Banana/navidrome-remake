
export function ndConfig() {
  let raw = window.__APP_CONFIG__ || {};
  if (typeof raw === 'string') { try { raw = JSON.parse(raw); } catch { raw = {}; } }
  const c = Object.assign({}, raw);
  c.origin = window.location.origin;
  return JSON.stringify(c);
}
