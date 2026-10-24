use crate::State;
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
    up: bool,
    ms: u64,
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
    (target, Status { up, ms })
}

pub async fn run_checks(state: web::Data<State>) {
    let client = reqwest::Client::builder()
        .timeout(TIMEOUT)
        .build()
        .expect("http client");
    loop {
        let (cfg, _) = state.config();
        let targets: HashSet<String> = cfg.checks().map(String::from).collect();
        // Forget services removed from the config, then publish each result as it lands
        // so one slow host doesn't hold back the others.
        state
            .status
            .write()
            .unwrap()
            .retain(|t, _| targets.contains(t));
        let mut set = JoinSet::new();
        for t in targets {
            set.spawn(check(client.clone(), t));
        }
        while let Some(r) = set.join_next().await {
            if let Ok((t, s)) = r {
                state.status.write().unwrap().insert(t, s);
            }
        }
        // Sleep until the next tick, or until a config reload asks for a fresh round.
        let _ = time::timeout(
            Duration::from_secs(cfg.check_interval),
            state.reloaded.notified(),
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
