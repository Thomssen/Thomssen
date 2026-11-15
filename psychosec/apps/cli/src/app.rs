//! Application state, input handling and command dispatch.
use crate::api::{Api, ApiError, Release, SessionInfo};
use crate::commands::{self, Command};
use crate::config::{self, Settings, APP_NAME, VERSION};
use crate::osint;
use crate::output::{OutLine, Output, Status};
use crate::report;
use crate::store::Store;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::cell::Cell;
use std::future::Future;
use std::path::PathBuf;
use std::time::Duration;
use tokio::runtime::Handle;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;

pub enum Msg {
    Result(Output),
    Notice(Status, String),
    Done,
    LoginUrl(String),
    Authed { info: SessionInfo, token: String },
    Locked(String),
    UpdateFound(Release),
    UpdateReady(PathBuf, String),
}

#[derive(Clone)]
pub enum AuthState {
    Checking,
    Waiting(#[allow(dead_code)] String),
    Authorized(SessionInfo),
    Locked(String),
}

pub struct Busy {
    pub label: String,
    handle: JoinHandle<()>,
}

enum Confirm {
    DeleteCase(i64, String),
    InstallUpdate(PathBuf),
}

pub struct App {
    rt: Handle,
    tx: UnboundedSender<Msg>,
    rx: UnboundedReceiver<Msg>,
    api: Api,
    pub settings: Settings,
    store: Store,
    pub auth: AuthState,
    token: Option<String>,
    pub input: String,
    pub cursor: usize, // char index
    history: Vec<String>,
    hist_pos: Option<usize>,
    comp: Option<(String, Vec<&'static str>, usize)>,
    pub lines: Vec<OutLine>,
    pub scroll: usize,
    pub max_scroll: Cell<usize>,
    pub busy: Option<Busy>,
    last: Option<Output>,
    pub active_case: Option<(i64, String)>,
    confirm: Option<Confirm>,
    update: Option<Release>,
    update_path: Option<PathBuf>,
    install_after_download: bool,
    bg: Vec<JoinHandle<()>>,
    pub tick: u64,
    pub should_quit: bool,
}

const MAX_LINES: usize = 6000;

impl App {
    pub fn new(rt: Handle) -> Result<Self, String> {
        let crypto = crate::crypto::Crypto::load_or_create()?;
        let store = Store::open(&config::data_dir().join("cases.db"), crypto)?;
        let (tx, rx) = unbounded_channel();
        Ok(Self {
            rt,
            tx,
            rx,
            api: Api::new()?,
            settings: Settings::load(),
            store,
            auth: AuthState::Checking,
            token: None,
            input: String::new(),
            cursor: 0,
            history: vec![],
            hist_pos: None,
            comp: None,
            lines: vec![],
            scroll: 0,
            max_scroll: Cell::new(0),
            busy: None,
            last: None,
            active_case: None,
            confirm: None,
            update: None,
            update_path: None,
            install_after_download: false,
            bg: vec![],
            tick: 0,
            should_quit: false,
        })
    }

    pub fn authorized(&self) -> bool {
        matches!(self.auth, AuthState::Authorized(_))
    }
    pub fn confirm_prompt(&self) -> bool {
        self.confirm.is_some()
    }

    // ---------- output helpers ----------
    pub fn push(&mut self, l: OutLine) {
        self.lines.push(l);
        if self.lines.len() > MAX_LINES {
            self.lines.drain(0..MAX_LINES / 10);
        }
        self.scroll = 0;
    }
    fn notice(&mut self, s: Status, t: impl Into<String>) {
        self.push(OutLine::Notice(s, t.into()));
    }
    fn blank_if_needed(&mut self) {
        if !matches!(self.lines.last(), Some(OutLine::Blank)) {
            self.push(OutLine::Blank);
        }
    }

    // ---------- startup / auth ----------
    pub fn start(&mut self) {
        let api = self.api.clone();
        let tx = self.tx.clone();
        let h = self.rt.spawn(async move { bootstrap(api, tx).await });
        self.bg.push(h);
    }

    fn start_login(&mut self) {
        self.abort_bg();
        self.auth = AuthState::Checking;
        let api = self.api.clone();
        let tx = self.tx.clone();
        let h = self.rt.spawn(async move { browser_login(api, tx).await });
        self.bg.push(h);
    }

    fn abort_bg(&mut self) {
        for h in self.bg.drain(..) {
            h.abort();
        }
    }

    fn on_authed(&mut self, info: SessionInfo, token: String) {
        let first = !self.authorized();
        self.token = Some(token.clone());
        self.auth = AuthState::Authorized(info.clone());
        if first {
            self.blank_if_needed();
            self.notice(Status::Ok, format!("Authenticated as {}. Access authorized.", info.user.display_name.clone().unwrap_or(info.user.username.clone())));
        }
        // heartbeat + refresh loop
        let (api, tx, t) = (self.api.clone(), self.tx.clone(), token.clone());
        let h = self.rt.spawn(async move {
            let mut first = true;
            let mut n = 0u32;
            loop {
                match api.heartbeat(&t, first).await {
                    Ok(_) => {}
                    Err(ApiError::Unauthorized(m)) | Err(ApiError::Forbidden(m)) => {
                        let _ = tx.send(Msg::Locked(m));
                        return;
                    }
                    Err(_) => {} // transient network problems never lock the user out locally
                }
                first = false;
                n += 1;
                if n % 6 == 0 {
                    if let Err(ApiError::Unauthorized(m)) | Err(ApiError::Forbidden(m)) = api.refresh(&t).await {
                        let _ = tx.send(Msg::Locked(m));
                        return;
                    }
                }
                tokio::time::sleep(Duration::from_secs(300)).await;
            }
        });
        self.bg.push(h);
        if self.settings.update_check && first {
            self.spawn_update_check(false);
        }
    }

    fn spawn_update_check(&mut self, announce_none: bool) {
        let (api, tx, t) = (self.api.clone(), self.tx.clone(), self.token.clone());
        let auto = self.settings.auto_download;
        let h = self.rt.spawn(async move {
            match api.latest_release(t.as_deref()).await {
                Ok(Some(r)) if crate::update::is_newer(&r.version, VERSION) => match crate::update::validate(&r) {
                    Ok(()) => {
                        let _ = tx.send(Msg::UpdateFound(r.clone()));
                        if auto {
                            match crate::update::download(&r, |_, _| {}).await {
                                Ok(p) => {
                                    let _ = tx.send(Msg::UpdateReady(p, r.version));
                                }
                                Err(e) => {
                                    let _ = tx.send(Msg::Notice(Status::Err, format!("update download failed: {e}")));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Msg::Notice(Status::Warn, format!("update {} ignored: {e}", r.version)));
                    }
                },
                Ok(_) => {
                    if announce_none {
                        let _ = tx.send(Msg::Notice(Status::Ok, format!("You are on the latest version ({VERSION}).")));
                    }
                }
                Err(e) => {
                    if announce_none {
                        let _ = tx.send(Msg::Notice(Status::Err, format!("update check failed: {e}")));
                    }
                }
            }
        });
        self.bg.push(h);
    }

    // ---------- message pump ----------
    pub fn pump(&mut self) {
        self.tick = self.tick.wrapping_add(1);
        while let Ok(m) = self.rx.try_recv() {
            match m {
                Msg::Result(o) => {
                    let l = o.to_lines();
                    self.lines.extend(l);
                    self.scroll = 0;
                    self.last = Some(o);
                    if self.active_case.is_some() {
                        self.notice(Status::Dim, "Save this result with 'case add'.");
                    }
                }
                Msg::Notice(s, t) => self.notice(s, t),
                Msg::Done => self.busy = None,
                Msg::LoginUrl(u) => {
                    self.auth = AuthState::Waiting(u.clone());
                    self.blank_if_needed();
                    self.notice(Status::Info, "Opening your browser to sign in with Discord. Press Esc to cancel.");
                    self.push(OutLine::Text(format!("If it did not open, visit: {u}")));
                }
                Msg::Authed { info, token } => self.on_authed(info, token),
                Msg::Locked(m) => {
                    self.abort_bg();
                    self.token = None;
                    crate::creds::delete(crate::creds::SESSION_TOKEN);
                    if let Some(b) = self.busy.take() {
                        b.handle.abort();
                    }
                    self.auth = AuthState::Locked(m.clone());
                    self.blank_if_needed();
                    self.notice(Status::Err, format!("Access denied: {m}"));
                    self.notice(Status::Dim, "Type 'login' to sign in again, or 'exit'.");
                }
                Msg::UpdateFound(r) => {
                    self.blank_if_needed();
                    self.notice(Status::Info, format!("Update available: {} (current {VERSION}). Signature verified. Run 'update install'.", r.version));
                    self.update = Some(r);
                }
                Msg::UpdateReady(p, v) => {
                    self.update_path = Some(p.clone());
                    self.notice(Status::Ok, format!("Update {v} downloaded and verified against the signed checksum."));
                    if self.install_after_download {
                        self.install_after_download = false;
                        self.ask_install(p);
                    }
                }
            }
        }
    }

    fn ask_install(&mut self, p: PathBuf) {
        let extra = if self.active_case.is_some() { " Your active case is saved locally and will not be affected." } else { "" };
        self.notice(Status::Warn, format!("Install now? PSYCHOSEC will close.{extra} (y/n)"));
        self.confirm = Some(Confirm::InstallUpdate(p));
    }

    // ---------- keyboard ----------
    pub fn on_key(&mut self, k: KeyEvent) {
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        if !matches!(k.code, KeyCode::Tab) {
            self.comp = None;
        }
        match (k.code, ctrl) {
            (KeyCode::Char('c'), true) => {
                if let Some(b) = self.busy.take() {
                    b.handle.abort();
                    self.notice(Status::Warn, format!("Cancelled: {}", b.label));
                } else if matches!(self.auth, AuthState::Waiting(_)) {
                    self.cancel_login();
                } else if !self.input.is_empty() {
                    self.input.clear();
                    self.cursor = 0;
                } else {
                    self.should_quit = true;
                }
            }
            (KeyCode::Char('d'), true) if self.input.is_empty() => self.should_quit = true,
            (KeyCode::Char('l'), true) => self.lines.clear(),
            (KeyCode::Char('u'), true) => {
                let rest: String = self.input.chars().skip(self.cursor).collect();
                self.input = rest;
                self.cursor = 0;
            }
            (KeyCode::Char('w'), true) => {
                let chars: Vec<char> = self.input.chars().collect();
                let mut i = self.cursor;
                while i > 0 && chars[i - 1] == ' ' { i -= 1; }
                while i > 0 && chars[i - 1] != ' ' { i -= 1; }
                self.input = chars[..i].iter().chain(chars[self.cursor..].iter()).collect();
                self.cursor = i;
            }
            (KeyCode::Char('a'), true) | (KeyCode::Home, _) => self.cursor = 0,
            (KeyCode::Char('e'), true) | (KeyCode::End, _) => self.cursor = self.input.chars().count(),
            (KeyCode::Esc, _) => {
                if matches!(self.auth, AuthState::Waiting(_)) && self.input.is_empty() {
                    self.cancel_login();
                } else {
                    self.input.clear();
                    self.cursor = 0;
                    if self.confirm.take().is_some() {
                        self.notice(Status::Dim, "Cancelled.");
                    }
                }
            }
            (KeyCode::Enter, _) => {
                let line = std::mem::take(&mut self.input);
                self.cursor = 0;
                self.hist_pos = None;
                self.submit(line.trim().to_string());
            }
            (KeyCode::Backspace, _) => {
                if self.cursor > 0 {
                    let idx = self.byte_idx(self.cursor - 1);
                    self.input.remove(idx);
                    self.cursor -= 1;
                }
            }
            (KeyCode::Delete, _) => {
                if self.cursor < self.input.chars().count() {
                    let idx = self.byte_idx(self.cursor);
                    self.input.remove(idx);
                }
            }
            (KeyCode::Left, _) => self.cursor = self.cursor.saturating_sub(1),
            (KeyCode::Right, _) => self.cursor = (self.cursor + 1).min(self.input.chars().count()),
            (KeyCode::Up, _) => self.history_step(true),
            (KeyCode::Down, _) => self.history_step(false),
            (KeyCode::PageUp, _) => self.scroll = (self.scroll + 10).min(self.max_scroll.get()),
            (KeyCode::PageDown, _) => self.scroll = self.scroll.saturating_sub(10),
            (KeyCode::Tab, _) => self.tab_complete(),
            (KeyCode::Char(c), false) => {
                let idx = self.byte_idx(self.cursor);
                self.input.insert(idx, c);
                self.cursor += 1;
            }
            _ => {}
        }
    }

    fn byte_idx(&self, char_idx: usize) -> usize {
        self.input.char_indices().nth(char_idx).map(|(i, _)| i).unwrap_or(self.input.len())
    }

    fn history_step(&mut self, up: bool) {
        if self.history.is_empty() {
            return;
        }
        let pos = match (self.hist_pos, up) {
            (None, true) => self.history.len() - 1,
            (Some(p), true) => p.saturating_sub(1),
            (Some(p), false) if p + 1 < self.history.len() => p + 1,
            (Some(_), false) => {
                self.hist_pos = None;
                self.input.clear();
                self.cursor = 0;
                return;
            }
            (None, false) => return,
        };
        self.hist_pos = Some(pos);
        self.input = self.history[pos].clone();
        self.cursor = self.input.chars().count();
    }

    fn tab_complete(&mut self) {
        let next = match self.comp.take() {
            Some((base, cands, i)) => {
                let n = (i + 1) % cands.len();
                self.comp = Some((base, cands.clone(), n));
                Some(cands[n])
            }
            None => {
                let c = commands::complete(self.input.trim_start());
                if c.is_empty() {
                    None
                } else {
                    self.comp = Some((self.input.clone(), c.clone(), 0));
                    Some(c[0])
                }
            }
        };
        if let Some(s) = next {
            self.input = s.to_string();
            self.cursor = self.input.chars().count();
        }
    }

    fn cancel_login(&mut self) {
        self.abort_bg();
        self.auth = AuthState::Locked("sign-in cancelled".into());
        self.notice(Status::Warn, "Sign-in cancelled. Type 'login' to try again.");
    }

    // ---------- command dispatch ----------
    fn submit(&mut self, line: String) {
        if let Some(c) = self.confirm.take() {
            self.push(OutLine::Cmd(line.clone()));
            if matches!(line.to_ascii_lowercase().as_str(), "y" | "yes") {
                self.do_confirm(c);
            } else {
                self.notice(Status::Dim, "Cancelled.");
            }
            return;
        }
        if line.is_empty() {
            return;
        }
        if self.history.last() != Some(&line) {
            self.history.push(line.clone());
        }
        self.push(OutLine::Cmd(line.clone()));
        let cmd = match commands::parse(&line) {
            Ok(c) => c,
            Err(e) => return self.notice(Status::Err, e),
        };
        let open_cmd = matches!(cmd, Command::Help(_) | Command::Exit | Command::Clear | Command::Login | Command::Status | Command::Settings(_) | Command::Modules);
        if !self.authorized() && !open_cmd {
            return self.notice(Status::Err, "Not authenticated. Sign in with Discord first ('login').");
        }
        let is_invest = matches!(cmd, Command::UsernameSearch(_) | Command::DomainScan(_) | Command::IpLookup(_) | Command::EmailCheck(_) | Command::WebInspect(_) | Command::FileMetadata(_) | Command::Update(_));
        if is_invest && self.busy.is_some() {
            return self.notice(Status::Warn, "Another operation is running. Wait for it or press Ctrl+C to cancel.");
        }
        self.run(cmd);
    }

    fn do_confirm(&mut self, c: Confirm) {
        match c {
            Confirm::DeleteCase(id, name) => match self.store.delete_case(id) {
                Ok(()) => {
                    if self.active_case.as_ref().map(|c| c.0) == Some(id) {
                        self.active_case = None;
                    }
                    self.notice(Status::Ok, format!("Deleted case #{id} '{name}'."));
                }
                Err(e) => self.notice(Status::Err, e),
            },
            Confirm::InstallUpdate(p) => match crate::update::launch_installer(&p) {
                Ok(()) => {
                    self.notice(Status::Ok, "Installer started. Closing.");
                    self.should_quit = true;
                }
                Err(e) => self.notice(Status::Err, e),
            },
        }
    }

    fn spawn_task<F>(&mut self, label: &str, fut: F)
    where
        F: Future<Output = Result<Output, String>> + Send + 'static,
    {
        let tx = self.tx.clone();
        let handle = self.rt.spawn(async move {
            match fut.await {
                Ok(o) => {
                    let _ = tx.send(Msg::Result(o));
                }
                Err(e) => {
                    let _ = tx.send(Msg::Notice(Status::Err, e));
                }
            }
            let _ = tx.send(Msg::Done);
        });
        self.busy = Some(Busy { label: label.to_string(), handle });
    }

    fn need_case(&mut self) -> Option<i64> {
        match &self.active_case {
            Some((id, _)) => Some(*id),
            None => {
                self.notice(Status::Err, "No active case. Use 'case create <name>' or 'case open <id>'.");
                None
            }
        }
    }

    fn run(&mut self, cmd: Command) {
        let t = self.settings.timeout_secs;
        match cmd {
            Command::Help(topic) => self.help(topic),
            Command::Modules => {
                self.blank_if_needed();
                self.push(OutLine::Title("Modules".into()));
                for (n, d) in osint::MODULES {
                    self.push(OutLine::Item(Status::Ok, n.to_string(), format!("{d}")));
                }
                self.push(OutLine::Blank);
            }
            Command::Status => self.status(),
            Command::Account => self.account(),
            Command::Settings(None) => {
                self.blank_if_needed();
                self.push(OutLine::Title("Settings".into()));
                self.push(OutLine::Kv("update_check".into(), self.settings.update_check.to_string()));
                self.push(OutLine::Kv("auto_download".into(), self.settings.auto_download.to_string()));
                self.push(OutLine::Kv("timeout_secs".into(), self.settings.timeout_secs.to_string()));
                self.push(OutLine::Kv("data folder".into(), config::data_dir().display().to_string()));
                self.push(OutLine::Text("Change with: settings set <key> <value>.  Optional API keys are read from ABUSEIPDB_API_KEY and HIBP_API_KEY.".into()));
                self.push(OutLine::Blank);
            }
            Command::Settings(Some((k, v))) => match self.settings.set(&k, &v) {
                Ok(()) => self.notice(Status::Ok, format!("{k} = {v}")),
                Err(e) => self.notice(Status::Err, e),
            },
            Command::Clear => self.lines.clear(),
            Command::Login => {
                if self.authorized() {
                    self.notice(Status::Info, "Already signed in. Use 'logout' first to switch accounts.");
                } else {
                    self.start_login();
                }
            }
            Command::Logout => {
                if let Some(tok) = self.token.take() {
                    let api = self.api.clone();
                    self.rt.spawn(async move {
                        let _ = api.logout(&tok).await;
                    });
                }
                crate::creds::delete(crate::creds::SESSION_TOKEN);
                self.abort_bg();
                self.auth = AuthState::Locked("signed out".into());
                self.active_case = None;
                self.notice(Status::Ok, "Signed out. Local cases remain on this computer.");
            }
            Command::Exit => self.should_quit = true,
            Command::Update(install) => self.update_cmd(install),
            Command::UsernameSearch(u) => self.spawn_task(&format!("searching username {u}"), async move { osint::username::search(&u, t).await }),
            Command::DomainScan(d) => self.spawn_task(&format!("scanning {d}"), async move { osint::domain::scan(&d, t).await }),
            Command::IpLookup(i) => self.spawn_task(&format!("looking up {i}"), async move { osint::ip::lookup(&i, t).await }),
            Command::EmailCheck(e) => self.spawn_task("checking email", async move { osint::email::check(&e, t).await }),
            Command::WebInspect(u) => self.spawn_task(&format!("inspecting {u}"), async move { osint::web::inspect(&u, t).await }),
            Command::FileMetadata(p) => self.spawn_task("reading file", async move { osint::file::metadata(&p).await }),
            Command::CaseCreate(n) => match self.store.create_case(&n) {
                Ok(id) => {
                    self.active_case = Some((id, n.clone()));
                    self.notice(Status::Ok, format!("Created and opened case #{id} '{n}'."));
                }
                Err(e) => self.notice(Status::Err, e),
            },
            Command::CaseOpen(id) => match self.store.get_case(id) {
                Ok(c) => {
                    self.active_case = Some((c.id, c.name.clone()));
                    let n = self.store.findings(id).map(|f| f.len()).unwrap_or(0);
                    self.notice(Status::Ok, format!("Opened case #{} '{}' ({} findings, {} targets, {} notes).", c.id, c.name, n, c.targets.len(), c.notes.len()));
                }
                Err(e) => self.notice(Status::Err, e),
            },
            Command::CaseList => match self.store.list_cases() {
                Ok(v) if v.is_empty() => self.notice(Status::Info, "No cases yet. Create one with 'case create <name>'."),
                Ok(v) => {
                    self.blank_if_needed();
                    self.push(OutLine::Title("Cases".into()));
                    for (id, name, upd, n) in v {
                        let mark = if self.active_case.as_ref().map(|c| c.0) == Some(id) { "active" } else { "" };
                        self.push(OutLine::Item(if mark.is_empty() { Status::Dim } else { Status::Ok }, format!("#{id}"), format!("{name}  ({n} findings, updated {}) {mark}", upd.get(..10).unwrap_or(&upd))));
                    }
                    self.push(OutLine::Blank);
                }
                Err(e) => self.notice(Status::Err, e),
            },
            Command::CaseFindings => {
                let Some(id) = self.need_case() else { return };
                match (self.store.get_case(id), self.store.findings(id)) {
                    (Ok(c), Ok(f)) => {
                        self.blank_if_needed();
                        self.push(OutLine::Title(format!("Case #{} {}", c.id, c.name)));
                        for t in &c.targets { self.push(OutLine::Kv("target".into(), t.clone())); }
                        for n in &c.notes { self.push(OutLine::Kv("note".into(), n.clone())); }
                        if f.is_empty() { self.push(OutLine::Text("No findings yet.".into())); }
                        for x in f {
                            self.push(OutLine::Item(if x.verified { Status::Ok } else { Status::Warn }, format!("#{}", x.id), format!("{} [{}] {}{}", x.title, if x.verified { "verified" } else { "unverified" }, x.collected_at, if x.note.is_empty() { String::new() } else { format!(" - {}", x.note) })));
                        }
                        self.push(OutLine::Blank);
                    }
                    (Err(e), _) | (_, Err(e)) => self.notice(Status::Err, e),
                }
            }
            Command::CaseAdd(note) => {
                let Some(id) = self.need_case() else { return };
                match self.last.clone() {
                    None => self.notice(Status::Err, "Nothing to add. Run an investigation command first."),
                    Some(o) => match self.store.add_finding(id, &o, note.as_deref().unwrap_or("")) {
                        Ok(fid) => self.notice(Status::Ok, format!("Saved as finding #{fid} (unverified). Mark it with 'case verify {fid}'.")),
                        Err(e) => self.notice(Status::Err, e),
                    },
                }
            }
            Command::CaseDelete(id) => match self.store.get_case(id) {
                Ok(c) => {
                    self.notice(Status::Warn, format!("Permanently delete case #{id} '{}' and all its findings? (y/n)", c.name));
                    self.confirm = Some(Confirm::DeleteCase(id, c.name));
                }
                Err(e) => self.notice(Status::Err, e),
            },
            Command::CaseRename(n) => {
                let Some(id) = self.need_case() else { return };
                match self.store.rename_case(id, &n) {
                    Ok(()) => {
                        self.active_case = Some((id, n.clone()));
                        self.notice(Status::Ok, format!("Renamed to '{n}'."));
                    }
                    Err(e) => self.notice(Status::Err, e),
                }
            }
            Command::CaseNote(n) => {
                let Some(id) = self.need_case() else { return };
                match self.store.add_note(id, &n) {
                    Ok(()) => self.notice(Status::Ok, "Note added."),
                    Err(e) => self.notice(Status::Err, e),
                }
            }
            Command::CaseTarget(n) => {
                let Some(id) = self.need_case() else { return };
                match self.store.add_target(id, &n) {
                    Ok(()) => self.notice(Status::Ok, format!("Target added: {n}")),
                    Err(e) => self.notice(Status::Err, e),
                }
            }
            Command::CaseVerify(fid, v) => {
                let Some(id) = self.need_case() else { return };
                match self.store.set_verified(id, fid, v) {
                    Ok(()) => self.notice(Status::Ok, format!("Finding #{fid} marked {}.", if v { "verified" } else { "unverified" })),
                    Err(e) => self.notice(Status::Err, e),
                }
            }
            Command::CaseClose => {
                self.active_case = None;
                self.notice(Status::Ok, "Case closed.");
            }
            Command::ReportExport(fmt, out) => {
                let Some(id) = self.need_case() else { return };
                match (self.store.get_case(id), self.store.findings(id)) {
                    (Ok(c), Ok(f)) => match report::export(&c, &f, &fmt, out.as_deref()) {
                        Ok(p) => self.notice(Status::Ok, format!("Report written to {}", p.display())),
                        Err(e) => self.notice(Status::Err, e),
                    },
                    (Err(e), _) | (_, Err(e)) => self.notice(Status::Err, e),
                }
            }
        }
    }

    fn update_cmd(&mut self, install: bool) {
        if !install {
            self.notice(Status::Info, "Checking for updates...");
            self.spawn_update_check(true);
            return;
        }
        let Some(r) = self.update.clone() else {
            return self.notice(Status::Info, "No update known yet. Run 'update' to check first.");
        };
        if let Some(p) = self.update_path.clone() {
            return self.ask_install(p);
        }
        self.install_after_download = true;
        self.notice(Status::Info, format!("Downloading {} and verifying checksum...", r.version));
        let tx = self.tx.clone();
        let handle = self.rt.spawn(async move {
            match crate::update::download(&r, |_, _| {}).await {
                Ok(p) => {
                    let _ = tx.send(Msg::UpdateReady(p, r.version.clone()));
                }
                Err(e) => {
                    let _ = tx.send(Msg::Notice(Status::Err, format!("update failed: {e}")));
                }
            }
            let _ = tx.send(Msg::Done);
        });
        self.busy = Some(Busy { label: "downloading update".into(), handle });
    }

    fn help(&mut self, topic: Option<String>) {
        self.blank_if_needed();
        let t = topic.map(|t| t.to_ascii_lowercase());
        self.push(OutLine::Title(match &t { Some(t) => format!("Help: {t}"), None => "Commands".into() }));
        let mut last = "";
        let mut any = false;
        for s in commands::SPECS {
            if let Some(t) = &t {
                if !s.name.starts_with(t.as_str()) && !s.group.eq_ignore_ascii_case(t) {
                    continue;
                }
            }
            any = true;
            if s.group != last {
                self.push(OutLine::Heading(s.group.into()));
                last = s.group;
            }
            self.push(OutLine::Kv(s.usage.into(), s.desc.into()));
        }
        if !any {
            self.push(OutLine::Text("No such topic. Try: help case, help report, help investigate.".into()));
        } else if t.is_none() {
            self.push(OutLine::Blank);
            self.push(OutLine::Text("Tab completes, Up/Down recalls history, PgUp/PgDn scrolls, Ctrl+C cancels a running task, Ctrl+L clears.".into()));
        }
        self.push(OutLine::Blank);
    }

    fn status(&mut self) {
        self.blank_if_needed();
        self.push(OutLine::Title(format!("{APP_NAME} status")));
        self.push(OutLine::Kv("Version".into(), VERSION.into()));
        self.push(OutLine::Kv("Service".into(), self.api.base().into()));
        let a = match &self.auth {
            AuthState::Authorized(i) => format!("authenticated as {} (session expires {})", i.user.username, i.expires_at.get(..16).unwrap_or(&i.expires_at)),
            AuthState::Checking => "checking".into(),
            AuthState::Waiting(_) => "waiting for Discord authorization".into(),
            AuthState::Locked(m) => format!("not authenticated ({m})"),
        };
        self.push(OutLine::Kv("Session".into(), a));
        self.push(OutLine::Kv("Active case".into(), self.active_case.as_ref().map(|c| format!("#{} {}", c.0, c.1)).unwrap_or("none".into())));
        let u = match (&self.update, &self.update_path) {
            (Some(r), Some(_)) => format!("{} downloaded, run 'update install'", r.version),
            (Some(r), None) => format!("{} available", r.version),
            _ => "none known".into(),
        };
        self.push(OutLine::Kv("Update".into(), u));
        self.push(OutLine::Kv("Local data".into(), config::data_dir().display().to_string()));
        self.push(OutLine::Blank);
    }

    fn account(&mut self) {
        let AuthState::Authorized(i) = self.auth.clone() else { return };
        self.blank_if_needed();
        self.push(OutLine::Title("Account".into()));
        self.push(OutLine::Kv("Username".into(), i.user.username.clone()));
        if let Some(d) = &i.user.display_name { self.push(OutLine::Kv("Display name".into(), d.clone())); }
        self.push(OutLine::Kv("Discord ID".into(), i.user.discord_id.clone()));
        self.push(OutLine::Kv("Role check".into(), if i.role_verified { "verified" } else { "not required" }.into()));
        self.push(OutLine::Kv("Session expires".into(), i.expires_at.clone()));
        self.push(OutLine::Text("The service records your Discord identity, sign-in IP addresses and session activity for access control and security. Investigation data never leaves this computer. See https://psychosec.org/privacy".into()));
        self.push(OutLine::Blank);
    }
}

async fn bootstrap(api: Api, tx: UnboundedSender<Msg>) {
    if let Some(tok) = crate::creds::get(crate::creds::SESSION_TOKEN) {
        match api.session(&tok).await {
            Ok(info) => {
                let _ = tx.send(Msg::Authed { info, token: tok });
                return;
            }
            Err(ApiError::Forbidden(m)) => {
                crate::creds::delete(crate::creds::SESSION_TOKEN);
                let _ = tx.send(Msg::Locked(m));
                return;
            }
            Err(ApiError::Unauthorized(_)) => crate::creds::delete(crate::creds::SESSION_TOKEN),
            Err(e) => {
                let _ = tx.send(Msg::Locked(e.to_string()));
                return;
            }
        }
    }
    browser_login(api, tx).await;
}

async fn browser_login(api: Api, tx: UnboundedSender<Msg>) {
    let p = crate::auth::begin();
    let url = api.login_url(&p.login_id, &p.challenge);
    let _ = webbrowser::open(&url);
    let _ = tx.send(Msg::LoginUrl(url));
    let mut failures = 0;
    for _ in 0..150 {
        tokio::time::sleep(Duration::from_secs(2)).await;
        match api.poll_login(&p.login_id, &p.verifier).await {
            Ok(r) => {
                failures = 0;
                match r.status.as_str() {
                    "pending" => {}
                    "approved" => match (r.token, r.session) {
                        (Some(token), Some(info)) => {
                            if let Err(e) = crate::creds::set(crate::creds::SESSION_TOKEN, &token) {
                                let _ = tx.send(Msg::Notice(Status::Warn, format!("could not save session securely: {e}")));
                            }
                            let _ = tx.send(Msg::Authed { info, token });
                            return;
                        }
                        _ => {
                            let _ = tx.send(Msg::Locked("server sent an incomplete approval".into()));
                            return;
                        }
                    },
                    "denied" => {
                        let _ = tx.send(Msg::Locked(r.message.unwrap_or_else(|| "access denied".into())));
                        return;
                    }
                    _ => {
                        let _ = tx.send(Msg::Locked(r.message.unwrap_or_else(|| "the sign-in request expired".into())));
                        return;
                    }
                }
            }
            Err(ApiError::Network(m)) => {
                failures += 1;
                if failures > 5 {
                    let _ = tx.send(Msg::Locked(format!("cannot reach the PSYCHOSEC service: {m}")));
                    return;
                }
            }
            Err(e) => {
                let _ = tx.send(Msg::Locked(e.to_string()));
                return;
            }
        }
    }
    let _ = tx.send(Msg::Locked("sign-in timed out".into()));
}
