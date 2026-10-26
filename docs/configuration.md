# Configuration

All configuration lives in a single YAML file (`config.yaml` by default, or the path in
`HOMEPAGE_CONFIG`). A full example is in [`config.example.yaml`](../config.example.yaml).

- [Top-level keys](#top-level-keys)
- [Services](#services)
- [Status checks](#status-checks)
- [Icons](#icons)
- [Bookmarks](#bookmarks)
- [Header widgets](#header-widgets)
- [Service widgets](#service-widgets)
- [Secrets and variables](#secrets-and-variables)
- [Hot reload and errors](#hot-reload-and-errors)
- [Docker and Kubernetes discovery](#docker-and-kubernetes-discovery)
- [UI and keyboard shortcuts](#ui-and-keyboard-shortcuts)
- [Command line and environment variables](#command-line-and-environment-variables)
- [HTTP endpoints](#http-endpoints)
- [Security](#security)
- [Known limitations](#known-limitations)

## Top-level keys

Every key is optional: an empty file is a valid, empty page. Unknown keys (typos included) are
an error, reported with their line.

| Key | Default | Values |
|---|---|---|
| `title` | `Homepage` | Page and browser tab title. |
| `theme` | `auto` | `auto` (follows the OS), `light`, `dark`. |
| `accent` | `"#7c6cf2"` | Accent color, `"#rrggbb"` only. Quote it: `#` starts a YAML comment. |
| `search` | `duckduckgo` | `duckduckgo`, `google`, `bing`, `startpage`, or a URL containing `{q}` (e.g. `https://searx.lan/search?q={q}`). Used by the search bar and the command palette. |
| `check_interval` | `30` | Seconds between rounds of status checks and service widget refreshes. At least 1. |
| `widgets` | `[clock, search, resources]` | Header widgets to show, in this order. `[]` shows none. |
| `groups` | `[]` | Service groups, see [Services](#services). |
| `bookmarks` | `[]` | Link groups, see [Bookmarks](#bookmarks). |
| `docker` | `{}` | Docker hosts to discover services from, by name. See [Discovery](#docker-and-kubernetes-discovery). |
| `kubernetes` | `false` | `true` to discover services from Kubernetes Ingresses. |
| `widget_types` | `{}` | Your own widget types (or overrides of built-in ones). See [widgets.md](widgets.md#writing-a-widget-type). |

## Services

```yaml
groups:
  - name: Media              # required
    services:
      - name: Jellyfin        # required, unique within the group
        url: https://jellyfin.lan
        icon: jellyfin
        description: Movies & shows
        check: https://jellyfin.lan/health
        widget:
          type: jellyfin
          key: ${JELLYFIN_API_KEY}
```

| Key | Purpose |
|---|---|
| `name` | Shown on the card. Together with the group name it identifies the service (widgets, merging with discovery): keep it unique within a group. |
| `url` | Makes the card a link. Also the default address of the widget. Without a `url`, the card is not a link and is not in the command palette. |
| `icon` | See [Icons](#icons). |
| `description` | Card subtitle. |
| `check` | See [Status checks](#status-checks). No check, no status dot. |
| `widget` | Live stats, see [Service widgets](#service-widgets). |

Groups are shown in file order, and so are their services. Groups created by discovery come
after them.

## Status checks

| Form | Up when | Latency shown |
|---|---|---|
| `http://…`, `https://…` | The server answers with anything but a 5xx within 5 s. A 401, 403 or 404 counts as up: the service is there, it just wants auth. Redirects are followed. | yes |
| `tcp://host:port` | A TCP connection opens within 5 s. | yes (connect time) |
| `docker://<host>/<container>` | The container exists, is running and is not `unhealthy`. `<host>` is a name declared under `docker:`, `<container>` the container name (no leading `/`). | no |

- Checks run every `check_interval` seconds, in parallel, and right after a config reload or a
  change in discovered services.
- Services with the same check share one result (a single request).
- TLS certificates are verified. An `https` service with a self-signed certificate shows as
  down: use a `tcp://` check or an internal `http://` URL instead.
- `docker://` checks update at the discovery pace (15 s). When the Docker host stops answering,
  its containers are down.
- Invalid values (`tcp://host` without a port, `docker://` to an undeclared host, another
  scheme) are a config error.

The dot is green (up), red (down) or grey (not checked yet). Hovering it shows the state and the
latency.

## Icons

- A bare name (`jellyfin`, `proxmox`…) points to the
  [dashboard-icons](https://github.com/homarr-labs/dashboard-icons) SVG, through the jsDelivr CDN:
  `https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/<name>.svg`.
- Any value containing a `/` is used as the image URL as is (`https://…/logo.png`). Homepage
  does not serve static files: use a full URL.
- An icon that fails to load is removed from the card, no broken image.

Icons are loaded by the browser, from the CDN or the given URL.

## Bookmarks

```yaml
bookmarks:
  - name: Dev                 # required
    links:
      - name: GitHub          # required
        url: https://github.com   # required
        icon: github          # optional, same rules as for services
```

Bookmark groups are shown after the services. They are also in the command palette.

## Header widgets

| Widget | Shows |
|---|---|
| `clock` | Time (hours and minutes) and long date, in the **browser's** time zone and language. |
| `search` | Web search bar using the `search` engine. `/` focuses it, `Esc` leaves it. |
| `resources` | CPU, memory, disk and uptime of the **machine running Homepage**, refreshed every 5 s. Bars turn orange above 75 % and red above 90 %. |

For `resources`, the disk is the filesystem mounted on `/`. In a container, that is the
container's filesystem (in practice the host disk that stores the images); CPU, memory and
uptime are the host's.

## Service widgets

A widget shows stats from the service's API on its card:

```yaml
widget:
  type: sonarr                # required, see the list
  key: ${SONARR_API_KEY}      # options of the type (key, username, password…)
  url: http://sonarr:8989     # optional: API address when it differs from the service url
  fields: [Wanted, Queued]    # optional: fields to show, in this order (default: all)
```

- **140 built-in types**, with their options and fields: [widget-types.md](widget-types.md).
- Writing your own types (requests, auth, jq): [widgets.md](widgets.md).

Options are checked on load: an unknown type, a missing required option, an unknown option or a
field that does not exist are errors, listing the valid values. A widget needs a `url` (its own
or the service's).

How it works:

- Requests are made by the server: API keys never reach the browser.
- Refreshed every `check_interval` seconds, 10 s timeout per request.
- Sessions (login) are kept in memory and reused until the service rejects them; Homepage then
  logs in again, once.
- On failure the card shows the error (`stats: HTTP 401 Unauthorized`, `login failed`…) instead
  of the values. A missing value shows as `–`.
- As for checks, TLS certificates are verified.

## Secrets and variables

Anywhere in the file, before the YAML is parsed:

- `${NAME}` is replaced by the environment variable `NAME`.
- `${file:/path}` is replaced by the file contents, without leading and trailing whitespace
  (handy for Docker secrets: `${file:/run/secrets/sonarr}`).

```yaml
widget:
  type: sonarr
  key: ${SONARR_API_KEY}
  password: ${file:/run/secrets/sonarr_password}
```

- An unset variable or an unreadable file is an error, with the line number.
- YAML comments are not substituted: a commented-out `${…}` does no harm.
- The value is inserted into the YAML as is. If a secret contains special characters (`: `,
  ` #`, a leading quote…), quote the reference: `key: "${MY_KEY}"`.
- There is no escape: a literal `${` cannot be written in the config.

Substitution runs on every reload, but a process's environment does not change: restart to pick
up a new environment value. Files are read again on every reload.

## Hot reload and errors

- The file's modification time is checked on every page load, every `/api/state` call and every
  check round: edits apply without a restart.
- If the new config is invalid, the page shows the error (file, line, message) in a banner and
  **the last valid config keeps being served**. The error is also written to stderr.
- At startup, an invalid config stops the process (exit code 1).
- New services are checked right away, without waiting for `check_interval`.

The CSS (accent color) and the content update when the page is reloaded. Statuses, widgets and
resources update on their own every 5 s.

## Docker and Kubernetes discovery

Services can come from Docker labels and Kubernetes Ingress annotations. The names are the
gethomepage.dev ones, so an existing setup works as is. Discovery runs every 15 s, and right
after a config change.

```yaml
docker:
  local: unix:///var/run/docker.sock
  nas: tcp://nas.lan:2375        # http:// works too
kubernetes: true
```

The host name (`local`, `nas`) must not contain `/`; it is the one used in
`docker://<host>/<container>`. All containers are listed, stopped ones included.

### Docker labels

| Label | Purpose |
|---|---|
| `homepage.name` | **Required** for the container to show up. |
| `homepage.group` | Group (default `Docker`). |
| `homepage.href` or `homepage.url` | Card URL. |
| `homepage.icon` | Icon. |
| `homepage.description` | Description. |
| `homepage.check` | Check. Defaults to `docker://<host>/<container>`. |
| `homepage.widget.type` | Widget type. |
| `homepage.widget.url` | Widget API URL. |
| `homepage.widget.fields` | Fields, as a JSON list (`["Wanted","Queued"]`) or comma-separated (`Wanted, Queued`). |
| `homepage.widget.<option>` | Any other widget option: `homepage.widget.key`, `homepage.widget.username`… |

```yaml
services:
  sonarr:
    image: lscr.io/linuxserver/sonarr
    labels:
      homepage.name: Sonarr
      homepage.group: Media
      homepage.href: https://sonarr.lan
      homepage.icon: sonarr
      homepage.widget.type: sonarr
      homepage.widget.url: http://sonarr:8989
      homepage.widget.key: ${SONARR_API_KEY}   # substituted by docker compose, not by Homepage
```

Labels are readable by anyone who can list containers: prefer setting secrets in the config file
when that matters.

### Kubernetes annotations

Only `networking.k8s.io/v1` `Ingress` objects are read, in all namespaces.

| Annotation | Purpose |
|---|---|
| `gethomepage.dev/enabled: "true"` | **Required** for the Ingress to show up. |
| `gethomepage.dev/name` | Name (default: the Ingress name). |
| `gethomepage.dev/group` | Group (default `Kubernetes`). |
| `gethomepage.dev/href` | URL. Defaults to the host and path of the first rule, `https` when a TLS block covers that host. |
| `gethomepage.dev/icon`, `gethomepage.dev/description` | Icon, description. |
| `gethomepage.dev/check` | Check. Defaults to the service URL. |
| `gethomepage.dev/widget.type`, `gethomepage.dev/widget.<option>` | Widget, as for Docker. |

Cluster access: in-cluster config (the pod's service account) or, outside a cluster,
`~/.kube/config` / `KUBECONFIG`. Required permission: `list` on `ingresses` in the
`networking.k8s.io` group (full example in [deployment.md](deployment.md#kubernetes)).

### Merging with the config

- A discovered service goes into the group it names; the group is added at the end when the
  config does not have it.
- A service declared in the config **with the same name in the same group** wins: the
  discovered one is ignored.
- Discovered services are sorted by group, then by name.
- An invalid discovered check or widget is dropped (the service stays on the page), with a
  message in the logs.
- When a Docker host or the cluster stops answering, its services stay on the page with their
  last known state and the containers turn down. Errors are logged once, not on every round.

## UI and keyboard shortcuts

| Key | Action |
|---|---|
| `/` | Focus the search bar (when the `search` widget is shown). |
| `Esc` | Leave the search bar, close the palette. |
| `Ctrl+K` / `⌘K` | Open (or close) the command palette. |
| `↑` `↓` | Move through palette results. |
| `Enter` | Open the result. |
| `Ctrl+Enter` / `⌘Enter` (or Ctrl/⌘+click) | Open in a new tab. |

The palette fuzzy-matches service and bookmark names (and, with less weight, their group name),
shows the 8 best results, then a "Search the web" row that sends the query to the `search`
engine.

When the server stops answering, the page keeps the last values on screen.

## Command line and environment variables

| | |
|---|---|
| `homepage` | Starts the server. |
| `homepage check` | Loads and validates the config, prints `<path>: OK` and exits (code 0), or prints the error (code 1). Useful in CI or before a deployment. |
| `HOMEPAGE_CONFIG` | Config path. Default `config.yaml` (relative to the working directory), `/config/config.yaml` in the Docker image. |
| `HOMEPAGE_ADDR` | Listen address. Default `0.0.0.0:3000`. |
| `KUBECONFIG` | Kubeconfig to use outside a cluster. |

## HTTP endpoints

| Path | Content |
|---|---|
| `/` | The dashboard (server-rendered HTML). |
| `/app.css`, `/app.js` | Page styles and script. |
| `/api/state` | Live state as JSON, see below. |
| `/healthz` | Returns `ok` (200). For health probes. |

`/api/state`:

```json
{
  "status": {
    "https://jellyfin.lan/health": { "up": true, "ms": 42 },
    "docker://nas/sonarr": { "up": false, "ms": null }
  },
  "widgets": {
    "Media/Sonarr": { "fields": [["Wanted", "3"], ["Queued", "1"]], "error": null }
  },
  "resources": {
    "cpu": 12.5, "mem_used": 4294967296, "mem_total": 17179869184,
    "disk_used": 53687091200, "disk_total": 107374182400, "uptime": 86400
  }
}
```

`status` is keyed by check, `widgets` by `<group>/<service>`; widget values are already
formatted. `resources` is `null` right after startup. Bytes are raw, `uptime` is in seconds.

## Security

- **No built-in authentication.** Anyone who reaches the port sees the dashboard and
  `/api/state` (statuses, check URLs, widget stats). Do not expose it to the Internet without a
  reverse proxy that authenticates (Authelia, Authentik, oauth2-proxy…), see
  [deployment.md](deployment.md#reverse-proxy-and-authentication).
- API keys and passwords stay on the server: neither the page nor `/api/state` contains them.
- Values returned by third-party APIs are inserted as text, never as HTML.
- Headers sent on every response: `Content-Security-Policy` (everything from the server itself,
  except images), `X-Content-Type-Options: nosniff`, `Referrer-Policy: no-referrer`. The page
  cannot be embedded in an iframe.
- The Docker image runs as non-root (uid 65532), on a distroless base without a shell.

## Known limitations

- Self-signed TLS certificates are rejected (checks and widgets).
- A single config file, no includes.
- `resources` only shows the `/` disk.
- No multiple pages, tabs, weather or authentication yet: see the roadmap in
  [FEATURES.md](../FEATURES.md).
