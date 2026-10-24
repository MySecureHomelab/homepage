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
#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct WidgetConfig {
    #[serde(rename = "type")]
    pub kind: String,
    /// Defaults to the service URL.
    pub url: Option<String>,
    /// Labels to show, in order. Defaults to all fields of the definition.
    pub fields: Option<Vec<String>>,
    /// `key`, `username`, `password`, and whatever else the definition uses.
    #[serde(flatten)]
    pub options: BTreeMap<String, String>,
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

fn placeholders(s: &str) -> impl Iterator<Item = &str> {
    s.split('{')
        .skip(1)
        .filter_map(|p| p.split_once('}').map(|(name, _)| name))
}

impl Definition {
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
        let built_in = ["url", "token", "basic", "item"];
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
        if login.is_none()
            && self
                .templates()
                .iter()
                .any(|t| placeholders(t).any(|p| p == "token"))
        {
            return Err("{token} is only available with a login step".into());
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
            .chain(self.computed.values())
            .chain(each);
        for code in jq {
            run_jq(code, jaq_json::Val::Null).map_err(|e| format!("invalid jq {code:?}: {e}"))?;
        }
        Ok(())
    }
}

/// Checks a service's widget against its definition.
pub fn validate(
    custom: &BTreeMap<String, Definition>,
    w: &WidgetConfig,
    has_service_url: bool,
) -> Result<(), String> {
    let def = lookup(custom, &w.kind).ok_or_else(|| {
        let known: Vec<&str> = BUILTIN
            .keys()
            .chain(custom.keys())
            .map(String::as_str)
            .collect();
        format!(
            "unknown widget type {:?}, expected one of: {}",
            w.kind,
            known.join(", ")
        )
    })?;
    if w.url.is_none() && !has_service_url {
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
    let known = def.options();
    for o in w.options.keys() {
        if !known.contains(o) {
            let list: Vec<&str> = known.iter().map(String::as_str).collect();
            return Err(format!(
                "widget {}: unknown option `{o}`, expected one of: {}",
                w.kind,
                list.join(", ")
            ));
        }
    }
    if let Some(fields) = &w.fields {
        for f in fields {
            if !def.fields.iter().any(|d| &d.label == f) {
                let labels: Vec<&str> = def.fields.iter().map(|d| d.label.as_str()).collect();
                return Err(format!(
                    "widget {}: unknown field {f:?}, expected one of: {}",
                    w.kind,
                    labels.join(", ")
                ));
            }
        }
    }
    Ok(())
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
    ) -> Result<String, Failure> {
        match self.send(name, r, def, vars).await {
            Err(Failure::Other(_)) if r.optional => Ok("null".into()),
            other => other,
        }
    }

    /// All requests: plain ones first, then `each` ones, which can use their responses.
    async fn get_all(
        &self,
        def: &Definition,
        vars: &HashMap<String, String>,
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
            parts.push((name.clone(), self.send_or_null(name, r, def, vars).await?));
        }
        for (name, r) in later {
            let items = jq_items(r.each.as_deref().unwrap_or("[]"), &combine(&parts), vars);
            let mut results = Vec::with_capacity(items.len());
            for item in items {
                let mut vars = vars.clone();
                vars.insert("item".into(), item);
                results.push(self.send_or_null(name, r, def, &vars).await?);
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
        let url = w.url.as_deref().or(service_url).ok_or("no url")?;
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
            body = self.get_all(def, &vars).await;
            match &body {
                Err(Failure::Auth(_)) if vars.contains_key("token") && attempt == 0 => {
                    self.tokens.lock().unwrap().remove(key);
                }
                _ => break,
            }
        }
        let body = body.map_err(|(Failure::Auth(e) | Failure::Other(e))| e)?;
        Ok(evaluate(def, w, &body, &vars))
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
        Some(labels) => labels
            .iter()
            .filter_map(|l| def.fields.iter().find(|f| &f.label == l))
            .collect(),
        None => def.fields.iter().collect(),
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
fn login_possible(login: &Login, vars: &HashMap<String, String>) -> bool {
    let r = &login.request;
    let mut templates = vec![r.path.clone()];
    templates.extend(r.query.values().cloned());
    templates.extend(r.form.iter().flat_map(|f| f.values().cloned()));
    templates.extend(r.json.iter().map(ToString::to_string));
    templates
        .iter()
        .flat_map(|t| placeholders(t).map(String::from).collect::<Vec<_>>())
        .all(|p| {
            p == "url" || p == "basic" && vars.contains_key("username") || vars.contains_key(&p)
        })
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
            url: None,
            fields: None,
            options: BTreeMap::new(),
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
        assert!(token_no_login.validate().is_err());
    }
}
