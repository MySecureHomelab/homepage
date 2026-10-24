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
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
    time::SystemTime,
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
}

struct Loaded {
    cfg: Arc<Config>,
    mtime: Option<SystemTime>,
    error: Option<String>,
}

fn mtime(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).and_then(|m| m.modified()).ok()
}

impl State {
    /// Current config, reloaded when the file changed. A broken edit keeps the
    /// last good config and surfaces the error instead of taking the page down.
    pub(crate) fn config(&self) -> (Arc<Config>, Option<String>) {
        let now = mtime(&self.path);
        {
            let l = self.loaded.read().unwrap();
            if l.mtime == now {
                return (l.cfg.clone(), l.error.clone());
            }
        }
        let mut l = self.loaded.write().unwrap();
        if l.mtime != now {
            l.mtime = now;
            match config::load(&self.path) {
                Ok(cfg) => {
                    eprintln!("config reloaded");
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
        (l.cfg.clone(), l.error.clone())
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
    error: Option<String>,
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
    let accent = state.config().0.accent.clone();
    HttpResponse::Ok()
        .content_type("text/css; charset=utf-8")
        .insert_header((header::CACHE_CONTROL, "no-cache"))
        .body(format!(":root {{ --accent: {accent}; }}\n{CSS}"))
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
    let cfg = config::load(&path).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(1)
    });
    if std::env::args().nth(1).as_deref() == Some("check") {
        println!("{}: OK", path.display());
        return Ok(());
    }

    let state = web::Data::new(State {
        loaded: RwLock::new(Loaded {
            cfg: Arc::new(cfg),
            mtime: mtime(&path),
            error: None,
        }),
        path,
        config_changed: Notify::new(),
        targets_changed: Notify::new(),
        discovered: RwLock::default(),
        containers: RwLock::default(),
        status: RwLock::default(),
        widgets: RwLock::default(),
        resources: RwLock::default(),
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
            .service(api_state)
            .service(healthz)
    })
    .bind(addr)?
    .run()
    .await
}
