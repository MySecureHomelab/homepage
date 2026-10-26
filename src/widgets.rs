//! Service widgets: stats pulled from a service's API, described declaratively.
//!
//! A definition lists named requests, optional auth headers and login step, and fields
//! computed with jq over `{"<request name>": <response>, ...}`. Built-in definitions live in
//! `widgets/*.yaml`; users can add their own under `widget_types` in the config.

use base64::{Engine as _, prelude::BASE64_STANDARD};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    sync::{LazyLock, Mutex},
    time::Duration,
};

const BUILTIN_SOURCES: &[&str] = &[
    include_str!("../widgets/media.yaml"),
    include_str!("../widgets/arr.yaml"),
    include_str!("../widgets/downloads.yaml"),
    include_str!("../widgets/network.yaml"),
    include_str!("../widgets/infra.yaml"),
    include_str!("../widgets/apps.yaml"),
];

pub static BUILTIN: LazyLock<BTreeMap<String, Definition>> = LazyLock::new(|| {
    let mut all = BTreeMap::new();
    for src in BUILTIN_SOURCES {
        let defs: BTreeMap<String, Definition> =
            serde_saphyr::from_str(src).expect("built-in widget definitions are valid YAML");
        all.extend(defs);
    }
    all
});

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Definition {
    /// Options the user must set, e.g. `[key]`. `url` is always required.
    #[serde(default)]
    pub needs: Vec<String>,
    /// Values for options the user may leave out.
    #[serde(default)]
    pub defaults: BTreeMap<String, String>,
    /// Sent with every request. A header whose placeholders aren't all set is skipped,
    /// so a definition can offer alternatives (API key or basic auth).
    #[serde(default)]
    pub headers: BTreeMap<String, Header>,
    /// Extra placeholders computed with jq (input `null`, `$options` available),
    /// e.g. `month_start: 'now | strftime("%Y-%m-01")'`.
    #[serde(default)]
    pub computed: BTreeMap<String, String>,
    pub login: Option<Login>,
    pub requests: BTreeMap<String, Request>,
    pub fields: Vec<Field>,
}

/// A header value, or alternatives tried in order: the first whose placeholders are all set
/// is sent (`Authorization: ["Bearer {key}", "Basic {basic}"]`).
#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum Header {
    One(String),
    Any(Vec<String>),
}

impl Header {
    fn alternatives(&self) -> &[String] {
        match self {
            Header::One(v) => std::slice::from_ref(v),
            Header::Any(v) => v,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum Request {
    Get(String),
    Full(FullRequest),
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct FullRequest {
    pub path: String,
    #[serde(default)]
    pub method: Method,
    /// Query parameters; like headers, one whose placeholders aren't all set is skipped.
    #[serde(default)]
    pub query: BTreeMap<String, String>,
    /// JSON body; object entries whose placeholders aren't all set are dropped.
    pub json: Option<serde_json::Value>,
    /// Form body; entries whose placeholders aren't all set are dropped.
    pub form: Option<BTreeMap<String, String>>,
    #[serde(default)]
    pub headers: BTreeMap<String, Header>,
    /// Use this response header (as a string) instead of the body, e.g. `X-Total-Count`.
    pub header: Option<String>,
    /// On failure (or an unset placeholder in the path) the response is `null` instead of
    /// failing the whole widget.
    #[serde(default)]
    pub optional: bool,
    /// jq over the plain (non-`each`) responses, giving an array: the request is sent once
    /// per element with `{item}` set to it, and its response is the array of results.
    pub each: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Copy, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum Method {
    #[default]
    Get,
    Post,
}

/// Obtains `{token}`: from the JSON response (jq), a cookie, or a response header.
/// For a cookie, `{token}` is the whole `name=value` pair (send it as `Cookie: "{token}"`)
/// and the name may use `*` wildcards, e.g. `*SID*` for qBittorrent's `SID` / `QBT_SID_8080`.
/// Skipped when one of its placeholders is unset (e.g. a service usable without auth).
#[derive(Debug, Deserialize, Clone)]
pub struct Login {
    #[serde(flatten)]
    pub request: FullRequest,
    pub token: Option<String>,
    pub cookie: Option<String>,
    pub header: Option<String>,
    /// jq over each response; true means the session expired even though the status is 2xx.
    pub expired: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Field {
    pub label: String,
    /// The gethomepage name of this field (`wanted`, `blocked_percent`…), so `fields` lists
    /// written for gethomepage select the same fields.
    pub key: Option<String>,
    /// Only shown when the user lists it in `fields` (gethomepage shows a subset by default).
    #[serde(default)]
    pub hidden: bool,
    /// jq over the responses (like `value`, with `$options`): when false or null, the field is
    /// hidden like `hidden: true`. For fields that only apply to one mode
    /// (`'$options.version == "2"'`) or one state (`'.status.state == "printing"'`).
    pub when: Option<String>,
    /// jq over the object of responses.
    pub value: String,
    #[serde(default)]
    pub format: Format,
}

#[derive(Debug, Deserialize, Clone, Copy, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    #[default]
    Number,
    /// Bytes, binary units.
    Bytes,
    /// Bytes per second.
    Rate,
    /// Bits per second.
    Bitrate,
    /// Already 0–100.
    Percent,
    /// Seconds.
    Duration,
    /// Unix timestamp in seconds, shown relative to now ("3h ago").
    Ago,
    Text,
}

/// What a user writes under `widget:` on a service.
#[derive(Debug, Deserialize, Clone, PartialEq, Default)]
#[serde(from = "RawWidget")]
pub struct WidgetConfig {
    pub kind: String,
    /// Defaults to the service URL.
    pub url: Option<String>,
    /// Labels (or gethomepage keys) to show, in order. Defaults to all fields of the definition.
    pub fields: Option<Vec<String>>,
    /// `key`, `username`, `password`, and whatever else the definition uses.
    pub options: BTreeMap<String, String>,
    /// Options that are lists or maps (gethomepage display settings), reported and left out.
    pub ignored: Vec<String>,
}

/// Option values may be numbers or booleans (`enableQueue: true`): they are used as strings.
#[derive(Deserialize)]
struct RawWidget {
    #[serde(rename = "type")]
    kind: String,
    url: Option<String>,
    fields: Option<Vec<String>>,
    #[serde(flatten)]
    options: BTreeMap<String, serde_json::Value>,
}

impl From<RawWidget> for WidgetConfig {
    fn from(raw: RawWidget) -> Self {
        let mut w = WidgetConfig {
            kind: raw.kind,
            url: raw.url,
            fields: raw.fields,
            ..Default::default()
        };
        for (k, v) in raw.options {
            match v {
                serde_json::Value::String(s) => drop(w.options.insert(k, s)),
                serde_json::Value::Number(_) | serde_json::Value::Bool(_) => {
                    drop(w.options.insert(k, v.to_string()));
                }
                _ => w.ignored.push(k),
            }
        }
        w
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Stats {
    pub fields: Vec<(String, String)>,
    pub error: Option<String>,
}

pub fn lookup<'a>(custom: &'a BTreeMap<String, Definition>, kind: &str) -> Option<&'a Definition> {
    custom.get(kind).or_else(|| BUILTIN.get(kind))
}

// ---------- validation ----------

/// Same rule as `fill`: only `{simple_name}` is a placeholder, other braces (JSON) are literal.
fn placeholders(s: &str) -> impl Iterator<Item = &str> {
    s.split('{')
        .skip(1)
        .filter_map(|p| p.split_once('}').map(|(name, _)| name))
        .filter(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
}

impl Definition {
    /// A field by label or gethomepage key, ignoring case.
    pub fn field(&self, name: &str) -> Option<&Field> {
        self.fields.iter().find(|f| {
            f.label.eq_ignore_ascii_case(name)
                || f.key
                    .as_deref()
                    .is_some_and(|k| k.eq_ignore_ascii_case(name))
        })
    }

    fn templates(&self) -> Vec<String> {
        fn add(out: &mut Vec<String>, r: &FullRequest) {
            out.push(r.path.clone());
            out.extend(r.query.values().cloned());
            out.extend(r.headers.values().flat_map(|h| h.alternatives().to_vec()));
            out.extend(r.form.iter().flat_map(|f| f.values().cloned()));
            if let Some(j) = &r.json {
                out.push(j.to_string());
            }
        }
        let mut out: Vec<String> = self
            .headers
            .values()
            .flat_map(|h| h.alternatives().to_vec())
            .collect();
        if let Some(l) = &self.login {
            add(&mut out, &l.request);
        }
        for r in self.requests.values() {
            match r {
                Request::Get(p) => out.push(p.clone()),
                Request::Full(f) => add(&mut out, f),
            }
        }
        out
    }

    /// Option names the definition can use.
    fn options(&self) -> BTreeSet<String> {
        // Without a login step, `token` is an ordinary option (e.g. Navidrome's token).
        let built_in: &[&str] = if self.login.is_some() {
            &["url", "token", "basic", "item"]
        } else {
            &["url", "basic", "item"]
        };
        let mut options: BTreeSet<String> = self
            .templates()
            .iter()
            .flat_map(|t| placeholders(t).map(String::from).collect::<Vec<_>>())
            .chain(self.needs.iter().cloned())
            .chain(self.defaults.keys().cloned())
            .collect();
        for name in self.computed.keys() {
            options.remove(name);
        }
        if options.contains("basic") {
            options.extend(["username".into(), "password".into()]);
        }
        options.retain(|o| !built_in.contains(&o.as_str()));
        options
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.fields.is_empty() {
            return Err("needs at least one field".into());
        }
        let login = self.login.as_ref();
        if let Some(l) = login
            && [&l.token, &l.cookie, &l.header]
                .iter()
                .filter(|x| x.is_some())
                .count()
                != 1
        {
            return Err("login: set exactly one of token, cookie, header".into());
        }
        let each = self.requests.values().filter_map(|r| match r {
            Request::Full(f) => f.each.as_ref(),
            Request::Get(_) => None,
        });
        let jq = self
            .fields
            .iter()
            .map(|f| &f.value)
            .chain(login.and_then(|l| l.token.as_ref()))
            .chain(login.and_then(|l| l.expired.as_ref()))
            .chain(self.fields.iter().filter_map(|f| f.when.as_ref()))
            .chain(self.computed.values())
            .chain(each);
        for code in jq {
            run_jq(code, jaq_json::Val::Null).map_err(|e| format!("invalid jq {code:?}: {e}"))?;
        }
        Ok(())
    }
}

/// Checks a service's widget against its definition. `Err` means the widget cannot work (its
/// card shows the error); `Ok` lists what is ignored (unknown options and fields, e.g. ones
/// that only exist in gethomepage).
pub fn validate(
    custom: &BTreeMap<String, Definition>,
    w: &WidgetConfig,
    has_service_url: bool,
) -> Result<Vec<String>, String> {
    let def = lookup(custom, &w.kind).ok_or_else(|| {
        // ponytail: shared-prefix match, edit distance if typos slip through
        let wanted = w.kind.to_lowercase();
        let close: Vec<&str> = BUILTIN
            .keys()
            .chain(custom.keys())
            .map(String::as_str)
            .filter(|k| {
                k.len() >= 3 && (wanted.starts_with(&k[..3]) || k.contains(wanted.as_str()))
            })
            .take(5)
            .collect();
        let hint = if close.is_empty() {
            String::new()
        } else {
            format!(" (did you mean {}?)", close.join(", "))
        };
        format!(
            "unknown widget type {:?}{hint}, see docs/widget-types.md",
            w.kind
        )
    })?;
    if w.url.is_none() && !has_service_url && !def.defaults.contains_key("url") {
        return Err(format!(
            "widget {}: set widget.url or the service url",
            w.kind
        ));
    }
    for need in &def.needs {
        if !w.options.contains_key(need) {
            return Err(format!("widget {}: missing option `{need}`", w.kind));
        }
    }
    let mut warnings: Vec<String> = w
        .ignored
        .iter()
        .map(|o| {
            format!(
                "widget {}: option `{o}` ignored (lists and maps are not supported)",
                w.kind
            )
        })
        .collect();
    let known = def.options();
    let list: Vec<&str> = known.iter().map(String::as_str).collect();
    for o in w.options.keys().filter(|o| !known.contains(*o)) {
        warnings.push(format!(
            "widget {}: unknown option `{o}` ignored, expected one of: {}",
            w.kind,
            list.join(", ")
        ));
    }
    let labels: Vec<&str> = def.fields.iter().map(|d| d.label.as_str()).collect();
    for f in w.fields.iter().flatten().filter(|f| def.field(f).is_none()) {
        warnings.push(format!(
            "widget {}: unknown field {f:?} ignored, expected one of: {}",
            w.kind,
            labels.join(", ")
        ));
    }
    Ok(warnings)
}

// ---------- templating ----------

/// Replaces `{name}` with `vars[name]`; `None` if any placeholder is unset.
fn fill(template: &str, vars: &HashMap<String, String>, encode: bool) -> Option<String> {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        let Some(len) = rest[start..].find('}') else {
            break;
        };
        let name = &rest[start + 1..start + len];
        // `{` not followed by a simple name (e.g. inside JSON) is literal.
        if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            out.push_str(&rest[..=start]);
            rest = &rest[start + 1..];
            continue;
        }
        let value = vars.get(name)?;
        out.push_str(&rest[..start]);
        if encode && name != "url" {
            out.push_str(&percent_encode(value));
        } else {
            out.push_str(value);
        }
        rest = &rest[start + len + 1..];
    }
    out.push_str(rest);
    Some(out)
}

fn percent_encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

fn fill_json(v: &serde_json::Value, vars: &HashMap<String, String>) -> Option<serde_json::Value> {
    use serde_json::Value;
    Some(match v {
        Value::String(s) => Value::String(fill(s, vars, false)?),
        Value::Array(a) => Value::Array(
            a.iter()
                .map(|x| fill_json(x, vars))
                .collect::<Option<_>>()?,
        ),
        Value::Object(o) => Value::Object(
            o.iter()
                .filter_map(|(k, x)| Some((k.clone(), fill_json(x, vars)?)))
                .collect(),
        ),
        other => other.clone(),
    })
}

// ---------- jq ----------

/// Runs `code` on `input` and returns its first output. `Err` only for programs that don't
/// compile; a runtime error or no output gives `Ok(None)`. Widget options are `$options`.
fn run_jq(code: &str, input: jaq_json::Val) -> Result<Option<jaq_json::Val>, String> {
    run_jq_with(code, input, &HashMap::new())
}

fn run_jq_with(
    code: &str,
    input: jaq_json::Val,
    options: &HashMap<String, String>,
) -> Result<Option<jaq_json::Val>, String> {
    use jaq_core::{
        Compiler, Ctx, Vars, data,
        load::{Arena, File, Loader},
        unwrap_valr,
    };
    // ponytail: compiles on every run; cache compiled filters if widget count gets large
    let defs = jaq_core::defs()
        .chain(jaq_std::defs())
        .chain(jaq_json::defs());
    let funs = jaq_core::funs()
        .chain(jaq_std::funs())
        .chain(jaq_json::funs());
    let arena = Arena::default();
    let modules = Loader::new(defs)
        .load(&arena, File { code, path: () })
        .map_err(|e| format!("{e:?}"))?;
    let filter = Compiler::default()
        .with_global_vars(["$options"])
        .with_funs(funs)
        .compile(modules)
        .map_err(|e| format!("{e:?}"))?;
    let options = serde_json::to_string(options).unwrap_or_default();
    let options = jaq_json::read::parse_single(options.as_bytes()).unwrap_or_default();
    let ctx = Ctx::<data::JustLut<jaq_json::Val>>::new(&filter.lut, Vars::new([options]));
    let first = filter.id.run((ctx, input)).map(unwrap_valr).next();
    Ok(first.and_then(Result::ok))
}

fn to_json(v: &jaq_json::Val) -> serde_json::Value {
    serde_json::from_str(&v.to_string()).unwrap_or(serde_json::Value::Null)
}

// ---------- formatting ----------

fn group_thousands(n: i64) -> String {
    let digits = n.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push('\u{202f}');
        }
        out.push(c);
    }
    if n < 0 { format!("-{out}") } else { out }
}

fn scaled(mut n: f64, base: f64, units: &[&str]) -> String {
    let mut i = 0;
    while n.abs() >= base && i < units.len() - 1 {
        n /= base;
        i += 1;
    }
    let decimals = usize::from(i > 0 && n.abs() < 100.0);
    format!("{n:.decimals$} {}", units[i])
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn duration(seconds: f64) -> String {
    let s = seconds.max(0.0) as u64;
    let (d, h, m) = (s / 86400, s % 86400 / 3600, s % 3600 / 60);
    if d > 0 {
        format!("{d}d {h}h")
    } else if h > 0 {
        format!("{h}h {m}m")
    } else {
        format!("{m}m")
    }
}

#[allow(clippy::cast_possible_truncation)]
pub fn format(v: &serde_json::Value, f: Format) -> String {
    use serde_json::Value;
    let n = match v {
        Value::Null => return "–".into(),
        Value::String(s) if f == Format::Text => return s.clone(),
        Value::Bool(b) => return if *b { "yes" } else { "no" }.into(),
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse::<f64>().ok(),
        _ => None,
    };
    let Some(n) = n else {
        return match v {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        };
    };
    match f {
        Format::Number | Format::Text if n.fract() == 0.0 && n.abs() < 1e15 => {
            group_thousands(n as i64)
        }
        Format::Number | Format::Text => format!("{n:.1}"),
        Format::Bytes => scaled(n, 1024.0, &["B", "KiB", "MiB", "GiB", "TiB", "PiB"]),
        Format::Rate => format!(
            "{}/s",
            scaled(n, 1024.0, &["B", "KiB", "MiB", "GiB", "TiB"])
        ),
        Format::Bitrate => scaled(n, 1000.0, &["bps", "Kbps", "Mbps", "Gbps", "Tbps"]),
        Format::Percent => format!("{n:.0}%"),
        Format::Duration => duration(n),
        Format::Ago => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0.0, |d| d.as_secs_f64());
            let delta = now - n;
            if delta < 60.0 {
                "just now".into()
            } else {
                format!("{} ago", duration(delta))
            }
        }
    }
}

// ---------- fetching ----------

pub struct Fetcher {
    client: reqwest::Client,
    /// Login tokens per service, reused until the service rejects them.
    tokens: Mutex<HashMap<String, String>>,
}

enum Failure {
    /// Rejected credentials or an expired session: worth logging in again.
    Auth(String),
    Other(String),
}

impl Fetcher {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("http client");
        Fetcher {
            client,
            tokens: Mutex::default(),
        }
    }

    fn build(
        &self,
        r: &FullRequest,
        base_headers: &BTreeMap<String, Header>,
        vars: &HashMap<String, String>,
    ) -> Result<reqwest::RequestBuilder, String> {
        let path =
            fill(&r.path, vars, true).ok_or_else(|| format!("missing option for {}", r.path))?;
        let url = format!("{}{path}", vars["url"]);
        let mut req = match r.method {
            Method::Get => self.client.get(&url),
            Method::Post => self.client.post(&url),
        };
        let query: Vec<(&String, String)> = r
            .query
            .iter()
            .filter_map(|(k, v)| Some((k, fill(v, vars, false)?)))
            .collect();
        if !query.is_empty() {
            req = req.query(&query);
        }
        for (k, h) in base_headers.iter().chain(&r.headers) {
            if let Some(v) = h.alternatives().iter().find_map(|v| fill(v, vars, false)) {
                req = req.header(k, v);
            }
        }
        if let Some(j) = &r.json {
            let body = fill_json(j, vars).ok_or("missing option in request body")?;
            req = req.json(&body);
        }
        if let Some(form) = &r.form {
            let body: BTreeMap<&str, String> = form
                .iter()
                .filter_map(|(k, v)| Some((k.as_str(), fill(v, vars, false)?)))
                .collect();
            req = req.form(&body);
        }
        Ok(req)
    }

    async fn login(
        &self,
        def: &Definition,
        login: &Login,
        vars: &HashMap<String, String>,
    ) -> Result<String, String> {
        let res = self
            .build(&login.request, &def.headers, vars)?
            .send()
            .await
            .map_err(|e| format!("login: {}", without_url(e)))?;
        if let Some(name) = &login.header {
            return header_str(&res, name)
                .ok_or_else(|| format!("login: no {name} header ({})", res.status()));
        }
        if let Some(pattern) = &login.cookie {
            return res
                .headers()
                .get_all(reqwest::header::SET_COOKIE)
                .iter()
                .filter_map(|c| c.to_str().ok()?.split(';').next())
                .find(|pair| {
                    pair.split_once('=')
                        .is_some_and(|(name, _)| glob(pattern, name))
                })
                .map(String::from)
                .ok_or_else(|| format!("login failed ({})", res.status()));
        }
        let status = res.status();
        let body = res
            .bytes()
            .await
            .map_err(|e| format!("login: {}", without_url(e)))?;
        let code = login.token.as_deref().unwrap_or(".");
        // A non-JSON body (plain-text token) is seen by jq as a string.
        let input = jaq_json::read::parse_single(&body).unwrap_or_else(|_| {
            jaq_json::Val::from(String::from_utf8_lossy(&body).trim().to_string())
        });
        let token = run_jq(code, input).ok().flatten().map(|v| to_json(&v));
        match token {
            Some(serde_json::Value::String(t)) => Ok(t),
            Some(serde_json::Value::Number(n)) => Ok(n.to_string()),
            _ => Err(format!("login failed ({status})")),
        }
    }

    /// One request; the body as JSON text (non-JSON bodies become a JSON string).
    async fn send(
        &self,
        name: &str,
        r: &FullRequest,
        def: &Definition,
        vars: &HashMap<String, String>,
    ) -> Result<String, Failure> {
        let res = self
            .build(r, &def.headers, vars)
            .map_err(Failure::Other)?
            .send()
            .await
            .map_err(|e| Failure::Other(without_url(e)))?;
        let status = res.status();
        if matches!(status.as_u16(), 401 | 403 | 409) {
            return Err(Failure::Auth(format!("{name}: HTTP {status}")));
        }
        if !status.is_success() {
            return Err(Failure::Other(format!("{name}: HTTP {status}")));
        }
        if let Some(h) = &r.header {
            let value =
                header_str(&res, h).map_or(serde_json::Value::Null, serde_json::Value::String);
            return Ok(value.to_string());
        }
        let body = res
            .text()
            .await
            .map_err(|e| Failure::Other(without_url(e)))?;
        let json = if serde_json::from_str::<serde::de::IgnoredAny>(&body).is_ok() {
            body
        } else {
            serde_json::Value::String(body).to_string()
        };
        let expired = def.login.as_ref().and_then(|l| l.expired.as_deref());
        if expired.is_some_and(|code| jq_truthy(code, &json, vars)) {
            return Err(Failure::Auth(format!("{name}: session expired")));
        }
        Ok(json)
    }

    /// `send`, except that an optional request that fails gives `null`. Auth failures still
    /// propagate so the login can be retried.
    async fn send_or_null(
        &self,
        name: &str,
        r: &FullRequest,
        def: &Definition,
        vars: &HashMap<String, String>,
        misses: &mut Vec<String>,
    ) -> Result<String, Failure> {
        match self.send(name, r, def, vars).await {
            // An auth failure with a login is kept, so the login is retried; without a login
            // there is nothing to retry and a refused optional request is null too.
            Err(Failure::Other(e)) if r.optional => {
                // An unset path placeholder is a request skipped on purpose (another mode).
                if !e.starts_with("missing option") {
                    misses.push(e);
                }
                Ok("null".into())
            }
            Err(Failure::Auth(e)) if r.optional && def.login.is_none() => {
                misses.push(e);
                Ok("null".into())
            }
            other => other,
        }
    }

    /// All requests: plain ones first, then `each` ones, which can use their responses.
    async fn get_all(
        &self,
        def: &Definition,
        vars: &HashMap<String, String>,
        misses: &mut Vec<String>,
    ) -> Result<String, Failure> {
        let mut parts: Vec<(String, String)> = Vec::new();
        let mut later = Vec::new();
        for (name, r) in &def.requests {
            let r = match r {
                Request::Get(path) => &FullRequest {
                    path: path.clone(),
                    method: Method::Get,
                    query: BTreeMap::new(),
                    json: None,
                    form: None,
                    headers: BTreeMap::new(),
                    header: None,
                    optional: false,
                    each: None,
                },
                Request::Full(f) if f.each.is_some() => {
                    later.push((name, f));
                    continue;
                }
                Request::Full(f) => f,
            };
            parts.push((
                name.clone(),
                self.send_or_null(name, r, def, vars, misses).await?,
            ));
        }
        for (name, r) in later {
            let items = jq_items(r.each.as_deref().unwrap_or("[]"), &combine(&parts), vars);
            let mut results = Vec::with_capacity(items.len());
            for item in items {
                let mut vars = vars.clone();
                vars.insert("item".into(), item);
                results.push(self.send_or_null(name, r, def, &vars, misses).await?);
            }
            parts.push((name.clone(), format!("[{}]", results.join(","))));
        }
        Ok(combine(&parts))
    }

    pub async fn fetch(
        &self,
        key: &str,
        def: &Definition,
        w: &WidgetConfig,
        service_url: Option<&str>,
    ) -> Stats {
        match self.try_fetch(key, def, w, service_url).await {
            Ok(fields) => Stats {
                fields,
                error: None,
            },
            Err(e) => Stats {
                fields: Vec::new(),
                error: Some(e),
            },
        }
    }

    async fn try_fetch(
        &self,
        key: &str,
        def: &Definition,
        w: &WidgetConfig,
        service_url: Option<&str>,
    ) -> Result<Vec<(String, String)>, String> {
        let mut vars: HashMap<String, String> = def
            .defaults
            .clone()
            .into_iter()
            .chain(w.options.clone())
            .collect();
        // A fixed API address (`defaults: {url: https://api.tailscale.com}`) wins over the
        // service url, which then points at the app's web UI.
        let url = w
            .url
            .as_deref()
            .or(def.defaults.get("url").map(String::as_str))
            .or(service_url)
            .ok_or("no url")?;
        vars.insert("url".into(), url.trim_end_matches('/').into());
        if let (Some(u), Some(p)) = (vars.get("username"), vars.get("password")) {
            let basic = BASE64_STANDARD.encode(format!("{u}:{p}"));
            vars.insert("basic".into(), basic);
        }
        for (name, code) in &def.computed {
            if let Some(v) = jq_scalar(code, &vars) {
                vars.insert(name.clone(), v);
            }
        }

        let mut body = Err(Failure::Other(String::new()));
        let mut misses = Vec::new();
        for attempt in 0..2 {
            if let Some(login) = def.login.as_ref().filter(|l| login_possible(l, &vars)) {
                let cached = self.tokens.lock().unwrap().get(key).cloned();
                let token = match cached {
                    Some(t) if attempt == 0 => t,
                    _ => {
                        let t = self.login(def, login, &vars).await?;
                        self.tokens.lock().unwrap().insert(key.into(), t.clone());
                        t
                    }
                };
                vars.insert("token".into(), token);
            }
            misses.clear();
            body = self.get_all(def, &vars, &mut misses).await;
            match &body {
                Err(Failure::Auth(_)) if vars.contains_key("token") && attempt == 0 => {
                    self.tokens.lock().unwrap().remove(key);
                }
                _ => break,
            }
        }
        let body = body.map_err(|(Failure::Auth(e) | Failure::Other(e))| e)?;
        let fields = evaluate(def, w, &body, &vars);
        // Nothing to show and an optional request failed: that failure is the explanation.
        if let Some(e) = misses.first()
            && fields.iter().all(|(_, v)| v == "–")
        {
            return Err(e.clone());
        }
        Ok(fields)
    }
}

/// Computes the fields to show from the combined responses.
fn evaluate(
    def: &Definition,
    w: &WidgetConfig,
    body: &str,
    options: &HashMap<String, String>,
) -> Vec<(String, String)> {
    let input = jaq_json::read::parse_single(body.as_bytes()).unwrap_or_default();
    let wanted: Vec<&Field> = match &w.fields {
        Some(labels) => labels.iter().filter_map(|l| def.field(l)).collect(),
        None => def
            .fields
            .iter()
            .filter(|f| !f.hidden && f.when.as_ref().is_none_or(|c| jq_truthy(c, body, options)))
            .collect(),
    };
    wanted
        .into_iter()
        .map(|f| {
            let v = run_jq_with(&f.value, input.clone(), options)
                .ok()
                .flatten()
                .map(|v| to_json(&v))
                .unwrap_or_default();
            (f.label.clone(), format(&v, f.format))
        })
        .collect()
}

/// `*` matches any run of characters; everything else is literal.
fn glob(pattern: &str, name: &str) -> bool {
    let mut parts = pattern.split('*');
    let first = parts.next().unwrap_or_default();
    let Some(mut rest) = name.strip_prefix(first) else {
        return false;
    };
    let parts: Vec<&str> = parts.collect();
    for (i, part) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            return rest.ends_with(part);
        }
        match rest.find(part) {
            Some(at) => rest = &rest[at + part.len()..],
            None => return false,
        }
    }
    rest.is_empty()
}

fn combine(parts: &[(String, String)]) -> String {
    let fields: Vec<String> = parts
        .iter()
        .map(|(name, json)| format!("{}:{json}", serde_json::Value::String(name.clone())))
        .collect();
    format!("{{{}}}", fields.join(","))
}

/// The login runs only when all its placeholders are set, so auth can be optional.
/// The login runs when its path is complete and, if its query or body use placeholders, at
/// least one of those entries is complete: incomplete entries are left out as in any request,
/// so one login can take alternatives (`{apiKey: "{key}", username: "{username}"…}`), and a
/// login with none of its credentials set is skipped (auth is optional).
fn login_possible(login: &Login, vars: &HashMap<String, String>) -> bool {
    fn leaves(v: &serde_json::Value, out: &mut Vec<String>) {
        match v {
            serde_json::Value::String(s) => out.push(s.clone()),
            serde_json::Value::Array(a) => a.iter().for_each(|x| leaves(x, out)),
            serde_json::Value::Object(o) => o.values().for_each(|x| leaves(x, out)),
            _ => {}
        }
    }
    let complete = |t: &str| placeholders(t).all(|p| p == "url" || vars.contains_key(p));
    let r = &login.request;
    let mut entries: Vec<String> = r.query.values().cloned().collect();
    entries.extend(r.form.iter().flat_map(|f| f.values().cloned()));
    if let Some(j) = &r.json {
        leaves(j, &mut entries);
    }
    let mut with_placeholders = entries.iter().filter(|e| placeholders(e).next().is_some());
    complete(&r.path)
        && (with_placeholders.clone().next().is_none() || with_placeholders.any(|e| complete(e)))
}

fn jq_scalar(code: &str, options: &HashMap<String, String>) -> Option<String> {
    match run_jq_with(code, jaq_json::Val::Null, options)
        .ok()
        .flatten()
        .map(|v| to_json(&v))?
    {
        serde_json::Value::Null => None,
        serde_json::Value::String(s) => Some(s),
        other => Some(other.to_string()),
    }
}

fn jq_truthy(code: &str, json: &str, options: &HashMap<String, String>) -> bool {
    let input = jaq_json::read::parse_single(json.as_bytes()).unwrap_or_default();
    matches!(
        run_jq_with(code, input, options).ok().flatten().map(|v| to_json(&v)),
        Some(v) if !v.is_null() && v != serde_json::Value::Bool(false)
    )
}

/// The elements of the array `code` produces, as placeholder values.
fn jq_items(code: &str, json: &str, options: &HashMap<String, String>) -> Vec<String> {
    let input = jaq_json::read::parse_single(json.as_bytes()).unwrap_or_default();
    let Some(serde_json::Value::Array(items)) = run_jq_with(code, input, options)
        .ok()
        .flatten()
        .map(|v| to_json(&v))
    else {
        return Vec::new();
    };
    items
        .into_iter()
        .map(|i| match i {
            serde_json::Value::String(s) => s,
            other => other.to_string(),
        })
        .collect()
}

fn header_str(res: &reqwest::Response, name: &str) -> Option<String> {
    res.headers().get(name)?.to_str().ok().map(String::from)
}

/// reqwest errors embed the URL, which may carry an API key in its query.
fn without_url(e: reqwest::Error) -> String {
    let e = e.without_url();
    let mut msg = e.to_string();
    let mut source = std::error::Error::source(&e);
    while let Some(s) = source {
        msg = format!("{msg}: {s}");
        source = s.source();
    }
    msg
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn builtins_are_valid() {
        assert!(BUILTIN.len() > 10);
        for (name, def) in BUILTIN.iter() {
            def.validate()
                .unwrap_or_else(|e| panic!("widget {name}: {e}"));
            let labels: BTreeSet<&str> = def.fields.iter().map(|f| f.label.as_str()).collect();
            assert_eq!(
                labels.len(),
                def.fields.len(),
                "widget {name}: duplicate field label"
            );
        }
    }

    #[test]
    fn globbing() {
        assert!(glob("*SID*", "SID") && glob("*SID*", "QBT_SID_8080"));
        assert!(glob("jwt", "jwt") && !glob("jwt", "jwtx") && !glob("SID", "QBT_SID"));
        assert!(glob("a*c", "abbc") && !glob("a*c", "abcd"));
    }

    #[test]
    fn templating() {
        let v = vars(&[("url", "http://x"), ("key", "a b&c")]);
        assert_eq!(
            fill("{url}/api?k={key}", &v, true).unwrap(),
            "http://x/api?k=a%20b%26c"
        );
        assert_eq!(fill("Bearer {key}", &v, false).unwrap(), "Bearer a b&c");
        assert_eq!(
            fill("Basic {basic}", &v, false),
            None,
            "unset placeholder skips the header"
        );
        assert_eq!(fill(r#"{"a": 1}"#, &v, false).unwrap(), r#"{"a": 1}"#);
    }

    #[test]
    fn formats() {
        use serde_json::json;
        assert_eq!(
            format(&json!(1_234_567), Format::Number),
            "1\u{202f}234\u{202f}567"
        );
        assert_eq!(format(&json!(1.25), Format::Number), "1.2");
        assert_eq!(format(&json!(1_610_612_736), Format::Bytes), "1.5 GiB");
        assert_eq!(format(&json!(2048), Format::Rate), "2.0 KiB/s");
        assert_eq!(format(&json!(250_000_000), Format::Bitrate), "250 Mbps");
        assert_eq!(format(&json!(42.4), Format::Percent), "42%");
        assert_eq!(format(&json!(90_061), Format::Duration), "1d 1h");
        assert_eq!(format(&json!(null), Format::Number), "–");
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert_eq!(format(&json!(now - 7200), Format::Ago), "2h 0m ago");
        assert_eq!(format(&json!("ok"), Format::Text), "ok");
    }

    #[test]
    fn login_alternatives() {
        let kavita = &BUILTIN["kavita"];
        let login = kavita.login.as_ref().unwrap();
        let possible = |pairs: &[(&str, &str)]| login_possible(login, &vars(pairs));
        assert!(possible(&[("url", "u"), ("key", "k")]));
        assert!(possible(&[
            ("url", "u"),
            ("username", "a"),
            ("password", "b")
        ]));
        assert!(!possible(&[("url", "u")]), "no credentials: no login");
        let header_only: Login = serde_saphyr::from_str("{path: /rpc, header: X-Id}").unwrap();
        assert!(login_possible(&header_only, &vars(&[("url", "u")])));
        let in_path: Login = serde_saphyr::from_str("{path: '/login/{key}', token: .t}").unwrap();
        assert!(!login_possible(&in_path, &vars(&[("url", "u")])));
        for opts in [
            vec![("key", "k")],
            vec![("username", "a"), ("password", "b")],
        ] {
            let w = WidgetConfig {
                kind: "kavita".into(),
                options: opts
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect(),
                ..Default::default()
            };
            assert_eq!(validate(&BTreeMap::new(), &w, true), Ok(vec![]));
        }
    }

    #[test]
    fn evaluation() {
        let def: Definition = serde_saphyr::from_str(
            r#"
requests: { a: /a, b: /b }
fields:
  - { label: Count, value: ".a | length" }
  - { label: Sum, value: "[.b[].size] | add", format: bytes }
  - { label: Missing, value: ".nope.deeper" }
"#,
        )
        .unwrap();
        def.validate().unwrap();
        let w = WidgetConfig {
            kind: "x".into(),
            ..Default::default()
        };
        let none = HashMap::new();
        let out = evaluate(
            &def,
            &w,
            r#"{"a":[1,2,3],"b":[{"size":1024},{"size":1024}]}"#,
            &none,
        );
        let expected = [("Count", "3"), ("Sum", "2.0 KiB"), ("Missing", "–")];
        assert_eq!(out, expected.map(|(a, b)| (a.to_string(), b.to_string())));

        let only = WidgetConfig {
            fields: Some(vec!["Sum".into()]),
            ..w
        };
        assert_eq!(
            evaluate(&def, &only, r#"{"a":[],"b":[]}"#, &none)[0].0,
            "Sum"
        );
        let opts = vars(&[("site", "home")]);
        assert_eq!(
            jq_items(
                "[.x[] | select(.s == $options.site) | .id]",
                r#"{"x":[{"s":"home","id":7},{"s":"b","id":8}]}"#,
                &opts
            ),
            ["7"]
        );
        assert!(jq_truthy(".error != null", r#"{"error":"expired"}"#, &none));
        assert!(!jq_truthy(".error != null", r#"{"error":null}"#, &none));
        assert_eq!(
            jq_scalar("$options.site + \"-x\"", &opts).as_deref(),
            Some("home-x")
        );
        assert_eq!(
            jq_scalar("now | strftime(\"%Y\") | length", &none).as_deref(),
            Some("4")
        );
    }

    #[test]
    fn rejects_bad_definitions() {
        let bad_jq: Definition =
            serde_saphyr::from_str("requests: {a: /a}\nfields: [{label: X, value: '.a |'}]")
                .unwrap();
        assert!(bad_jq.validate().unwrap_err().contains("invalid jq"));
        let token_no_login: Definition = serde_saphyr::from_str(
            "headers: {X: '{token}'}\nrequests: {a: /a}\nfields: [{label: X, value: .a}]",
        )
        .unwrap();
        assert!(
            token_no_login.options().contains("token"),
            "a plain option without login"
        );
    }

    /// Runs every built-in type against a local server that accepts anything: no definition may
    /// fail to build its requests (an unset placeholder, a bad body) with only `needs` set.
    #[actix_web::test]
    async fn every_builtin_builds_its_requests() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        std::thread::spawn(move || {
            for mut conn in listener.incoming().flatten() {
                let _ = conn.read(&mut [0; 8192]);
                let _ = write!(
                    conn,
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\nset-cookie: SID=x\r\n\
                     x-transmission-session-id: x\r\ncontent-length: 2\r\nconnection: close\r\n\r\n{{}}"
                );
            }
        });
        let fetcher = Fetcher::new();
        let mut broken = Vec::new();
        for (name, def) in BUILTIN.iter() {
            let w = WidgetConfig {
                kind: name.clone(),
                url: Some(url.clone()),
                options: def.needs.iter().map(|n| (n.clone(), "x".into())).collect(),
                ..Default::default()
            };
            // A cached session skips the login, so the requests behind it run too.
            fetcher
                .tokens
                .lock()
                .unwrap()
                .insert(name.clone(), "t".into());
            let stats = fetcher.fetch(name, def, &w, None).await;
            if let Some(e) = stats.error.filter(|e| e.contains("missing option")) {
                broken.push(format!("{name}: {e}"));
            }
        }
        assert!(broken.is_empty(), "{broken:#?}");
    }

    #[test]
    fn hidden_fields() {
        let def: Definition = serde_saphyr::from_str(
            "requests: {a: /a}\nfields: [{label: A, value: .a}, {label: B, value: .a, hidden: true, key: bee}]",
        )
        .unwrap();
        let shown = |fields: Option<Vec<String>>| {
            let w = WidgetConfig {
                fields,
                ..Default::default()
            };
            evaluate(&def, &w, r#"{"a":1}"#, &HashMap::new())
                .into_iter()
                .map(|(l, _)| l)
                .collect::<Vec<_>>()
        };
        assert_eq!(shown(None), ["A"]);
        let modes: Definition = serde_saphyr::from_str(
            "defaults: {mode: a}\nrequests: {a: /a}\nfields: [{label: A, value: .a, when: '$options.mode == \"a\"'}, {label: B, value: .a, when: '$options.mode == \"b\"'}]",
        )
        .unwrap();
        modes.validate().unwrap();
        let in_mode = |mode: &str| {
            let w = WidgetConfig::default();
            evaluate(&modes, &w, r#"{"a":1}"#, &vars(&[("mode", mode)]))
                .into_iter()
                .map(|(l, _)| l)
                .collect::<Vec<_>>()
        };
        assert_eq!(
            (in_mode("a"), in_mode("b")),
            (vec!["A".to_string()], vec!["B".to_string()])
        );
        assert_eq!(shown(Some(vec!["bee".into(), "a".into()])), ["B", "A"]);
    }

    /// `docs/widget-types.md` lists every built-in type; this keeps it in sync.
    /// Regenerate with `UPDATE_DOCS=1 cargo test widget_types_doc`.
    #[test]
    fn widget_types_doc() {
        let code = |s: &str| format!("`{s}`");
        let mut doc = String::from(
            "# Built-in widget types\n\n\
             Generated from [`widgets/*.yaml`](../widgets) by `UPDATE_DOCS=1 cargo test widget_types_doc`, \
             do not edit by hand. How widgets work and how to write new types: \
             [widgets.md](widgets.md).\n\n\
             Every type also accepts `url` (the API address, when it differs from the service url) \
             and `fields` (which fields to show, in order). In `fields`, a field can be named by \
             its label or by its gethomepage key (in parentheses), ignoring case. Fields in \
             _italics_ are only shown when listed in `fields`; the others are shown by default, \
             in this order.\n",
        );
        for src in BUILTIN_SOURCES {
            let title = src
                .lines()
                .next()
                .unwrap_or_default()
                .trim_start_matches("# ")
                .trim_end_matches('.');
            let defs: BTreeMap<String, Definition> = serde_saphyr::from_str(src).unwrap();
            // Comment lines right above a type are its notes (the file header is followed by a blank line).
            let lines: Vec<&str> = src.lines().collect();
            let notes = |name: &str| {
                let at = lines
                    .iter()
                    .position(|l| *l == format!("{name}:"))
                    .unwrap_or(0);
                let mut start = at;
                while start > 0 && lines[start - 1].starts_with('#') {
                    start -= 1;
                }
                if start == 0 {
                    start = at;
                }
                // Plus trailing comments on the definition's own keys (`needs: [key]  # …`).
                let own = lines[at + 1..]
                    .iter()
                    .take_while(|l| l.is_empty() || l.starts_with(' '))
                    .filter(|l| l.starts_with("  ") && !l.starts_with("   "))
                    .filter_map(|l| l.split_once("  # ").map(|(_, c)| c));
                let header: Vec<&str> = lines[start..at]
                    .iter()
                    .map(|l| l.trim_start_matches(['#', ' ']))
                    .collect();
                let mut text = vec![header.join(" ")];
                text.extend(own.map(String::from));
                text.retain(|t| !t.is_empty());
                text.join(" · ").replace('|', "\\|")
            };
            doc += &format!(
                "\n## {title}\n\n| Type | Required options | Optional options | Fields | Notes |\n|---|---|---|---|---|\n"
            );
            for (name, def) in &defs {
                let optional: Vec<String> = def
                    .options()
                    .into_iter()
                    .filter(|o| !def.needs.contains(o))
                    .map(|o| match def.defaults.get(&o) {
                        Some(d) => format!("`{o}` (default `{d:?}`)"),
                        None => code(&o),
                    })
                    .collect();
                let needs: Vec<String> = def.needs.iter().map(|n| code(n)).collect();
                // `Label` plus its gethomepage key when it differs; hidden ones in italics.
                let fields: Vec<String> = def
                    .fields
                    .iter()
                    .map(|f| {
                        let mut out = f.label.clone();
                        if let Some(k) = f
                            .key
                            .as_deref()
                            .filter(|k| !k.eq_ignore_ascii_case(&f.label))
                        {
                            out += &format!(" (`{k}`)");
                        }
                        if f.hidden { format!("_{out}_") } else { out }
                    })
                    .collect();
                doc += &format!(
                    "| `{name}` | {} | {} | {} | {} |\n",
                    needs.join(", "),
                    optional.join(", "),
                    fields.join(", "),
                    notes(name)
                );
            }
        }
        let path = "docs/widget-types.md";
        if std::env::var_os("UPDATE_DOCS").is_some() {
            std::fs::write(path, &doc).unwrap();
        }
        assert_eq!(
            std::fs::read_to_string(path).unwrap_or_default(),
            doc,
            "{path} is stale, run: UPDATE_DOCS=1 cargo test widget_types_doc"
        );
    }
}
