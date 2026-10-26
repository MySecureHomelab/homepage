mod config;
mod discovery;
mod monitor;
mod widgets;

use actix_web::{
    App, HttpResponse, HttpServer, Responder, get, http::header, middleware::DefaultHeaders, web,
};
use askama::Template;
use config::{Config, Group, Widget};
use monitor::{Resources, Status};
use serde::Serialize;
use std::{
    collections::HashMap,
    fs,
    hash::{DefaultHasher, Hash, Hasher},
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
    time::{Duration, SystemTime},
};
use tokio::sync::Notify;

const CSS: &str = include_str!("../assets/app.css");
const JS: &str = include_str!("../assets/app.js");

// Everything is served from here; only icons may come from elsewhere.
const CSP: &str = "default-src 'self'; img-src * data:; base-uri 'none'; form-action 'self'; frame-ancestors 'none'";

pub struct State {
    path: PathBuf,
    loaded: RwLock<Loaded>,
    /// Wakes discovery when the config changed.
    pub config_changed: Notify,
    /// Wakes the status checker so new services are checked right away.
    pub targets_changed: Notify,
    pub discovered: RwLock<Vec<discovery::Found>>,
    /// `docker://host/container` → running and not unhealthy.
    pub containers: RwLock<HashMap<String, bool>>,
    pub status: RwLock<HashMap<String, Status>>,
    /// Widget stats by `group/service`.
    pub widgets: RwLock<HashMap<String, widgets::Stats>>,
    pub resources: RwLock<Option<Resources>>,
    /// Last downloaded remote background: its URL and the image.
    background: RwLock<Option<(String, Image)>>,
}

/// Content type and bytes.
type Image = (String, web::Bytes);

const MAX_BACKGROUND: usize = 20 << 20;

fn is_url(src: &str) -> bool {
    src.starts_with("http://") || src.starts_with("https://")
}

struct Loaded {
    cfg: Arc<Config>,
    mtime: Option<SystemTime>,
    error: Option<String>,
    loaded_once: bool,
}

impl Loaded {
    fn error(&self) -> Option<ConfigError> {
        self.error.clone().map(|message| ConfigError {
            message,
            has_previous: self.loaded_once,
        })
    }
}

/// Problems that don't stop the dashboard, logged on every (re)load.
fn report(cfg: &Config) {
    for issue in &cfg.issues {
        eprintln!("config: {issue}");
    }
}

/// Why the page may not show the config as written.
#[derive(Clone)]
pub struct ConfigError {
    pub message: String,
    /// False until a config has loaded: the page is then empty rather than stale.
    pub has_previous: bool,
}

fn mtime(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).and_then(|m| m.modified()).ok()
}

impl State {
    /// Current config, reloaded when the file changed. A broken edit keeps the
    /// last good config and surfaces the error instead of taking the page down.
    pub(crate) fn config(&self) -> (Arc<Config>, Option<ConfigError>) {
        let now = mtime(&self.path);
        {
            let l = self.loaded.read().unwrap();
            if l.mtime == now {
                return (l.cfg.clone(), l.error());
            }
        }
        let mut l = self.loaded.write().unwrap();
        if l.mtime != now {
            l.mtime = now;
            match config::load(&self.path) {
                Ok(cfg) => {
                    eprintln!("config reloaded");
                    report(&cfg);
                    l.loaded_once = true;
                    l.cfg = Arc::new(cfg);
                    l.error = None;
                    self.config_changed.notify_one();
                    self.targets_changed.notify_one();
                }
                Err(e) => {
                    eprintln!("config reload failed, keeping previous:\n{e}");
                    l.error = Some(e);
                }
            }
        }
        (l.cfg.clone(), l.error())
    }

    /// A local background path, relative to the config file's directory.
    fn background_path(&self, src: &str) -> PathBuf {
        self.path.parent().unwrap_or(Path::new(".")).join(src)
    }

    async fn background_image(&self, src: &str) -> Result<Image, String> {
        if !is_url(src) {
            let path = self.background_path(src);
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or_default();
            let kind = match ext.to_ascii_lowercase().as_str() {
                "jpg" | "jpeg" => "image/jpeg".into(),
                e @ ("png" | "webp" | "gif" | "avif") => format!("image/{e}"),
                "svg" => "image/svg+xml".into(),
                _ => {
                    return Err(format!(
                        "{}: expected a jpg, png, webp, gif, avif or svg file",
                        path.display()
                    ));
                }
            };
            let read = path.clone();
            let bytes = web::block(move || fs::read(read))
                .await
                .map_err(|e| e.to_string())?
                .map_err(|e| format!("{}: {e}", path.display()))?;
            return Ok((kind, bytes.into()));
        }
        if let Some((url, img)) = &*self.background.read().unwrap()
            && url == src
        {
            return Ok(img.clone());
        }
        let res = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map(|c| c.get(src))
            .map_err(|e| e.to_string())?
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|e| format!("{src}: {e}"))?;
        let kind = res
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .filter(|t| t.starts_with("image/"))
            .ok_or_else(|| format!("{src}: not an image"))?
            .to_string();
        if res
            .content_length()
            .is_some_and(|n| n as usize > MAX_BACKGROUND)
        {
            return Err(format!("{src}: larger than 20 MB"));
        }
        let bytes = res.bytes().await.map_err(|e| format!("{src}: {e}"))?;
        if bytes.len() > MAX_BACKGROUND {
            return Err(format!("{src}: larger than 20 MB"));
        }
        let img = (kind, bytes);
        *self.background.write().unwrap() = Some((src.to_string(), img.clone()));
        Ok(img)
    }

    /// Config groups with discovered services merged in.
    pub(crate) fn groups(&self, cfg: &Config) -> Vec<Group> {
        discovery::merge(&cfg.groups, &self.discovered.read().unwrap())
    }
}

#[derive(Template)]
#[template(path = "index.html")]
struct Index<'a> {
    cfg: &'a Config,
    groups: Vec<Group>,
    error: Option<ConfigError>,
    widgets: HashMap<String, widgets::Stats>,
}

impl Index<'_> {
    fn stats(&self, group: &str, service: &str) -> Option<&widgets::Stats> {
        self.widgets.get(&monitor::widget_key(group, service))
    }
}

#[get("/")]
async fn index(state: web::Data<State>) -> impl Responder {
    let (cfg, error) = state.config();
    let groups = state.groups(&cfg);
    let widgets = state.widgets.read().unwrap().clone();
    let page = Index {
        cfg: &cfg,
        groups,
        error,
        widgets,
    };
    match page.render() {
        Ok(html) => HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(html),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[get("/app.css")]
async fn css(state: web::Data<State>) -> impl Responder {
    // The accent is validated as #rrggbb, so it is safe to splice in.
    let cfg = state.config().0;
    let accent = &cfg.accent;
    let margin = cfg
        .side_margin
        .map(|px| {
            format!("main {{ max-width: none; padding-left: {px}px; padding-right: {px}px; }}\n")
        })
        .unwrap_or_default();
    // The image URL changes with the source (and a local file's mtime), so the browser can cache it.
    let bg = cfg
        .background
        .as_ref()
        .map(|b| {
            let mut h = DefaultHasher::new();
            b.image.hash(&mut h);
            if !is_url(&b.image) {
                mtime(&state.background_path(&b.image)).hash(&mut h);
            }
            format!(
                "body::before {{ content: \"\"; position: fixed; inset: -{edge}px; z-index: -1; background: linear-gradient({veil}, {veil}), center / cover no-repeat url(\"background?v={v:x}\"); filter: blur({blur}px); }}\n",
                veil = format_args!("color-mix(in srgb, var(--bg) {}%, transparent)", b.dim),
                edge = b.blur * 2,
                v = h.finish(),
                blur = b.blur,
            )
        })
        .unwrap_or_default();
    HttpResponse::Ok()
        .content_type("text/css; charset=utf-8")
        .insert_header((header::CACHE_CONTROL, "no-cache"))
        .body(format!(
            ":root {{ --accent: {accent}; }}\n{CSS}{margin}{bg}"
        ))
}

#[get("/background")]
async fn background(state: web::Data<State>) -> impl Responder {
    let Some(src) = state
        .config()
        .0
        .background
        .as_ref()
        .map(|b| b.image.clone())
    else {
        return HttpResponse::NotFound().finish();
    };
    match state.background_image(&src).await {
        Ok((kind, bytes)) => HttpResponse::Ok()
            .content_type(kind)
            .insert_header((header::CACHE_CONTROL, "max-age=86400"))
            // An SVG opened directly must not run scripts.
            .insert_header((header::CONTENT_SECURITY_POLICY, "sandbox"))
            .body(bytes),
        Err(e) => {
            eprintln!("background: {e}");
            HttpResponse::BadGateway().body(e)
        }
    }
}

#[get("/app.js")]
async fn js() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/javascript; charset=utf-8")
        .insert_header((header::CACHE_CONTROL, "no-cache"))
        .body(JS)
}

#[derive(Serialize)]
struct Snapshot {
    status: HashMap<String, Status>,
    widgets: HashMap<String, widgets::Stats>,
    resources: Option<Resources>,
}

#[get("/api/state")]
async fn api_state(state: web::Data<State>) -> impl Responder {
    let mut status = state.status.read().unwrap().clone();
    // A container that is missing, or on a host that stopped answering, counts as down.
    let containers = state.containers.read().unwrap();
    for g in state.groups(&state.config().0) {
        for target in g.services.into_iter().filter_map(|s| s.check) {
            if target.starts_with("docker://") {
                let up = containers.get(&target).copied().unwrap_or(false);
                status.insert(target, Status { up, ms: None });
            }
        }
    }
    web::Json(Snapshot {
        status,
        widgets: state.widgets.read().unwrap().clone(),
        resources: state.resources.read().unwrap().clone(),
    })
}

#[get("/healthz")]
async fn healthz() -> &'static str {
    "ok"
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let path =
        PathBuf::from(std::env::var("HOMEPAGE_CONFIG").unwrap_or_else(|_| "config.yaml".into()));
    let loaded = config::load(&path);
    if std::env::args().nth(1).as_deref() == Some("check") {
        // Strict: any problem fails, so CI catches what the server would only report.
        match loaded {
            Ok(cfg) if cfg.issues.is_empty() => println!("{}: OK", path.display()),
            Ok(cfg) => {
                for issue in &cfg.issues {
                    eprintln!("{}: {issue}", path.display());
                }
                std::process::exit(1);
            }
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
        return Ok(());
    }
    // A broken or missing config never stops the server: the page shows the error and the
    // file is reloaded as soon as it changes.
    let (cfg, error) = match loaded {
        Ok(cfg) => {
            report(&cfg);
            (cfg, None)
        }
        Err(e) => {
            eprintln!("config error, starting with an empty page:\n{e}");
            (Config::empty(), Some(e))
        }
    };
    let loaded_once = error.is_none();

    let state = web::Data::new(State {
        loaded: RwLock::new(Loaded {
            cfg: Arc::new(cfg),
            mtime: mtime(&path),
            error,
            loaded_once,
        }),
        path,
        config_changed: Notify::new(),
        targets_changed: Notify::new(),
        discovered: RwLock::default(),
        containers: RwLock::default(),
        status: RwLock::default(),
        widgets: RwLock::default(),
        resources: RwLock::default(),
        background: RwLock::default(),
    });
    actix_web::rt::spawn(discovery::run(state.clone()));
    actix_web::rt::spawn(monitor::run_checks(state.clone()));
    actix_web::rt::spawn(monitor::run_resources(state.clone()));

    let addr = std::env::var("HOMEPAGE_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".into());
    eprintln!("listening on http://{addr}");
    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .wrap(
                DefaultHeaders::new()
                    .add((header::CONTENT_SECURITY_POLICY, CSP))
                    .add((header::X_CONTENT_TYPE_OPTIONS, "nosniff"))
                    .add((header::REFERRER_POLICY, "no-referrer")),
            )
            .service(index)
            .service(css)
            .service(js)
            .service(background)
            .service(api_state)
            .service(healthz)
    })
    .bind(addr)?
    .run()
    .await
}
