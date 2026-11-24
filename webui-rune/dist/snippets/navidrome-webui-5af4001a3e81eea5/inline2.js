
export function ndPlay(url) {
  const a = document.getElementById('nd-audio');
  if (!a) return false;
  a.src = url; a.play().catch(() => {}); return true;
}
export function ndPause() {
  const a = document.getElementById('nd-audio');
  if (a) a.pause();
}
