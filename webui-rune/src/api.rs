//! Thin Subsonic API client for the Rune frontend.
//!
//! On wasm32 it uses the browser's own `fetch()` through a tiny wasm-bindgen
//! bridge (Rune has no general HTTP client). On native builds it returns demo
//! data, which lets the same view be rendered headlessly for visual checks.

use serde::Deserialize;

#[derive(Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Auth {
    pub username: String,
    pub token: String,
    pub subsonic_salt: String,
    pub subsonic_token: String,
}

#[derive(Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Artist {
    pub id: String,
    pub name: String,
    pub cover_art: String,
    pub album_count: i32,
}

#[derive(Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Album {
    pub id: String,
    pub name: String,
    pub artist: String,
    pub artist_id: String,
    pub cover_art: String,
    pub song_count: i32,
    pub duration: i32,
    pub year: i32,
    pub genre: String,
}

#[derive(Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Song {
    pub id: String,
    pub title: String,
    pub album: String,
    pub artist: String,
    pub album_id: String,
    pub cover_art: String,
    pub track: i32,
    pub disc_number: i32,
    pub duration: i32,
}

#[derive(Clone, Default)]
pub struct SearchResults {
    pub artists: Vec<Artist>,
    pub albums: Vec<Album>,
    pub songs: Vec<Song>,
}

#[derive(Clone, Default)]
pub struct Payload {
    pub artists: Vec<Artist>,
    pub albums: Vec<Album>,
    pub songs: Vec<Song>,
    pub search: SearchResults,
}

#[derive(Clone, Debug)]
pub enum Request {
    Artists,
    Albums,
    Artist(String),
    Album(String),
    Search(String),
    Starred,
}

impl Auth {
    /// Subsonic authentication query parameters.
    fn query(&self) -> String {
        format!(
            "u={}&t={}&s={}&v=1.16.1&c=NaviRune",
            pct(&self.username),
            pct(&self.subsonic_token),
            pct(&self.subsonic_salt)
        )
    }
}

/// Minimal percent-encoding (unreserved characters plus a few safe ones).
pub fn pct(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for b in value.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn base(url: &str) -> String {
    url.trim_end_matches('/').to_string()
}

#[derive(Deserialize)]
struct Envelope {
    #[serde(rename = "subsonic-response")]
    response: Response,
}

#[derive(Deserialize)]
struct Response {
    status: String,
    #[serde(default)]
    error: Option<ApiError>,
    #[serde(default)]
    artists: Option<ArtistIndex>,
    #[serde(default, rename = "albumList2")]
    album_list: Option<AlbumList>,
    #[serde(default)]
    artist: Option<ArtistDetail>,
    #[serde(default)]
    album: Option<AlbumDetail>,
    #[serde(default, rename = "searchResult3")]
    search: Option<SearchPayload>,
    #[serde(default, rename = "starred2")]
    starred: Option<SearchPayload>,
}

#[derive(Deserialize)]
struct ApiError {
    #[serde(default)]
    message: String,
}

#[derive(Deserialize)]
struct ArtistIndex {
    #[serde(default)]
    index: Vec<IndexGroup>,
}

#[derive(Deserialize)]
struct IndexGroup {
    #[serde(default)]
    artist: Vec<Artist>,
}

#[derive(Deserialize)]
struct AlbumList {
    #[serde(default)]
    album: Vec<Album>,
}

#[derive(Deserialize)]
struct ArtistDetail {
    #[serde(flatten)]
    artist: Artist,
    #[serde(default)]
    album: Vec<Album>,
}

#[derive(Deserialize)]
struct AlbumDetail {
    #[serde(flatten)]
    album: Album,
    #[serde(default)]
    song: Vec<Song>,
}

#[derive(Deserialize, Default)]
struct SearchPayload {
    #[serde(default)]
    artist: Vec<Artist>,
    #[serde(default)]
    album: Vec<Album>,
    #[serde(default)]
    song: Vec<Song>,
}

fn into_payload(response: Response) -> Result<Payload, String> {
    if response.status != "ok" {
        return Err(response
            .error
            .map(|e| e.message)
            .unwrap_or_else(|| "Subsonic request failed".into()));
    }
    let mut out = Payload::default();
    if let Some(index) = response.artists {
        out.artists = index.index.into_iter().flat_map(|g| g.artist).collect();
    }
    if let Some(list) = response.album_list {
        out.albums = list.album;
    }
    if let Some(detail) = response.artist {
        out.albums = detail.album;
    }
    if let Some(detail) = response.album {
        out.songs = detail.song;
    }
    if let Some(search) = response.search.or(response.starred) {
        out.search = SearchResults {
            artists: search.artist,
            albums: search.album,
            songs: search.song,
        };
    }
    Ok(out)
}

fn parse_envelope(text: &str) -> Result<Payload, String> {
    let envelope: Envelope = serde_json::from_str(text)
        .map_err(|e| format!("Could not parse server response: {e}"))?;
    into_payload(envelope.response)
}

// ---------------------------------------------------------------------------
// Browser transport
// ---------------------------------------------------------------------------

// Rune has no DOM API, so read the config that Navidrome's Go server injects
// into index.html. `origin` is added here so the default Server URL just works.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(inline_js = r#"
export function ndConfig() {
  let raw = window.__APP_CONFIG__ || {};
  if (typeof raw === 'string') { try { raw = JSON.parse(raw); } catch { raw = {}; } }
  const c = Object.assign({}, raw);
  c.origin = window.location.origin;
  return JSON.stringify(c);
}
"#)]
extern "C" {
    fn ndConfig() -> String;
}

#[cfg(target_arch = "wasm32")]
fn config() -> serde_json::Value {
    serde_json::from_str(&ndConfig()).unwrap_or_default()
}

pub fn first_time() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        config()["firstTime"].as_bool().unwrap_or(false)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        false
    }
}

pub fn default_server() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        let c = config();
        let origin = c["origin"].as_str().unwrap_or("");
        let base = c["baseURL"].as_str().unwrap_or("");
        format!("{origin}{base}")
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        "http://localhost:4533".to_string()
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(inline_js = r#"
export function ndFetch(url, method, body) {
  const opts = {method: method || 'GET', headers: {Accept: 'application/json'}};
  if (body) { opts.headers['Content-Type'] = 'application/json'; opts.body = body; }
  return fetch(url, opts).then(async r => {
    const text = await r.text();
    return JSON.stringify({ok: r.ok, status: r.status, body: text});
  });
}
"#)]
extern "C" {
    fn ndFetch(url: &str, method: &str, body: &str) -> js_sys::Promise;
}

#[cfg(target_arch = "wasm32")]
pub async fn http(url: &str, method: &str, body: Option<&str>) -> Result<String, String> {
    let promise = ndFetch(url, method, body.unwrap_or(""));
    let value = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(|e| format!("Network error: {e:?}"))?;
    let text = value.as_string().unwrap_or_default();
    let result: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("Bridge error: {e}"))?;
    let ok = result["ok"].as_bool().unwrap_or(false);
    let body = result["body"].as_str().unwrap_or("").to_string();
    if ok {
        Ok(body)
    } else {
        Err(format!("HTTP {}", result["status"].as_u64().unwrap_or(0)))
    }
}

#[cfg(target_arch = "wasm32")]
async fn auth_request(path: &str, url: &str, username: &str, password: &str) -> Result<Auth, String> {
    let body = serde_json::json!({"username": username, "password": password}).to_string();
    let text = http(&format!("{}/{path}", base(url)), "POST", Some(&body)).await?;
    serde_json::from_str(&text).map_err(|e| format!("Invalid login response: {e}"))
}

#[cfg(target_arch = "wasm32")]
pub async fn login(url: &str, username: &str, password: &str) -> Result<Auth, String> {
    auth_request("auth/login", url, username, password).await
}

#[cfg(target_arch = "wasm32")]
pub async fn create_admin(url: &str, username: &str, password: &str) -> Result<Auth, String> {
    auth_request("auth/createAdmin", url, username, password).await
}

#[cfg(target_arch = "wasm32")]
pub async fn load(url: &str, auth: &Auth, req: Request) -> Result<Payload, String> {
    let root = base(url);
    let auth_q = auth.query();
    let (command, extra): (&str, String) = match req {
        Request::Artists => ("getArtists", String::new()),
        Request::Albums => (
            "getAlbumList2",
            "&type=alphabeticalByName&size=200&offset=0".into(),
        ),
        Request::Artist(id) => ("getArtist", format!("&id={}", pct(&id))),
        Request::Album(id) => ("getAlbum", format!("&id={}", pct(&id))),
        Request::Search(q) => (
            "search3",
            format!(
                "&query={}&artistCount=20&albumCount=40&songCount=40",
                pct(&q)
            ),
        ),
        Request::Starred => ("getStarred2", String::new()),
    };
    let url = format!("{root}/rest/{command}?{auth_q}{extra}");
    let text = http(&url, "GET", None).await?;
    parse_envelope(&text)
}

// ---------------------------------------------------------------------------
// Native demo transport (used for headless `--snap` / `--tree` checks)
// ---------------------------------------------------------------------------

#[cfg(not(target_arch = "wasm32"))]
pub fn demo_auth() -> Auth {
    Auth {
        username: "demo".into(),
        subsonic_salt: "00ff".into(),
        subsonic_token: "demo".into(),
        token: "demo".into(),
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn demo_payload(req: Request) -> Payload {
    let artists = vec![
        Artist { id: "ar-1".into(), name: "The Beatles".into(), cover_art: "ar-1".into(), album_count: 4 },
        Artist { id: "ar-2".into(), name: "Björk".into(), cover_art: "ar-2".into(), album_count: 3 },
        Artist { id: "ar-3".into(), name: "Miles Davis".into(), cover_art: "ar-3".into(), album_count: 5 },
    ];
    let albums = vec![
        Album { id: "al-1".into(), name: "Abbey Road".into(), artist: "The Beatles".into(), artist_id: "ar-1".into(), cover_art: "al-1".into(), song_count: 17, duration: 2799, year: 1969, genre: "Rock".into() },
        Album { id: "al-2".into(), name: "Debut".into(), artist: "Björk".into(), artist_id: "ar-2".into(), cover_art: "al-2".into(), song_count: 13, duration: 2485, year: 1993, genre: "Electronic".into() },
        Album { id: "al-3".into(), name: "Kind of Blue".into(), artist: "Miles Davis".into(), artist_id: "ar-3".into(), cover_art: "al-3".into(), song_count: 9, duration: 2555, year: 1959, genre: "Jazz".into() },
    ];
    let songs = vec![
        Song { id: "mf-1".into(), title: "Come Together".into(), album: "Abbey Road".into(), artist: "The Beatles".into(), album_id: "al-1".into(), cover_art: "al-1".into(), track: 1, disc_number: 1, duration: 259 },
        Song { id: "mf-2".into(), title: "Something".into(), album: "Abbey Road".into(), artist: "The Beatles".into(), album_id: "al-1".into(), cover_art: "al-1".into(), track: 2, disc_number: 1, duration: 182 },
        Song { id: "mf-3".into(), title: "Human Nature".into(), album: "Début".into(), artist: "Björk".into(), album_id: "al-2".into(), cover_art: "al-2".into(), track: 3, disc_number: 1, duration: 268 },
    ];
    match req {
        Request::Artists => Payload { artists, ..Default::default() },
        Request::Albums | Request::Artist(_) | Request::Starred => Payload { albums, ..Default::default() },
        Request::Album(_) => Payload { songs, ..Default::default() },
        Request::Search(_) => Payload {
            search: SearchResults { artists, albums, songs },
            ..Default::default()
        },
    }
}

// ---------------------------------------------------------------------------
// Audio bridge (browser <audio> element; Rune itself has no web audio)
// ---------------------------------------------------------------------------

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(inline_js = r#"
export function ndPlay(url) {
  const a = document.getElementById('nd-audio');
  if (!a) return false;
  a.src = url; a.play().catch(() => {}); return true;
}
export function ndPause() {
  const a = document.getElementById('nd-audio');
  if (a) a.pause();
}
"#)]
extern "C" {
    fn ndPlay(url: &str) -> bool;
    fn ndPause();
}

pub fn play_stream(stream_url: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        ndPlay(stream_url);
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = stream_url;
}

pub fn pause() {
    #[cfg(target_arch = "wasm32")]
    ndPause();
}

pub fn stream_url(url: &str, auth: &Auth, song_id: &str) -> String {
    format!(
        "{}/rest/stream?id={}&{}",
        base(url),
        pct(song_id),
        auth.query()
    )
}

pub fn cover_url(url: &str, auth: &Auth, cover_id: &str, size: u32) -> String {
    if cover_id.is_empty() {
        return String::new();
    }
    format!(
        "{}/rest/getCoverArt?id={}&size={}&square=true&{}",
        base(url),
        pct(cover_id),
        size,
        auth.query()
    )
}
