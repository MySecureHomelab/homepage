use crate::{State, widgets};
use actix_web::web;
use serde::Serialize;
use std::{
    collections::HashSet,
    time::{Duration, Instant},
};
use sysinfo::{Disks, System};
use tokio::{net::TcpStream, task::JoinSet, time};

const TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone, Serialize)]
pub struct Status {
    pub up: bool,
    /// Response time; `None` for checks that don't measure one (`docker://`).
    pub ms: Option<u64>,
}

#[derive(Clone, Serialize)]
pub struct Resources {
    cpu: f32,
    mem_used: u64,
    mem_total: u64,
    disk_used: u64,
    disk_total: u64,
    uptime: u64,
}

async fn check(client: reqwest::Client, target: String) -> (String, Status) {
    let start = Instant::now();
    let up = match target.strip_prefix("tcp://") {
        Some(addr) => matches!(
            time::timeout(TIMEOUT, TcpStream::connect(addr)).await,
            Ok(Ok(_))
        ),
        // A 401/404 still means the service is there; only 5xx or no answer is down.
        None => client
            .get(&target)
            .send()
            .await
            .is_ok_and(|r| !r.status().is_server_error()),
    };
    let ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
    (target, Status { up, ms: Some(ms) })
}

pub fn widget_key(group: &str, service: &str) -> String {
    format!("{group}/{service}")
}

enum Outcome {
    Check(String, Status),
    Widget(String, widgets::Stats),
}

pub async fn run_checks(state: web::Data<State>) {
    let client = reqwest::Client::builder()
        .timeout(TIMEOUT)
        .build()
        .expect("http client");
    let fetcher = std::sync::Arc::new(widgets::Fetcher::new());
    loop {
        let (cfg, _) = state.config();
        // docker:// states come straight from discovery, see `api_state`.
        let groups = state.groups(&cfg);
        let targets: HashSet<String> = groups
            .iter()
            .flat_map(|g| &g.services)
            .filter_map(|s| s.check.clone())
            .filter(|c| !c.starts_with("docker://"))
            .collect();
        // Forget services removed from the config, then publish each result as it lands
        // so one slow host doesn't hold back the others.
        state
            .status
            .write()
            .unwrap()
            .retain(|t, _| targets.contains(t));
        let mut set = JoinSet::new();
        for t in targets {
            let client = client.clone();
            set.spawn(async move {
                let (t, s) = check(client, t).await;
                Outcome::Check(t, s)
            });
        }
        let mut keys = HashSet::new();
        for g in &groups {
            for s in &g.services {
                let Some(w) = &s.widget else { continue };
                let Some(def) = widgets::lookup(&cfg.widget_types, &w.kind).cloned() else {
                    continue;
                };
                let key = widget_key(&g.name, &s.name);
                keys.insert(key.clone());
                let (fetcher, w, url) = (fetcher.clone(), w.clone(), s.url.clone());
                set.spawn(async move {
                    let result = fetcher.fetch(&key, &def, &w, url.as_deref()).await;
                    Outcome::Widget(key, result)
                });
            }
        }
        state
            .widgets
            .write()
            .unwrap()
            .retain(|k, _| keys.contains(k));
        while let Some(r) = set.join_next().await {
            match r {
                Ok(Outcome::Check(t, s)) => drop(state.status.write().unwrap().insert(t, s)),
                Ok(Outcome::Widget(k, s)) => drop(state.widgets.write().unwrap().insert(k, s)),
                Err(_) => {}
            }
        }
        // Sleep until the next tick, or until a config reload asks for a fresh round.
        let _ = time::timeout(
            Duration::from_secs(cfg.check_interval),
            state.targets_changed.notified(),
        )
        .await;
    }
}

pub async fn run_resources(state: web::Data<State>) {
    let mut sys = System::new();
    let mut disks = Disks::new_with_refreshed_list();
    loop {
        sys.refresh_cpu_usage();
        sys.refresh_memory();
        disks.refresh(true);
        // ponytail: root filesystem only, per-mount list when someone needs it
        let root = disks
            .iter()
            .find(|d| d.mount_point() == std::path::Path::new("/"));
        let (disk_total, disk_avail) =
            root.map_or((0, 0), |d| (d.total_space(), d.available_space()));
        *state.resources.write().unwrap() = Some(Resources {
            cpu: sys.global_cpu_usage(),
            mem_used: sys.used_memory(),
            mem_total: sys.total_memory(),
            disk_used: disk_total - disk_avail,
            disk_total,
            uptime: System::uptime(),
        });
        time::sleep(Duration::from_secs(5)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::TcpListener,
        thread,
    };

    /// One-shot HTTP server answering every connection with `status`.
    fn http_server(status: u16) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        thread::spawn(move || {
            for mut conn in listener.incoming().flatten() {
                let _ = conn.read(&mut [0; 1024]);
                let _ = write!(
                    conn,
                    "HTTP/1.1 {status} X\r\ncontent-length: 0\r\nconnection: close\r\n\r\n"
                );
            }
        });
        format!("http://{addr}/")
    }

    fn closed_port() -> String {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        l.local_addr().unwrap().to_string() // listener dropped here, port now closed
    }

    async fn up(target: String) -> bool {
        check(reqwest::Client::new(), target).await.1.up
    }

    #[actix_web::test]
    async fn checks() {
        assert!(up(http_server(200)).await);
        assert!(
            up(http_server(401)).await,
            "auth wall still means the service is there"
        );
        assert!(!up(http_server(503)).await);
        assert!(!up(format!("http://{}/", closed_port())).await);

        let open = TcpListener::bind("127.0.0.1:0").unwrap();
        assert!(up(format!("tcp://{}", open.local_addr().unwrap())).await);
        assert!(!up(format!("tcp://{}", closed_port())).await);
    }
}
