use serde::Deserialize;
use std::{collections::BTreeMap, fs, path::Path};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default = "default_title")]
    pub title: String,
    #[serde(default)]
    pub theme: Theme,
    #[serde(default = "default_accent")]
    pub accent: String,
    /// `duckduckgo`, `google`, `bing`, `startpage`, or a URL containing `{q}` (e.g. `SearXNG`).
    #[serde(default = "default_search")]
    pub search: String,
    /// Seconds between status checks.
    #[serde(default = "default_interval")]
    pub check_interval: u64,
    #[serde(default = "default_widgets")]
    pub widgets: Vec<Widget>,
    #[serde(default)]
    pub groups: Vec<Group>,
    #[serde(default)]
    pub bookmarks: Vec<BookmarkGroup>,
    /// Docker hosts to discover services from, by name: `unix:///var/run/docker.sock` or `tcp://host:2375`.
    #[serde(default)]
    pub docker: BTreeMap<String, String>,
    /// Discover services from Kubernetes Ingresses (in-cluster config or `~/.kube/config`).
    #[serde(default)]
    pub kubernetes: bool,
}

#[derive(Debug, Deserialize, Default, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    Auto,
    Light,
    Dark,
}

impl Theme {
    pub fn as_str(self) -> &'static str {
        match self {
            Theme::Auto => "auto",
            Theme::Light => "light",
            Theme::Dark => "dark",
        }
    }
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Widget {
    Clock,
    Search,
    Resources,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Group {
    pub name: String,
    #[serde(default)]
    pub services: Vec<Service>,
}

#[derive(Debug, Deserialize, Clone, Default, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Service {
    pub name: String,
    pub url: Option<String>,
    pub icon: Option<String>,
    pub description: Option<String>,
    /// `http(s)://…` (up if the server answers without a 5xx), `tcp://host:port`,
    /// or `docker://<docker host name>/<container>` (up if running and not unhealthy).
    pub check: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BookmarkGroup {
    pub name: String,
    #[serde(default)]
    pub links: Vec<Link>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Link {
    pub name: String,
    pub url: String,
    pub icon: Option<String>,
}

fn default_title() -> String {
    "Homepage".into()
}
fn default_accent() -> String {
    "#7c6cf2".into()
}
fn default_search() -> String {
    "duckduckgo".into()
}
fn default_interval() -> u64 {
    30
}
fn default_widgets() -> Vec<Widget> {
    vec![Widget::Clock, Widget::Search, Widget::Resources]
}

/// Bare names resolve to dashboard-icons; anything with a `/` is used as a URL.
pub fn icon_url(icon: &str) -> String {
    if icon.contains('/') {
        icon.into()
    } else {
        format!("https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/{icon}.svg")
    }
}

impl Service {
    pub fn icon_src(&self) -> Option<String> {
        self.icon.as_deref().map(icon_url)
    }
}

impl Link {
    pub fn icon_src(&self) -> Option<String> {
        self.icon.as_deref().map(icon_url)
    }
}

impl Config {
    pub fn search_template(&self) -> &str {
        match self.search.as_str() {
            "duckduckgo" => "https://duckduckgo.com/?q={q}",
            "google" => "https://www.google.com/search?q={q}",
            "bing" => "https://www.bing.com/search?q={q}",
            "startpage" => "https://www.startpage.com/do/search?q={q}",
            custom => custom,
        }
    }

    pub fn valid_check(&self, check: &str) -> bool {
        check.starts_with("http://")
            || check.starts_with("https://")
            || check.strip_prefix("tcp://").is_some_and(|a| {
                a.rsplit_once(':')
                    .is_some_and(|(_, p)| p.parse::<u16>().is_ok())
            })
            || check
                .strip_prefix("docker://")
                .and_then(|r| r.split_once('/'))
                .is_some_and(|(host, name)| self.docker.contains_key(host) && !name.is_empty())
    }

    fn validate(&self) -> Result<(), String> {
        let hex = self.accent.len() == 7
            && self.accent.starts_with('#')
            && self.accent[1..].chars().all(|c| c.is_ascii_hexdigit());
        if !hex {
            return Err(format!(
                "accent: expected a color like \"#7c6cf2\", got {:?}",
                self.accent
            ));
        }
        if !self.search_template().contains("{q}") {
            return Err(format!(
                "search: expected duckduckgo, google, bing, startpage or a URL containing {{q}}, got {:?}",
                self.search
            ));
        }
        if self.check_interval == 0 {
            return Err("check_interval: must be at least 1 second".into());
        }
        for (name, endpoint) in &self.docker {
            let scheme_ok = ["unix://", "tcp://", "http://"]
                .iter()
                .any(|p| endpoint.starts_with(p));
            if name.is_empty() || name.contains('/') || !scheme_ok {
                return Err(format!(
                    "docker.{name}: expected a name without '/' and an endpoint like unix:///var/run/docker.sock or tcp://host:2375, got {endpoint:?}"
                ));
            }
        }
        for (gi, g) in self.groups.iter().enumerate() {
            for (si, s) in g.services.iter().enumerate() {
                let Some(c) = &s.check else { continue };
                if !self.valid_check(c) {
                    return Err(format!(
                        "groups[{gi}].services[{si}] ({}): check must be http(s)://…, tcp://host:port or docker://<docker host>/<container>, got {c:?}",
                        s.name
                    ));
                }
            }
        }
        Ok(())
    }
}

pub fn load(path: &Path) -> Result<Config, String> {
    let raw = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let cfg: Config =
        serde_saphyr::from_str(&raw).map_err(|e| format!("{}: {e}", path.display()))?;
    cfg.validate()
        .map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(cfg)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(yaml: &str) -> Result<Config, String> {
        let cfg: Config = serde_saphyr::from_str(yaml).map_err(|e| e.to_string())?;
        cfg.validate().map(|()| cfg)
    }

    #[test]
    fn example_config_is_valid() {
        load(Path::new("config.example.yaml")).unwrap();
    }

    #[test]
    fn rejects_bad_input() {
        assert!(
            parse("titel: x")
                .unwrap_err()
                .contains("unknown field `titel`")
        );
        assert!(parse("accent: red").unwrap_err().contains("accent"));
        assert!(
            parse("search: https://s.lan/?q=")
                .unwrap_err()
                .contains("{q}")
        );
        let bad_check = "groups: [{name: a, services: [{name: b, check: 'tcp://host'}]}]";
        assert!(parse(bad_check).unwrap_err().contains("services[0] (b)"));
        assert!(parse("search: https://s.lan/?q={q}").is_ok());
        let unknown_host =
            "groups: [{name: a, services: [{name: b, check: 'docker://nas/jellyfin'}]}]";
        assert!(parse(unknown_host).is_err());
        assert!(
            parse(&format!(
                "docker: {{nas: 'tcp://nas:2375'}}\n{unknown_host}"
            ))
            .is_ok()
        );
        assert!(
            parse("docker: {nas: 'nas:2375'}")
                .unwrap_err()
                .contains("docker.nas")
        );
    }
}
