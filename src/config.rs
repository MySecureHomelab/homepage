use crate::widgets::{self, Definition, WidgetConfig};
use serde::Deserialize;
use serde::de::IgnoredAny;
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
    /// Left and right page margin in px. Unset: content centered, at most 1320px wide.
    pub side_margin: Option<u32>,
    #[serde(default)]
    pub background: Option<Background>,
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
    /// Problems found on load. They don't stop the dashboard: the faulty value is left out
    /// (or replaced by its default) and the page lists them.
    #[serde(skip)]
    pub issues: Vec<String>,
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

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Background {
    /// `http(s)://…` (downloaded once by the server, kept in memory) or a file path,
    /// relative to the config file's directory.
    pub image: String,
    /// Blur radius in px.
    #[serde(default)]
    pub blur: u32,
    /// Veil of the theme's background color over the image, 0–100 %, so text stays readable.
    #[serde(default = "default_dim")]
    pub dim: u8,
}

fn default_dim() -> u8 {
    40
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

/// Also accepts gethomepage's names (`href`, `siteMonitor`, `server` + `container`, `widgets`),
/// so its services.yaml entries carry over. Other keys are reported and ignored.
#[derive(Debug, Deserialize, Clone, Default, PartialEq)]
pub struct Service {
    pub name: String,
    #[serde(alias = "href")]
    pub url: Option<String>,
    pub icon: Option<String>,
    pub description: Option<String>,
    /// `http(s)://…` (up if the server answers without a 5xx), `tcp://host:port`,
    /// or `docker://<docker host name>/<container>` (up if running and not unhealthy).
    #[serde(alias = "siteMonitor")]
    pub check: Option<String>,
    pub widget: Option<WidgetConfig>,
    /// gethomepage: several widgets per service; the first one is used.
    #[serde(default)]
    pub widgets: Vec<WidgetConfig>,
    /// gethomepage: Docker host and container, turned into a `docker://` check.
    pub server: Option<String>,
    pub container: Option<String>,
    #[serde(flatten)]
    pub other: BTreeMap<String, IgnoredAny>,
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
    #[serde(alias = "href")]
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

    /// Resolves gethomepage keys, then checks the service. Problems go to `issues`; the
    /// service stays on the page without the faulty part.
    fn review_service(&self, s: &mut Service, issues: &mut Vec<String>) {
        if s.widget.is_none() && !s.widgets.is_empty() {
            s.widget = Some(s.widgets.remove(0));
        }
        if !s.widgets.is_empty() {
            issues.push("only one widget per service, the others are ignored".into());
            s.widgets.clear();
        }
        if let (None, Some(container)) = (&s.check, s.container.take()) {
            match s.server.take() {
                Some(host) if self.docker.contains_key(&host) => {
                    s.check = Some(format!("docker://{host}/{container}"));
                }
                host => issues.push(format!(
                    "container {container:?}: docker host {:?} is not declared under `docker:`",
                    host.unwrap_or_default()
                )),
            }
        }
        (s.server, s.container) = (None, None);
        if !s.other.is_empty() {
            let keys: Vec<&str> = s.other.keys().map(String::as_str).collect();
            issues.push(format!("ignored keys: {}", keys.join(", ")));
            s.other.clear();
        }
        if let Some(c) = &s.check
            && !self.valid_check(c)
        {
            issues.push(format!(
                "check must be http(s)://…, tcp://host:port or docker://<docker host>/<container>, got {c:?}"
            ));
            s.check = None;
        }
        // A widget that cannot work stays: its card shows why.
        if let Some(w) = &s.widget {
            match widgets::validate(&self.widget_types, w, s.url.is_some()) {
                Ok(warnings) => issues.extend(warnings),
                Err(e) => issues.push(e),
            }
        }
    }

    /// Replaces invalid settings by their default and records why, so a mistake never takes
    /// the whole dashboard down.
    fn review(&mut self) {
        let mut issues = Vec::new();
        let hex = self.accent.len() == 7
            && self.accent.starts_with('#')
            && self.accent[1..].chars().all(|c| c.is_ascii_hexdigit());
        if !hex {
            issues.push(format!(
                "accent: expected a color like \"#7c6cf2\", got {:?}",
                self.accent
            ));
            self.accent = default_accent();
        }
        if !self.search_template().contains("{q}") {
            issues.push(format!(
                "search: expected duckduckgo, google, bing, startpage or a URL containing {{q}}, got {:?}",
                self.search
            ));
            self.search = default_search();
        }
        if self
            .background
            .as_ref()
            .is_some_and(|b| b.image.trim().is_empty())
        {
            issues.push("background.image: expected a file path or an http(s):// URL".into());
            self.background = None;
        }
        if let Some(b) = &mut self.background
            && b.dim > 100
        {
            issues.push(format!("background.dim: expected 0 to 100, got {}", b.dim));
            b.dim = default_dim();
        }
        if self.check_interval == 0 {
            issues.push("check_interval: must be at least 1 second".into());
            self.check_interval = default_interval();
        }
        self.docker.retain(|name, endpoint| {
            let scheme_ok = ["unix://", "tcp://", "http://"]
                .iter()
                .any(|p| endpoint.starts_with(p));
            let ok = !name.is_empty() && !name.contains('/') && scheme_ok;
            if !ok {
                issues.push(format!(
                    "docker.{name}: expected a name without '/' and an endpoint like unix:///var/run/docker.sock or tcp://host:2375, got {endpoint:?}"
                ));
            }
            ok
        });
        self.widget_types.retain(|name, def| match def.validate() {
            Ok(()) => true,
            Err(e) => {
                issues.push(format!("widget_types.{name}: {e}"));
                false
            }
        });
        let mut groups = std::mem::take(&mut self.groups);
        for (gi, g) in groups.iter_mut().enumerate() {
            for (si, s) in g.services.iter_mut().enumerate() {
                let mut found = Vec::new();
                self.review_service(s, &mut found);
                let at = format!("groups[{gi}].services[{si}] ({})", s.name);
                issues.extend(found.into_iter().map(|e| format!("{at}: {e}")));
            }
        }
        self.groups = groups;
        self.issues.extend(issues);
    }

    /// The config used when the file cannot be read at all: an empty page with the error.
    pub fn empty() -> Config {
        serde_saphyr::from_str("title: Homepage").expect("default config")
    }
}

/// Expands `${NAME}` (environment variable) and `${file:/path}` (file contents, trimmed),
/// so secrets stay out of the config file. YAML comments are left alone. A missing variable
/// or file becomes an empty value and is reported.
fn substitute(raw: &str) -> (String, Vec<String>) {
    let mut out = String::with_capacity(raw.len());
    let mut issues = Vec::new();
    for (n, line) in raw.split_inclusive('\n').enumerate() {
        let (code, comment) = line.split_at(comment_start(line));
        out.push_str(&substitute_line(code, &mut |e| {
            issues.push(format!("line {}: {e}", n + 1));
        }));
        out.push_str(comment);
    }
    (out, issues)
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

fn substitute_line(line: &str, issue: &mut impl FnMut(String)) -> String {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(start) = rest.find("${") {
        let Some(end) = rest[start..].find('}') else {
            issue("unclosed ${".into());
            break;
        };
        let name = &rest[start + 2..start + end];
        let value = match name.strip_prefix("file:") {
            Some(file) => fs::read_to_string(file)
                .map(|v| v.trim().to_string())
                .unwrap_or_else(|e| {
                    issue(format!("${{{name}}}: {e}, using an empty value"));
                    String::new()
                }),
            None => std::env::var(name).unwrap_or_else(|_| {
                issue(format!(
                    "${{{name}}}: environment variable is not set, using an empty value"
                ));
                String::new()
            }),
        };
        out.push_str(&rest[..start]);
        out.push_str(&value);
        rest = &rest[start + end + 1..];
    }
    out.push_str(rest);
    out
}

/// `Err` only when the file cannot be read or is not a valid config at all (YAML syntax,
/// wrong types, unknown top-level key). Everything else ends up in `issues`.
pub fn load(path: &Path) -> Result<Config, String> {
    let raw = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let (raw, issues) = substitute(&raw);
    let mut cfg: Config =
        serde_saphyr::from_str(&raw).map_err(|e| format!("{}: {e}", path.display()))?;
    cfg.issues = issues;
    cfg.review();
    Ok(cfg)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The config and its issues, joined; `Err` only for configs that cannot load at all.
    fn parse(yaml: &str) -> Result<(Config, String), String> {
        let mut cfg: Config = serde_saphyr::from_str(yaml).map_err(|e| e.to_string())?;
        cfg.review();
        let issues = cfg.issues.join("\n");
        Ok((cfg, issues))
    }

    fn issues(yaml: &str) -> String {
        parse(yaml).unwrap().1
    }

    #[test]
    fn background() {
        let (cfg, found) = parse("background: {image: bg.jpg, dim: 150}").unwrap();
        assert!(found.contains("background.dim"));
        let b = cfg.background.unwrap();
        assert_eq!((b.blur, b.dim), (0, 40));
        let (cfg, found) = parse("background: {image: ' '}").unwrap();
        assert!(found.contains("background.image") && cfg.background.is_none());
    }

    #[test]
    fn example_config_is_valid() {
        let cfg = load(Path::new("config.example.yaml")).unwrap();
        assert!(cfg.issues.is_empty(), "{:?}", cfg.issues);
        let demo = load(Path::new("docs/demo/config.yaml")).unwrap();
        assert!(demo.issues.is_empty(), "{:?}", demo.issues);
        assert!(Config::empty().groups.is_empty());
    }

    #[test]
    fn substitution() {
        // SAFETY: test-only, no other thread reads this variable.
        unsafe { std::env::set_var("HP_TEST_KEY", "s3cret") };
        assert_eq!(substitute("key: ${HP_TEST_KEY}").0, "key: s3cret");
        let (out, errs) = substitute("a: 1\nkey: ${HP_TEST_UNSET}");
        assert_eq!(out, "a: 1\nkey: ");
        assert!(errs[0].starts_with("line 2:") && errs[0].contains("HP_TEST_UNSET"));
        assert_eq!(substitute("no vars {here}").0, "no vars {here}");
        let commented =
            "a: 1 # ${HP_TEST_UNSET}\n# key: ${HP_TEST_UNSET}\nb: \"#${HP_TEST_KEY}\"\n";
        assert_eq!(
            substitute(commented),
            (
                "a: 1 # ${HP_TEST_UNSET}\n# key: ${HP_TEST_UNSET}\nb: \"#s3cret\"\n".into(),
                vec![]
            )
        );
        assert!(substitute("k: ${file:/nonexistent/x}").1[0].contains("/nonexistent/x"));
    }

    #[test]
    fn widgets() {
        let ok = "groups: [{name: a, services: [{name: b, url: 'http://s', widget: {type: sonarr, key: k}}]}]";
        assert_eq!(issues(ok), "");
        let cases = [
            ("{type: nope, key: k}", "unknown widget type"),
            ("{type: sonarr}", "missing option `key`"),
            ("{type: sonarr, key: k, kye: k}", "unknown option `kye`"),
            ("{type: sonarr, key: k, fields: [Nope]}", "unknown field"),
            (
                "{type: sonarr, key: k, enableQueue: true, chart: [a]}",
                "`chart` ignored",
            ),
        ];
        for (widget, err) in cases {
            let yaml = format!(
                "groups: [{{name: a, services: [{{name: b, url: 'http://s', widget: {widget}}}]}}]"
            );
            let (cfg, found) = parse(&yaml).unwrap();
            assert!(found.contains(err), "{widget}: {found}");
            assert!(
                cfg.groups[0].services[0].widget.is_some(),
                "the card keeps its widget and shows the problem"
            );
        }
        let gethomepage_fields = "groups: [{name: a, services: [{name: b, url: 'http://s', widget: {type: sonarr, key: k, fields: [wanted, QUEUED]}}]}]";
        assert_eq!(issues(gethomepage_fields), "");
    }

    #[test]
    fn gethomepage_services() {
        let yaml = r#"
docker: {nas: "tcp://nas:2375"}
groups:
  - name: Media
    services:
      - name: Sonarr
        href: https://sonarr.lan
        siteMonitor: https://sonarr.lan/ping
        showStats: true
        widgets: [{type: sonarr, key: k}]
      - name: Jellyfin
        server: nas
        container: jellyfin
      - name: Plex
        server: other
        container: plex
"#;
        let (cfg, found) = parse(yaml).unwrap();
        let s = &cfg.groups[0].services;
        assert_eq!(s[0].url.as_deref(), Some("https://sonarr.lan"));
        assert_eq!(s[0].check.as_deref(), Some("https://sonarr.lan/ping"));
        assert_eq!(s[0].widget.as_ref().unwrap().kind, "sonarr");
        assert_eq!(s[1].check.as_deref(), Some("docker://nas/jellyfin"));
        assert_eq!(s[2].check, None);
        assert!(
            found.contains("(Sonarr): ignored keys: showStats"),
            "{found}"
        );
        assert!(found.contains("(Plex): container \"plex\""), "{found}");
    }

    #[test]
    fn bad_values_fall_back() {
        assert!(
            parse("titel: x")
                .unwrap_err()
                .contains("unknown field `titel`"),
            "unknown top-level keys still refuse the file"
        );
        let (cfg, found) =
            parse("accent: red\nsearch: https://s.lan/?q=\ncheck_interval: 0").unwrap();
        assert!(
            found.contains("accent") && found.contains("{q}") && found.contains("check_interval")
        );
        assert_eq!(
            (cfg.accent.as_str(), cfg.search.as_str(), cfg.check_interval),
            ("#7c6cf2", "duckduckgo", 30)
        );
        let bad_check = "groups: [{name: a, services: [{name: b, check: 'tcp://host'}]}]";
        let (cfg, found) = parse(bad_check).unwrap();
        assert!(found.contains("services[0] (b)"));
        assert_eq!(cfg.groups[0].services[0].check, None);
        assert_eq!(issues("search: https://s.lan/?q={q}"), "");
        let unknown_host =
            "groups: [{name: a, services: [{name: b, check: 'docker://nas/jellyfin'}]}]";
        assert!(!issues(unknown_host).is_empty());
        assert_eq!(
            issues(&format!(
                "docker: {{nas: 'tcp://nas:2375'}}\n{unknown_host}"
            )),
            ""
        );
        let (cfg, found) = parse("docker: {nas: 'nas:2375'}").unwrap();
        assert!(found.contains("docker.nas") && cfg.docker.is_empty());
    }
}
