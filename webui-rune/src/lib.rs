//! Navidrome Remake web interface, built with Rune.
//!
//! The same `view` drives a native window (handy for headless `--snap` checks)
//! and a WebAssembly browser app that talks to Navidrome's Subsonic API.

#![forbid(unsafe_code)]

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
mod api;

use api::{Album, Artist, Auth, Payload, Request, Song};
use rune::{AppHandle, AppSpec, Cx, El, ImageCache, ScrollSpec, SurfaceFit, TextEdit, col, el, row, text, text_input, tok};

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum Tab {
    #[default]
    Albums,
    Artists,
    Search,
    Favorites,
    Settings,
}

#[derive(Clone)]
enum Detail {
    Artist(Artist),
    Album(Album),
}

struct State {
    // Connection / login
    server: TextEdit,
    username: TextEdit,
    password: TextEdit,
    auth: Auth,
    logged_in: bool,
    first_time: bool,
    busy: bool,
    error: String,

    // Navigation
    tab: Tab,
    detail: Option<Detail>,
    query: TextEdit,

    // Data
    artists: Vec<Artist>,
    albums: Vec<Album>,
    artist_albums: Vec<Album>,
    album_songs: Vec<Song>,
    search: api::SearchResults,

    // Admin
    admin_status: String,

    // Runtime
    images: Option<ImageCache>,
    handle: Option<AppHandle<State>>,
    pending: Option<Request>,
    now_playing: Option<Song>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            server: TextEdit::new(api::default_server()),
            username: TextEdit::new(""),
            password: TextEdit::new("").secure(),
            auth: Auth::default(),
            logged_in: false,
            first_time: api::first_time(),
            busy: false,
            error: String::new(),
            tab: Tab::Albums,
            detail: None,
            query: TextEdit::new(""),
            artists: Vec::new(),
            albums: Vec::new(),
            artist_albums: Vec::new(),
            album_songs: Vec::new(),
            search: api::SearchResults::default(),
            admin_status: String::new(),
            images: None,
            handle: None,
            pending: None,
            now_playing: None,
        }
    }
}

impl State {
    fn apply(&mut self, payload: Payload) {
        self.busy = false;
        match self.pending.take() {
            Some(Request::Artists) => self.artists = payload.artists,
            Some(Request::Albums) => self.albums = payload.albums,
            Some(Request::Artist(_)) => self.artist_albums = payload.albums,
            Some(Request::Album(_)) => self.album_songs = payload.songs,
            Some(Request::Starred) => self.albums = payload.search.albums,
            Some(Request::Search(_)) => self.search = payload.search,
            None => {}
        }
    }

    fn fail(&mut self, message: String) {
        self.busy = false;
        self.error = message;
    }
}

// Dispatch a data request. The wasm path performs a real fetch; native uses
// deterministic demo data so the UI can be rendered headlessly.
fn dispatch(handle: AppHandle<State>, base: String, auth: Auth, req: Request) {
    #[cfg(target_arch = "wasm32")]
    {
        wasm_bindgen_futures::spawn_local(async move {
            match api::load(&base, &auth, req).await {
                Ok(payload) => handle.update(move |s, tx| {
                    s.apply(payload);
                    tx.instant();
                }),
                Err(e) => handle.update(move |s, tx| {
                    s.fail(e);
                    tx.instant();
                }),
            }
        });
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (base, auth);
        handle.update(move |s, tx| {
            s.apply(api::demo_payload(req));
            tx.instant();
        });
    }
}

fn request_for(tab: Tab, detail: &Option<Detail>, query: &str) -> Request {
    match detail {
        Some(Detail::Artist(a)) => Request::Artist(a.id.clone()),
        Some(Detail::Album(a)) => Request::Album(a.id.clone()),
        None => match tab {
            Tab::Artists => Request::Artists,
            Tab::Favorites => Request::Starred,
            Tab::Search if !query.trim().is_empty() => Request::Search(query.trim().to_string()),
            _ => Request::Albums,
        },
    }
}

fn reload(s: &mut State) {
    let Some(handle) = s.handle.clone() else { return };
    let req = request_for(s.tab, &s.detail, &s.query.text);
    s.pending = Some(req.clone());
    s.busy = true;
    s.error.clear();
    dispatch(handle, s.server.text.clone(), s.auth.clone(), req);
}

fn navigate(s: &mut State, tab: Tab) {
    s.tab = tab;
    s.detail = None;
    if tab == Tab::Settings {
        s.admin_status.clear();
        return;
    }
    reload(s);
}

fn admin_cmd(s: &mut State, command: &'static str) {
    let Some(handle) = s.handle.clone() else { return };
    let base = s.server.text.clone();
    let auth = s.auth.clone();
    s.admin_status = "Working…".into();

    #[cfg(target_arch = "wasm32")]
    wasm_bindgen_futures::spawn_local(async move {
        let result = api::command(&base, &auth, command, "").await;
        handle.update(move |s, tx| {
            s.admin_status = match result {
                Ok(v) if v["status"] == "ok" => {
                    if command == "getScanStatus" {
                        format!(
                            "Scan status: {} · {} files indexed",
                            v["scanStatus"]["scanning"].as_bool().unwrap_or(false),
                            v["scanStatus"]["count"].as_u64().unwrap_or(0)
                        )
                    } else {
                        "Command accepted".into()
                    }
                }
                Ok(v) => format!("Server error: {}", v["error"]["message"].as_str().unwrap_or("unknown")),
                Err(e) => e,
            };
            tx.instant();
        });
    });

    #[cfg(not(target_arch = "wasm32"))]
    handle.update(move |s, tx| {
        s.admin_status = format!("{command} accepted (demo)");
        tx.instant();
    });
}

fn login(s: &mut State) {
    let Some(handle) = s.handle.clone() else { return };
    let base = s.server.text.clone();
    let username = s.username.text.clone();
    let password = s.password.text.clone();
    #[cfg(target_arch = "wasm32")]
    let s_first_time = s.first_time;
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (&base, &username, &password);
    s.busy = true;
    s.error.clear();

    #[cfg(target_arch = "wasm32")]
    wasm_bindgen_futures::spawn_local(async move {
        let result = if s_first_time {
            api::create_admin(&base, &username, &password).await
        } else {
            api::login(&base, &username, &password).await
        };
        match result {
            Ok(auth) => handle.update(move |s, tx| {
                s.auth = auth;
                s.logged_in = true;
                s.busy = false;
                reload(s);
                tx.instant();
            }),
            Err(e) => handle.update(move |s, tx| {
                s.fail(e);
                tx.instant();
            }),
        }
    });

    #[cfg(not(target_arch = "wasm32"))]
    handle.update(move |s, tx| {
        s.auth = api::demo_auth();
        s.logged_in = true;
        s.busy = false;
        reload(s);
        tx.instant();
    });
}

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

fn view(s: &State, cx: &mut Cx<State>) -> El {
    let t = tok::theme(cx.system_dark());
    let scale = cx.env().scale;
    let [top, right, bottom, left] = cx.safe_area();

    if !s.logged_in {
        return login_view(s, &t, top, right, bottom, left);
    }

    // Local helpers capture the theme and the image cache.
    let cover = |cover_id: &str, key: String, size: f32| -> El {
        let mut box_el = el()
            .key(key)
            .size(size, size)
            .radius(8.0)
            .fill(t.control_off);
        if let (Some(cache), true) = (s.images.as_ref(), !cover_id.is_empty()) {
            let url = api::cover_url(&s.server.text, &s.auth, cover_id, 160);
            let image = cache.url(&url, size, scale);
            if image.error().is_none() {
                box_el = box_el.pixels_fit(image.pixels(), SurfaceFit::Fill);
            }
        }
        box_el
    };

    let tab_button = |label: &'static str, target: Tab, active: bool| -> El {
        el().key(format!("tab-{label}"))
            .role("button")
            .label(label)
            .h(38.0)
            .px(12.0)
            .center()
            .radius(9.0)
            .fill(if active { tok::accent(0, true) } else { t.control_off })
            .on_press(move |s: &mut State| navigate(s, target))
            .child(text(label).font(tok::type_::LABEL).color(if active { t.bg } else { t.text }))
    };

    let mut sidebar = col().key("sidebar").w(150.0).gap(4.0).padding(10.0).fill(t.card);
    sidebar = sidebar
        .child(tab_button("Albums", Tab::Albums, s.tab == Tab::Albums && s.detail.is_none()))
        .child(tab_button("Artists", Tab::Artists, s.tab == Tab::Artists && s.detail.is_none()))
        .child(tab_button("Search", Tab::Search, s.tab == Tab::Search && s.detail.is_none()))
        .child(tab_button("Favorites", Tab::Favorites, s.tab == Tab::Favorites && s.detail.is_none()));
    let sidebar = if s.auth.is_admin {
        sidebar.child(tab_button("Settings", Tab::Settings, s.tab == Tab::Settings && s.detail.is_none()))
    } else {
        sidebar
    };

    let mut content = col().key("content").w_full().gap(8.0).padding(14.0);
    if s.detail.is_some() {
        content = content.child(
            el().key("back").h(34.0).px(10.0).center().radius(8.0).fill(t.control_off)
                .on_press(|s: &mut State| { s.detail = None; reload(s); })
                .child(text("← Back").font(tok::type_::LABEL).color(t.text)),
        );
    }

    match &s.detail {
        Some(Detail::Artist(artist)) => {
            content = content
                .child(cover(&artist.cover_art, "artist-art".into(), 96.0))
                .child(text(artist.name.clone()).font(tok::type_::TITLE).color(t.text))
                .child(text(format!("{} albums", artist.album_count)).font(tok::type_::BODY).color(t.text_secondary));
            for album in &s.artist_albums {
                content = content.child(album_row(album, &cover, &t));
            }
        }
        Some(Detail::Album(album)) => {
            content = content
                .child(cover(&album.cover_art, "album-art".into(), 160.0))
                .child(text(album.name.clone()).font(tok::type_::TITLE).color(t.text))
                .child(text(format!("{} · {}", album.artist, album.year)).font(tok::type_::BODY).color(t.text_secondary));
            for song in &s.album_songs {
                content = content.child(song_row(song, &s.server.text, &s.auth, &t));
            }
        }
        None => match s.tab {
            Tab::Artists => {
                content = content.child(text("Artists").font(tok::type_::TITLE).color(t.text));
                for artist in &s.artists {
                    let id = artist.id.clone();
                    let key_id = id.clone();
                    let cover_id = artist.cover_art.clone();
                    let name = artist.name.clone();
                    let count = artist.album_count;
                    content = content.child(
                        row().key(format!("artist-{key_id}")).w_full().gap(12.0).padding(8.0)
                            .radius(10.0).fill(t.bg)
                            .on_press(move |s: &mut State| {
                                let artist = s.artists.iter().find(|a| a.id == id).cloned();
                                if let Some(artist) = artist { s.detail = Some(Detail::Artist(artist)); reload(s); }
                            })
                            .child(cover(&cover_id, format!("artist-cover-{key_id}"), 52.0))
                            .child(text(format!("{name} · {count} albums")).font(tok::type_::BODY).color(t.text).grow(1.0)),
                    );
                }
            }
            Tab::Albums | Tab::Favorites => {
                let heading = if s.tab == Tab::Favorites { "Favorites" } else { "Albums" };
                content = content.child(text(heading).font(tok::type_::TITLE).color(t.text));
                for album in &s.albums {
                    content = content.child(album_row(album, &cover, &t));
                }
            }
            Tab::Search => {
                let search_input = text_input(&s.query, tok::type_::BODY)
                    .key("search-input")
                    .placeholder("Search artists, albums, songs", t.text_secondary)
                    .color(t.text)
                    .single_line()
                    .h(44.0)
                    .padding(10.0)
                    .radius(10.0)
                    .fill(t.card)
                    .on_edit(|s: &mut State| &mut s.query)
                    .on_submit(|s: &mut State, _tx| reload(s));
                content = content.child(search_input);
                if !s.search.artists.is_empty() {
                    content = content.child(text("Artists").font(tok::type_::LABEL).color(t.text_secondary));
                }
                for artist in &s.search.artists {
                    let id = artist.id.clone();
                    let name = artist.name.clone();
                    content = content.child(
                        el().key(format!("s-artist-{id}")).w_full().padding(8.0).radius(8.0).fill(t.card)
                            .on_press(move |s: &mut State| {
                                if let Some(a) = s.search.artists.iter().find(|a| a.id == id).cloned() {
                                    s.detail = Some(Detail::Artist(a)); reload(s);
                                }
                            })
                            .child(text(name).font(tok::type_::BODY).color(t.text)),
                    );
                }
                if !s.search.albums.is_empty() {
                    content = content.child(text("Albums").font(tok::type_::LABEL).color(t.text_secondary));
                }
                for album in &s.search.albums {
                    content = content.child(album_row(album, &cover, &t));
                }
                if !s.search.songs.is_empty() {
                    content = content.child(text("Songs").font(tok::type_::LABEL).color(t.text_secondary));
                }
                for song in &s.search.songs {
                    content = content.child(song_row(song, &s.server.text, &s.auth, &t));
                }
            }
            Tab::Settings => {
                content = settings_view(s, &t);
            }
        },
    }

    if !s.error.is_empty() {
        content = content.child(text(s.error.clone()).font(tok::type_::BODY).color(t.text));
    }

    col().w_full().h_full().pt(top + 10.0).pr(right + 10.0).pb(bottom + 10.0).pl(left + 10.0)
        .gap(10.0)
        .fill(t.bg)
        .child(
            row().w_full().gap(10.0).center_y()
                .child(text("Navidrome Remake").font(tok::type_::TITLE).color(t.text).grow(1.0))
                .child(text(s.auth.username.clone()).font(tok::type_::LABEL).color(t.text_secondary)),
        )
        .child(row().w_full().grow(1.0).min_h(0.0).gap(10.0).child(sidebar)
            .child(col().key("scroll").w_full().grow(1.0).min_h(0.0)
                .scroll(ScrollSpec { stick_bottom: false }).child(content)))
        .child(now_playing(s, &t))
}

fn settings_view(s: &State, t: &rune::tok::Theme) -> El {
    let button = |key: &'static str, label: &'static str, command: &'static str| -> El {
        el().key(key).w(260.0).h(42.0).center().radius(9.0).fill(t.control_off)
            .on_press(move |s: &mut State| admin_cmd(s, command))
            .child(text(label).font(tok::type_::LABEL).color(t.text))
    };

    col().w_full().gap(12.0).padding(6.0)
        .child(text("Settings").font(tok::type_::TITLE).color(t.text))
        .child(col().w_full().gap(4.0).padding(12.0).radius(10.0).fill(t.card)
            .child(text("Server").font(tok::type_::LABEL).color(t.text_secondary))
            .child(text(format!("Version {}", api::config_string("version"))).font(tok::type_::BODY).color(t.text))
            .child(text(format!("Base path {}", api::config_string("baseURL"))).font(tok::type_::BODY).color(t.text))
            .child(text(format!("Signed in as {}", s.auth.username)).font(tok::type_::BODY).color(t.text)))
        .child(col().w_full().gap(4.0).padding(12.0).radius(10.0).fill(t.card)
            .child(text("Artist parsing").font(tok::type_::LABEL).color(t.text_secondary))
            .child(text("Accent/case-insensitive identities, collaboration splitting, alias and CV cleanup").font(tok::type_::BODY).color(t.text))
            .child(text("Changes apply to newly scanned files; run a full scan to rebuild all artists.").font(tok::type_::BODY).color(t.text_secondary)))
        .child(col().w_full().gap(8.0).padding(12.0).radius(10.0).fill(t.card)
            .child(text("Library").font(tok::type_::LABEL).color(t.text_secondary))
            .child(button("start-scan", "Start full scan", "startScan"))
            .child(button("scan-status", "Check scan status", "getScanStatus"))
            .child(text(s.admin_status.clone()).font(tok::type_::BODY).color(t.text_secondary)))
}

fn album_row(album: &Album, cover: &impl Fn(&str, String, f32) -> El, t: &rune::tok::Theme) -> El {
    let id = album.id.clone();
    let key_id = id.clone();
    let cover_id = album.cover_art.clone();
    row().key(format!("album-{key_id}")).w_full().gap(12.0).padding(8.0).radius(10.0).fill(t.card)
        .on_press(move |s: &mut State| {
            if let Some(a) = s.albums.iter().find(|a| a.id == id).cloned().or_else(|| {
                s.artist_albums.iter().find(|a| a.id == id).cloned()
            }).or_else(|| s.search.albums.iter().find(|a| a.id == id).cloned()) {
                s.detail = Some(Detail::Album(a)); reload(s);
            }
        })
        .child(cover(&cover_id, format!("album-cover-{key_id}"), 52.0))
        .child(text(format!("{}\n{} · {}", album.name, album.artist, album.year))
            .font(tok::type_::BODY).color(t.text).grow(1.0))
}

fn song_row(song: &Song, base: &str, auth: &Auth, t: &rune::tok::Theme) -> El {
    let song = song.clone();
    let url = api::stream_url(base, auth, &song.id);
    let title = song.title.clone();
    row().key(format!("song-{}", song.id)).w_full().gap(10.0).padding(8.0).radius(8.0).fill(t.bg)
        .child(text(song.track.to_string()).font(tok::type_::LABEL).color(t.text_secondary).w(28.0))
        .child(text(title).font(tok::type_::BODY).color(t.text).grow(1.0))
        .child(el().key(format!("play-{}", song.id)).size(34.0, 34.0).center().radius(17.0)
            .fill(t.control_off)
            .on_press(move |s: &mut State| {
                api::play_stream(&url);
                s.now_playing = Some(song.clone());
            })
            .child(text("▶").font(tok::type_::LABEL).color(t.text)))
}

fn now_playing(s: &State, t: &rune::tok::Theme) -> El {
    let title = s.now_playing.as_ref().map(|song| song.title.as_str()).unwrap_or("Nothing playing");
    row().key("now-playing").w_full().h(52.0).gap(10.0).padding(10.0).radius(10.0).fill(t.card).center_y()
        .child(text(title).font(tok::type_::BODY).color(t.text).grow(1.0))
        .child(el().key("pause").h(34.0).px(12.0).center().radius(8.0).fill(t.control_off)
            .on_press(|_: &mut State| api::pause())
            .child(text("❚❚").font(tok::type_::LABEL).color(t.text)))
}

fn login_view(s: &State, t: &rune::tok::Theme, top: f32, right: f32, bottom: f32, left: f32) -> El {
    let field = |key: &'static str,
                 label: &'static str,
                 secure: bool,
                 edit: &TextEdit,
                 access: for<'a> fn(&'a mut State) -> &'a mut TextEdit|
     -> El {
        let input = text_input(edit, tok::type_::BODY)
            .key(key)
            .placeholder(label, t.text_secondary)
            .color(t.text)
            .single_line()
            .h(46.0)
            .padding(10.0)
            .radius(10.0)
            .fill(t.card)
            .on_edit(access);
        let input = if secure { input.secure() } else { input };
        col().w_full().gap(4.0)
            .child(text(label).font(tok::type_::LABEL).color(t.text_secondary))
            .child(input)
    };
    col().w_full().h_full().pt(top + 20.0).pr(right + 20.0).pb(bottom + 20.0).pl(left + 20.0)
        .gap(14.0).fill(t.bg).center()
        .child(text("Navidrome Remake").font(tok::type_::TITLE).color(t.text))
        .child(text(if s.first_time { "Create your admin account" } else { "Sign in to your music server" })
            .font(tok::type_::BODY).color(t.text_secondary))
        .child(field("server", "Server URL", false, &s.server, |s| &mut s.server).w(320.0))
        .child(field("username", "Username", false, &s.username, |s| &mut s.username).w(320.0))
        .child(field("password", "Password", true, &s.password, |s| &mut s.password).w(320.0))
        .child(el().key("login").w(320.0).h(46.0).center().radius(10.0)
            .fill(tok::accent(0, true))
            .on_press(login)
            .child(text(if s.busy {
                "Connecting…"
            } else if s.first_time {
                "Create admin"
            } else {
                "Sign in"
            }).font(tok::type_::LABEL).color(t.bg)))
        .child(if s.error.is_empty() { el() } else {
            text(s.error.clone()).font(tok::type_::BODY).color(t.text)
        })
}

#[cfg(not(target_arch = "wasm32"))]
const SCENES: &[(&str, fn(&mut State))] = &[
    ("login", |s| s.logged_in = false),
    ("first-time", |s| {
        s.logged_in = false;
        s.first_time = true;
    }),
    ("albums", |s| {
        s.logged_in = true;
        s.tab = Tab::Albums;
        s.auth = api::demo_auth();
        s.albums = api::demo_payload(Request::Albums).albums;
        s.artists = api::demo_payload(Request::Artists).artists;
    }),
    ("artists", |s| {
        s.logged_in = true;
        s.tab = Tab::Artists;
        s.auth = api::demo_auth();
        s.artists = api::demo_payload(Request::Artists).artists;
    }),
    ("settings", |s| {
        s.logged_in = true;
        s.tab = Tab::Settings;
        s.auth = api::demo_auth();
        s.admin_status = "Ready".into();
    }),
    ("album", |s| {
        s.logged_in = true;
        s.tab = Tab::Albums;
        s.auth = api::demo_auth();
        let album = api::demo_payload(Request::Albums).albums.remove(0);
        s.detail = Some(Detail::Album(album));
        s.album_songs = api::demo_payload(Request::Album("al-1".into())).songs;
    }),
];

#[cfg(target_arch = "wasm32")]
const SCENES: &[(&str, fn(&mut State))] = &[];

pub fn app() -> rune::App<State> {
    let mut spec = AppSpec::new("Navidrome Remake", (1100.0, 760.0), State::default, view);
    spec.scenes = SCENES;
    spec.setup = Some(|s, waker| {
        s.images = Some(ImageCache::new(waker));
    });
    let app = rune::App::new(spec);
    let handle = app.handle();
    let clone = handle.clone();
    handle.update(move |s, _tx| s.handle = Some(clone));
    app
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen(start))]
pub fn start() {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = app().try_run();
    }
    #[cfg(not(target_arch = "wasm32"))]
    app().run();
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use rune::{Env, Key, KeyEvent, Mods, Runtime};

    fn env() -> Env {
        Env { system_dark: true, scale: 2.0 }
    }

    #[test]
    fn login_screen_exposes_inputs_and_button() {
        let mut rt = Runtime::new(State::default(), view, (1100.0, 760.0), env());
        rt.frame(0.0);
        for key in ["server", "username", "password", "login"] {
            assert!(rt.find(key).is_some(), "missing element {key}");
        }
    }

    #[test]
    fn albums_scene_lists_albums_and_now_playing_bar() {
        let mut state = State::default();
        SCENES[2].1(&mut state);
        let mut rt = Runtime::new(state, view, (1100.0, 760.0), env());
        rt.frame(0.0);
        assert!(rt.find("content").is_some());
        assert!(rt.find("album-al-1").is_some(), "Abbey Road row is missing");
        assert!(rt.find("album-al-2").is_some(), "Debut row is missing");
        assert!(rt.find("now-playing").is_some());
    }

    #[test]
    fn minimal_input_focus_check() {
        struct S { t: TextEdit }
        fn v(s: &S, _: &mut Cx<S>) -> El {
            text_input(&s.t, tok::type_::BODY).key("f").size(200.0, 40.0).on_edit(|s: &mut S| &mut s.t)
        }
        let mut rt = Runtime::new(S { t: TextEdit::new("") }, v, (400.0, 200.0), env());
        rt.frame(0.0);
        let (x, y) = rt.find("f").unwrap();
        rt.pointer_down(x, y);
        rt.pointer_up(x, y);
        rt.frame(0.0);
        rt.key(KeyEvent { key: Key::Text("x".into()), mods: Mods::default() });
        rt.frame(0.0);
        assert_eq!(rt.state.t.text, "x");
    }

    #[test]
    fn login_fields_accept_click_and_typed_text() {
        let mut rt = Runtime::new(State::default(), view, (1100.0, 760.0), env());
        rt.frame(0.0);
        let (x, y) = rt.find("username").expect("username field not found");
        rt.pointer_down(x, y);
        rt.pointer_up(x, y);
        rt.frame(0.0);
        rt.key(KeyEvent { key: Key::Text("admin".into()), mods: Mods::default() });
        rt.frame(0.0);
        assert_eq!(rt.state.username.text, "admin", "clicking then typing must fill the field");
    }

    #[test]
    fn admin_settings_scene_has_scan_actions() {
        let mut state = State::default();
        SCENES[4].1(&mut state);
        let mut rt = Runtime::new(state, view, (1100.0, 760.0), env());
        rt.frame(0.0);
        assert!(rt.find("start-scan").is_some());
        assert!(rt.find("scan-status").is_some());
        assert!(rt.find("tab-Settings").is_some());
    }

    #[test]
    fn album_scene_lists_tracks() {
        let mut state = State::default();
        SCENES[5].1(&mut state);
        let mut rt = Runtime::new(state, view, (1100.0, 760.0), env());
        rt.frame(0.0);
        assert!(rt.find("song-mf-1").is_some());
        assert!(rt.find("play-mf-1").is_some());
        assert!(rt.find("back").is_some());
    }
}
