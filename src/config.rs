use crate::widgets::{self, Definition, WidgetConfig};
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
    /// Extra widget definitions, same format as the built-in ones in `widgets/*.yaml`.
    #[serde(default)]
    pub widget_types: BTreeMap<String, Definition>,
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
    pub widget: Option<WidgetConfig>,
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

    /// Also used for discovered services.
    pub fn validate_service(&self, s: &Service) -> Result<(), String> {
        if let Some(c) = &s.check
            && !self.valid_check(c)
        {
            return Err(format!(
                "check must be http(s)://…, tcp://host:port or docker://<docker host>/<container>, got {c:?}"
            ));
        }
        if let Some(w) = &s.widget {
            widgets::validate(&self.widget_types, w, s.url.is_some())?;
        }
        Ok(())
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
        for (name, def) in &self.widget_types {
            def.validate()
                .map_err(|e| format!("widget_types.{name}: {e}"))?;
        }
        for (gi, g) in self.groups.iter().enumerate() {
            for (si, s) in g.services.iter().enumerate() {
                let at = format!("groups[{gi}].services[{si}] ({})", s.name);
                self.validate_service(s).map_err(|e| format!("{at}: {e}"))?;
            }
        }
        Ok(())
    }
}

/// Expands `${NAME}` (environment variable) and `${file:/path}` (file contents, trimmed),
/// so secrets stay out of the config file. YAML comments are left alone.
fn substitute(raw: &str) -> Result<String, String> {
    let mut out = String::with_capacity(raw.len());
    for (n, line) in raw.split_inclusive('\n').enumerate() {
        let (code, comment) = line.split_at(comment_start(line));
        out.push_str(&substitute_line(code).map_err(|e| format!("line {}: {e}", n + 1))?);
        out.push_str(comment);
    }
    Ok(out)
}

/// Byte offset of a YAML comment (`#` at line start or after whitespace, outside quotes).
fn comment_start(line: &str) -> usize {
    let mut quote = None;
    let mut prev = ' ';
    for (i, c) in line.char_indices() {
        match (quote, c) {
            (None, '\'' | '"') => quote = Some(c),
            (Some(q), c) if c == q => quote = None,
            (None, '#') if prev.is_whitespace() => return i,
            _ => {}
        }
        prev = c;
    }
    line.len()
}

fn substitute_line(line: &str) -> Result<String, String> {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(start) = rest.find("${") {
        let end = rest[start..].find('}').ok_or("unclosed ${")?;
        let name = &rest[start + 2..start + end];
        let value = match name.strip_prefix("file:") {
            Some(file) => fs::read_to_string(file)
                .map_err(|e| format!("${{{name}}}: {e}"))?
                .trim()
                .to_string(),
            None => std::env::var(name)
                .map_err(|_| format!("${{{name}}}: environment variable is not set"))?,
        };
        out.push_str(&rest[..start]);
        out.push_str(&value);
        rest = &rest[start + end + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

pub fn load(path: &Path) -> Result<Config, String> {
    let raw = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let raw = substitute(&raw).map_err(|e| format!("{}: {e}", path.display()))?;
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
    fn substitution() {
        // SAFETY: test-only, no other thread reads this variable.
        unsafe { std::env::set_var("HP_TEST_KEY", "s3cret") };
        assert_eq!(substitute("key: ${HP_TEST_KEY}").unwrap(), "key: s3cret");
        assert!(
            substitute("key: ${HP_TEST_UNSET}")
                .unwrap_err()
                .contains("HP_TEST_UNSET")
        );
        assert_eq!(substitute("no vars {here}").unwrap(), "no vars {here}");
        let commented =
            "a: 1 # ${HP_TEST_UNSET}\n# key: ${HP_TEST_UNSET}\nb: \"#${HP_TEST_KEY}\"\n";
        assert_eq!(
            substitute(commented).unwrap(),
            "a: 1 # ${HP_TEST_UNSET}\n# key: ${HP_TEST_UNSET}\nb: \"#s3cret\"\n"
        );
        assert!(
            substitute("a: 1\nb: ${HP_TEST_UNSET}")
                .unwrap_err()
                .starts_with("line 2:")
        );
    }

    #[test]
    fn widgets() {
        let ok = "groups: [{name: a, services: [{name: b, url: 'http://s', widget: {type: sonarr, key: k}}]}]";
        assert!(parse(ok).is_ok());
        let cases = [
            ("{type: nope, key: k}", "unknown widget type"),
            ("{type: sonarr}", "missing option `key`"),
            ("{type: sonarr, key: k, kye: k}", "unknown option `kye`"),
            ("{type: sonarr, key: k, fields: [Nope]}", "unknown field"),
        ];
        for (widget, err) in cases {
            let yaml = format!(
                "groups: [{{name: a, services: [{{name: b, url: 'http://s', widget: {widget}}}]}}]"
            );
            let e = parse(&yaml).unwrap_err();
            assert!(e.contains(err), "{widget}: {e}");
        }
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
