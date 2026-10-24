mod config;
mod monitor;

use actix_web::{
    App, HttpResponse, HttpServer, Responder, get, http::header, middleware::DefaultHeaders, web,
};
use askama::Template;
use config::{Config, Widget};
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
    /// Wakes the status checker so services added on reload are checked right away.
    pub reloaded: Notify,
    pub status: RwLock<HashMap<String, Status>>,
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
                    self.reloaded.notify_one();
                }
                Err(e) => {
                    eprintln!("config reload failed, keeping previous:\n{e}");
                    l.error = Some(e);
                }
            }
        }
        (l.cfg.clone(), l.error.clone())
    }
}

#[derive(Template)]
#[template(path = "index.html")]
struct Index<'a> {
    cfg: &'a Config,
    error: Option<String>,
}

#[get("/")]
async fn index(state: web::Data<State>) -> impl Responder {
    let (cfg, error) = state.config();
    match (Index { cfg: &cfg, error }).render() {
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
    resources: Option<Resources>,
}

#[get("/api/state")]
async fn api_state(state: web::Data<State>) -> impl Responder {
    web::Json(Snapshot {
        status: state.status.read().unwrap().clone(),
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
        reloaded: Notify::new(),
        status: RwLock::default(),
        resources: RwLock::default(),
    });
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
