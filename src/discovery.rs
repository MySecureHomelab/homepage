//! Service discovery from Docker labels and Kubernetes Ingress annotations.
//! Label and annotation names match gethomepage.dev, so existing setups carry over.

use crate::{
    State,
    config::{Config, Group, Service},
    widgets::WidgetConfig,
};
use actix_web::web;
use bollard::{
    API_DEFAULT_VERSION, Docker, models::ContainerSummaryStateEnum,
    query_parameters::ListContainersOptions,
};
use k8s_openapi::api::networking::v1::Ingress;
use kube::api::{Api, ListParams};
use std::{
    collections::{BTreeMap, HashMap},
    time::Duration,
};
use tokio::time;

// ponytail: polling, switch to Docker events / kube watchers if 15s feels slow
const INTERVAL: Duration = Duration::from_secs(15);

/// A discovered service and the group it goes in.
pub type Found = (String, Service);

/// `<prefix>type`, `<prefix>url`, `<prefix>fields` (JSON list or comma-separated) and any
/// other `<prefix><option>` (key, username…) as a widget config.
fn widget_from<'a>(
    pairs: impl Iterator<Item = (&'a String, &'a String)>,
    prefix: &str,
) -> Option<WidgetConfig> {
    let mut options: BTreeMap<String, String> = pairs
        .filter_map(|(k, v)| Some((k.strip_prefix(prefix)?.to_string(), v.clone())))
        .collect();
    let kind = options.remove("type")?;
    let fields = options.remove("fields").map(|f| {
        serde_json::from_str(&f)
            .unwrap_or_else(|_| f.split(',').map(|s| s.trim().to_string()).collect())
    });
    Some(WidgetConfig {
        kind,
        url: options.remove("url"),
        fields,
        options,
        ..Default::default()
    })
}

/// Docker labels: `homepage.name` opts a container in; `homepage.group`, `homepage.url`
/// (or `homepage.href`), `homepage.icon`, `homepage.description`, `homepage.check` are optional.
/// Without `homepage.check`, the service is up when its container is running.
/// `homepage.widget.type`, `homepage.widget.key`… add a widget.
pub fn from_labels(labels: &HashMap<String, String>, container_check: &str) -> Option<Found> {
    let get = |k: &str| labels.get(&format!("homepage.{k}")).cloned();
    let service = Service {
        name: get("name")?,
        url: get("url").or_else(|| get("href")),
        icon: get("icon"),
        description: get("description"),
        check: Some(get("check").unwrap_or_else(|| container_check.into())),
        widget: widget_from(labels.iter(), "homepage.widget."),
        ..Default::default()
    };
    Some((get("group").unwrap_or_else(|| "Docker".into()), service))
}

/// Ingresses opt in with `gethomepage.dev/enabled: "true"`. The URL defaults to the first
/// rule's host and path (https when TLS covers that host) and is also what gets checked.
pub fn from_ingress(ing: &Ingress) -> Option<Found> {
    let annotations = ing.metadata.annotations.as_ref()?;
    let get = |k: &str| annotations.get(&format!("gethomepage.dev/{k}")).cloned();
    if get("enabled").as_deref() != Some("true") {
        return None;
    }

    let spec = ing.spec.as_ref();
    let rule = spec.and_then(|s| s.rules.as_ref()).and_then(|r| r.first());
    let host = rule.and_then(|r| r.host.as_deref());
    let path = rule
        .and_then(|r| r.http.as_ref())
        .and_then(|h| h.paths.first())
        .and_then(|p| p.path.as_deref())
        .unwrap_or("/");
    let tls = spec.and_then(|s| s.tls.as_ref()).is_some_and(|tls| {
        tls.iter()
            .filter_map(|t| t.hosts.as_ref())
            .any(|hosts| host.is_some_and(|h| hosts.iter().any(|x| x == h)))
    });
    let scheme = if tls { "https" } else { "http" };
    let url = get("href").or_else(|| host.map(|h| format!("{scheme}://{h}{path}")));

    let service = Service {
        name: get("name").or_else(|| ing.metadata.name.clone())?,
        check: get("check").or_else(|| url.clone()),
        url,
        icon: get("icon"),
        description: get("description"),
        widget: widget_from(annotations.iter(), "gethomepage.dev/widget."),
        ..Default::default()
    };
    Some((get("group").unwrap_or_else(|| "Kubernetes".into()), service))
}

/// Config groups first, discovered services appended to the group they name (created at
/// the end if missing). A service declared in the config wins over a discovered one.
pub fn merge(manual: &[Group], found: &[Found]) -> Vec<Group> {
    let mut groups = manual.to_vec();
    for (group, service) in found {
        let declared = manual
            .iter()
            .filter(|g| &g.name == group)
            .flat_map(|g| &g.services)
            .any(|s| s.name == service.name);
        if declared {
            continue;
        }
        let i = groups
            .iter()
            .position(|g| &g.name == group)
            .unwrap_or_else(|| {
                groups.push(Group {
                    name: group.clone(),
                    services: vec![],
                });
                groups.len() - 1
            });
        groups[i].services.push(service.clone());
    }
    groups
}

/// Services from one Docker host, plus the up/down state of every container on it.
async fn docker(host: &str, endpoint: &str) -> Result<(Vec<Found>, HashMap<String, bool>), String> {
    let client = if endpoint.starts_with("unix://") {
        Docker::connect_with_unix(endpoint, 10, API_DEFAULT_VERSION)
    } else {
        Docker::connect_with_http(endpoint, 10, API_DEFAULT_VERSION)
    };
    let client = client
        .map_err(|e| e.to_string())?
        .negotiate_version()
        .await
        .map_err(|e| e.to_string())?;
    let containers = client
        .list_containers(Some(ListContainersOptions {
            all: true,
            ..Default::default()
        }))
        .await
        .map_err(|e| e.to_string())?;

    let mut found = Vec::new();
    let mut states = HashMap::new();
    for c in containers {
        let Some(name) = c.names.as_ref().and_then(|n| n.first()) else {
            continue;
        };
        let target = format!("docker://{host}/{}", name.trim_start_matches('/'));
        let up = c.state == Some(ContainerSummaryStateEnum::RUNNING)
            && !c
                .status
                .as_deref()
                .unwrap_or_default()
                .contains("unhealthy");
        if let Some(f) = c.labels.as_ref().and_then(|l| from_labels(l, &target)) {
            found.push(f);
        }
        states.insert(target, up);
    }
    Ok((found, states))
}

async fn kubernetes() -> Result<Vec<Found>, String> {
    let client = kube::Client::try_default()
        .await
        .map_err(|e| e.to_string())?;
    let ingresses = Api::<Ingress>::all(client)
        .list(&ListParams::default())
        .await
        .map_err(|e| e.to_string())?;
    Ok(ingresses.items.iter().filter_map(from_ingress).collect())
}

pub async fn run(state: web::Data<State>) {
    // Last good result per source, so a host that drops out keeps its services on the
    // page (its containers then read as down) instead of making them vanish.
    let mut last: BTreeMap<String, Vec<Found>> = BTreeMap::new();
    let mut last_errors = Vec::new();
    loop {
        let (cfg, _) = state.config();
        let mut current = BTreeMap::new();
        let mut containers = HashMap::new();
        let mut errors = Vec::new();

        for (host, endpoint) in &cfg.docker {
            let source = format!("docker {host}");
            match docker(host, endpoint).await {
                Ok((found, states)) => {
                    current.insert(source, found);
                    containers.extend(states);
                }
                Err(e) => {
                    errors.push(format!("{source}: {e}"));
                    current.insert(source.clone(), last.remove(&source).unwrap_or_default());
                }
            }
        }
        if cfg.kubernetes {
            let source = "kubernetes".to_string();
            match kubernetes().await {
                Ok(found) => {
                    current.insert(source, found);
                }
                Err(e) => {
                    errors.push(format!("{source}: {e}"));
                    current.insert(source.clone(), last.remove(&source).unwrap_or_default());
                }
            }
        }

        let found = collect(&cfg, &current, &mut errors);
        last = current;
        if errors != last_errors {
            for e in &errors {
                eprintln!("discovery: {e}");
            }
            last_errors = errors;
        }

        *state.containers.write().unwrap() = containers;
        if *state.discovered.read().unwrap() != found {
            *state.discovered.write().unwrap() = found;
            state.targets_changed.notify_one();
        }

        let _ = time::timeout(INTERVAL, state.config_changed.notified()).await;
    }
}

/// Flatten sources into a stable order and drop checks the config would reject. Widget
/// problems show on the service card instead.
fn collect(
    cfg: &Config,
    sources: &BTreeMap<String, Vec<Found>>,
    errors: &mut Vec<String>,
) -> Vec<Found> {
    let mut found: Vec<Found> = sources.values().flatten().cloned().collect();
    for (_, s) in &mut found {
        if let Some(c) = &s.check
            && !cfg.valid_check(c)
        {
            errors.push(format!("{}: ignoring invalid check {c:?}", s.name));
            s.check = None;
        }
    }
    found.sort_by(|a, b| (&a.0, &a.1.name).cmp(&(&b.0, &b.1.name)));
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use k8s_openapi::api::networking::v1::{
        HTTPIngressPath, HTTPIngressRuleValue, IngressRule, IngressSpec, IngressTLS,
    };
    use kube::api::ObjectMeta;

    fn svc(name: &str) -> Service {
        Service {
            name: name.into(),
            ..Default::default()
        }
    }

    #[test]
    fn labels() {
        let labels: HashMap<_, _> = [
            ("homepage.name", "Jellyfin"),
            ("homepage.href", "https://jf.lan"),
            ("homepage.icon", "jellyfin"),
            ("homepage.widget.type", "jellyfin"),
            ("homepage.widget.key", "abc"),
            ("homepage.widget.fields", r#"["Movies", "Series"]"#),
        ]
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .into();
        let (group, s) = from_labels(&labels, "docker://local/jellyfin").unwrap();
        assert_eq!(group, "Docker");
        assert_eq!(s.url.as_deref(), Some("https://jf.lan"));
        assert_eq!(s.check.as_deref(), Some("docker://local/jellyfin"));
        let w = s.widget.unwrap();
        assert_eq!(
            (w.kind.as_str(), w.options["key"].as_str()),
            ("jellyfin", "abc")
        );
        assert_eq!(w.fields.unwrap(), ["Movies", "Series"]);
        assert!(
            from_labels(&HashMap::new(), "x").is_none(),
            "unlabelled containers stay hidden"
        );
    }

    #[test]
    fn ingress() {
        let annotations = [
            ("gethomepage.dev/enabled", "true"),
            ("gethomepage.dev/group", "Apps"),
        ]
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .into();
        let mut ing = Ingress {
            metadata: ObjectMeta {
                name: Some("grafana".into()),
                annotations: Some(annotations),
                ..Default::default()
            },
            spec: Some(IngressSpec {
                rules: Some(vec![IngressRule {
                    host: Some("grafana.lan".into()),
                    http: Some(HTTPIngressRuleValue {
                        paths: vec![HTTPIngressPath {
                            path: Some("/ui".into()),
                            ..Default::default()
                        }],
                    }),
                }]),
                tls: Some(vec![IngressTLS {
                    hosts: Some(vec!["grafana.lan".into()]),
                    ..Default::default()
                }]),
                ..Default::default()
            }),
            ..Default::default()
        };
        let (group, s) = from_ingress(&ing).unwrap();
        assert_eq!((group.as_str(), s.name.as_str()), ("Apps", "grafana"));
        assert_eq!(s.url.as_deref(), Some("https://grafana.lan/ui"));
        assert_eq!(s.check, s.url);

        ing.spec.as_mut().unwrap().tls = None;
        assert_eq!(
            from_ingress(&ing).unwrap().1.url.as_deref(),
            Some("http://grafana.lan/ui")
        );

        ing.metadata
            .annotations
            .as_mut()
            .unwrap()
            .remove("gethomepage.dev/enabled");
        assert!(from_ingress(&ing).is_none());
    }

    #[test]
    fn merging() {
        let manual = vec![Group {
            name: "Media".into(),
            services: vec![svc("Jellyfin")],
        }];
        let found = vec![
            (
                "Media".into(),
                Service {
                    url: Some("ignored".into()),
                    ..svc("Jellyfin")
                },
            ),
            ("Media".into(), svc("Sonarr")),
            ("Infra".into(), svc("Traefik")),
        ];
        let groups = merge(&manual, &found);
        let names: Vec<Vec<&str>> = groups
            .iter()
            .map(|g| g.services.iter().map(|s| s.name.as_str()).collect())
            .collect();
        assert_eq!(names, [vec!["Jellyfin", "Sonarr"], vec!["Traefik"]]);
        assert_eq!(
            groups[0].services[0].url, None,
            "config wins over discovery"
        );
        assert_eq!(groups[1].name, "Infra");
    }
}
