# Homepage-rs: feature list

A homelab dashboard in Rust inspired by [gethomepage.dev](https://gethomepage.dev), but cleaner, more customizable and more complete.

Legend: **[H]** = exists in Homepage · **[+]** = new / improvement · **P0/P1/P2** = priority (P0 = MVP)

---

## 1. Technical foundation

| Component | Proposed choice | Why |
|---|---|---|
| HTTP server | `axum` + `tokio` | standard, fast, tower middleware |
| Rendering | SSR (`askama` or `maud`) + `htmx` + SSE | no JS build, page rendered in <50 ms, live updates without an SPA |
| HTTP client | `reqwest` (rustls) | calls to service APIs |
| Config | `serde` + `serde_yaml` / TOML | typed, precise errors |
| Docker | `bollard` | Docker / Podman API |
| Kubernetes | `kube-rs` | discovery + metrics |
| System | `sysinfo` | CPU/RAM/disk/network/temperatures |
| Storage | `sqlite` (`rusqlite`/`sqlx`) | history, uptime, state |
| Plugins | `wasmtime` (WASI) | sandboxed third-party widgets |
| Distribution | single static binary (assets embedded via `rust-embed`) + multi-arch distroless Docker image | |

---

## 2. Application core

### Configuration — P0
- [H] YAML files: services, widgets, bookmarks, settings
- [+] A single format of your choice: YAML **or** TOML
- [+] Validation at startup with precise errors (file, line, column, "did you mean…" suggestion)
- [+] Automatically generated JSON schema → autocompletion in VS Code
- [+] `homepage check` command (validates the config without starting the server, useful in CI)
- [H] Config hot reload (`notify` watcher), no restart
- [H] `{{HOMEPAGE_VAR_X}}` variable substitution
- [+] Secrets from a file (`_FILE`), Docker secrets, Vault, sops/age
- [+] Includes / fragments (`!include services/media.yaml`) to split the config
- [+] Reusable templates / anchors (e.g. same auth for 5 services)

### Layout & UI — P0/P1
- [H] Service groups, columns, rows, collapsible groups
- [H] Tabs
- [+] **Multiple pages** (`/media`, `/infra`, `/network`) with dedicated routes
- [+] Free-form Grafana-style grid (x/y/w/h positions), S/M/L/XL widget sizes
- [+] **Visual editor** (drag & drop, in-place editing) that rewrites the config — optional, config-as-code remains the source of truth
- [+] Responsive layout specific to mobile / tablet / TV / kiosk
- [+] Kiosk mode (fullscreen, automatic page rotation)
- [H] Icons: dashboard-icons, Material Design, Simple Icons, custom URL
- [+] selfh.st icons, emojis, Lucide; local icon cache (no external requests at runtime)
- [H] Custom background image, blur, opacity
- [+] Dynamic backgrounds (Unsplash, local folder, based on time of day/weather)

### Themes & customization — P1
- [H] Light/dark, color palettes
- [+] Full theme system via CSS variables (`theme.toml` file)
- [+] Automatic system theme tracking + scheduled switching (day/night)
- [+] Injected custom CSS and JS (already in H, but with hot reload)
- [+] Ready-made community themes: Catppuccin, Nord, Dracula, Gruvbox, Tokyo Night, Solarized
- [+] Custom fonts (self-hosted)
- [+] Per-card customization: color, border, transparency, compact/detailed style

### Navigation & search — P1
- [H] Web search bar (Google, DDG, Bing, SearXNG, custom)
- [H] Quick launch (type to filter services)
- [+] **Command palette** `Ctrl+K`: services, bookmarks, actions, web search, commands (restart container…)
- [+] Configurable keyboard shortcuts, 100% keyboard navigation
- [+] DuckDuckGo-style bangs (`!gh`, `!yt`, custom bangs)
- [+] Fuzzy search on name, description, tags

### Bookmarks — P0
- [H] Link groups with icon/abbreviation
- [+] Import from the browser (Netscape HTML) / Linkwarden / Linkding
- [+] Tags, favorites, most-used links first (local click counter)

### Auto-discovery — P1
- [H] Docker labels (`homepage.group`, `homepage.name`…)
- [H] Kubernetes (Ingress annotations, Traefik IngressRoute, HTTPRoute)
- [+] Podman, Docker Swarm, multi-host Docker (socket, TCP+TLS, SSH)
- [+] Proxmox (auto VMs/LXC), Traefik/Caddy/NPM (routes → automatic services)
- [+] Nomad, Consul
- [+] Smart merge of discovery + manual config (override by name)

### Service status — P0
- [H] Ping / siteMonitor (HTTP)
- [H] Docker container / K8s pod status
- [+] TCP, ICMP, DNS, TLS (certificate expiry), gRPC health, custom command checks
- [+] Expected HTTP codes, regex on the body, configurable timeouts
- [+] **History** (SQLite): latency sparkline, 24h/7d/30d uptime %
- [+] Optional public status page (Uptime Kuma-style, read-only)

### Alerting & notifications — P2
- [+] Alerts on service down, expiring cert, full disk, update available
- [+] Channels: ntfy, Gotify, Discord, Slack, Telegram, Matrix, email (SMTP), generic webhook, Apprise
- [+] Maintenance windows, anti-flapping, grouping

### Security & multi-user — P1
- [+] Built-in authentication: local (argon2), **OIDC** (Authelia, Authentik, Keycloak, Pocket ID), forward-auth (`Remote-User` / `Remote-Groups` headers)
- [+] Per-user/group visibility (groups, services, widgets, pages)
- [+] Personal dashboards per user
- [+] API keys are **never** sent to the browser (everything goes through the backend) — Homepage does this too, we keep it strict
- [+] Allowed hosts whitelist (`allowedHosts`), strict CSP, security headers
- [+] Protected actions (restart container, wake-on-LAN) behind a role + confirmation
- [+] Action audit log

### Performance & robustness — P0
- [+] Per-widget cache with configurable TTL, background refresh (stale-while-revalidate)
- [+] Live updates via SSE instead of client-side polling
- [+] Timeouts, retry with backoff, per-service circuit breaker (a slow service doesn't block the page)
- [+] Startup < 100 ms, RAM < 20 MB
- [+] `/healthz`, `/metrics` (Prometheus) endpoints to monitor the dashboard itself

### Extensibility — P2
- [+] **Generic API widget** (H's custom API, improved): URL, auth, JSONPath/JMESPath, field mapping, formatting (bytes, %, duration, date), color thresholds
- [+] Generic Prometheus widget (PromQL query → value/graph)
- [+] Generic HTML/iframe/Markdown widget
- [+] "Shell command" widget (command output, read-only, opt-in)
- [+] **WASM plugins**: sandboxed third-party widgets, no recompiling
- [+] REST/JSON API to read state (Home Assistant integration, scripts)
- [+] i18n (fr, en, de, es… via Fluent files)
- [+] Installable PWA, offline mode (last known value)

---

## 3. Information widgets (header / global)

| Widget | Details | Prio |
|---|---|---|
| Date / time | formats, multiple time zones | P0 |
| Greeting | message based on time of day, user name | P0 |
| Logo | custom logo | P0 |
| Search | see navigation section | P0 |
| System resources | CPU, RAM, disk, uptime, temperature, network, load | P0 |
| Weather | Open-Meteo (no key), OpenWeatherMap | P0 |
| Glances (info) | resources of a remote host | P1 |
| Kubernetes (info) | cluster and node CPU/RAM | P1 |
| Longhorn | volumes | P2 |
| UniFi console | network stats | P1 |
| Stocks / crypto | Finnhub, CoinGecko | P2 |
| **[+] Calendar** | iCal/CalDAV (Nextcloud, Google), today's agenda | P1 |
| **[+] Todo** | CalDAV VTODO, Vikunja, Todoist | P2 |
| **[+] RSS/Atom feed** | article list with cache | P1 |
| **[+] Notes / Markdown** | free-form text block | P1 |
| **[+] Countdown** | events, key dates | P2 |
| **[+] Sunrise/sunset, moon** | local calculation | P2 |
| **[+] Air quality / pollen** | Open-Meteo | P2 |
| **[+] Prometheus graph** | free-form query | P1 |
| **[+] Public IP / VPN** | WAN IP, VPN status, geolocation | P1 |
| **[+] Local speedtest** | throughput history | P2 |
| **[+] TLS certificates** | days remaining per domain | P1 |
| **[+] Updates** | Diun / WUD / outdated Docker images | P1 |
| **[+] Wake-on-LAN** | per-machine button | P2 |
| **[+] Now playing** | Spotify, Jellyfin, Plex "now playing" | P2 |
| **[+] Iframe / embed** | Grafana, camera, other | P1 |
| **[+] Camera** | MJPEG/RTSP snapshot via Frigate/go2rtc | P2 |

---

## 4. Service widgets (API integrations)

Homepage has ~150. Full list below, sorted by category. **[+]** items don't exist (or only partially) in Homepage.

### Media — servers & playback
Jellyfin, Emby, Plex, Tautulli, Jellystat, Navidrome, Audiobookshelf, Komga, Kavita, Calibre-web, Stash, Immich, PhotoPrism, Owncast, Channels DVR, Tdarr, Unmanic, [+] Lyrion/LMS, [+] Funkwhale, [+] Pinchflat, [+] Tube Archivist

### Media — *arr & requests
Sonarr, Radarr, Lidarr, Readarr, Bazarr, Prowlarr, Jackett, Mylar, Whisparr, Overseerr, Jellyseerr, Ombi, Autobrr, Kometa, [+] Huntarr, [+] Recyclarr (sync status), [+] Maintainerr, [+] Cleanuparr, [+] Profilarr

### Downloads
qBittorrent, Transmission, Deluge, rTorrent/ruTorrent, Flood, NZBGet, SABnzbd, Pyload, JDownloader, Aria2, [+] Unpackerr

### Network & DNS
Pi-hole, AdGuard Home, Technitium DNS, NextDNS, Blocky, UniFi, Omada, OPNsense, pfSense, MikroTik, OpenWRT, Fritz!Box, Tailscale, Headscale, NetAlertX, Gluetun, WireGuard (wg-easy), [+] Netbird, [+] CrowdSec, [+] Zerotier, [+] Firewalla, [+] Unbound

### Reverse proxy & security
Traefik, Caddy, Nginx Proxy Manager, Authentik, Authelia (status), Cloudflare Tunnels, CrowdSec, Fail2ban (via API), Vaultwarden (status), [+] Zoraxy, [+] Pangolin, [+] Pocket ID, [+] Keycloak

### Infra & virtualization
Proxmox, Proxmox Backup Server, TrueNAS, Unraid, OpenMediaVault, Synology (Disk Station, Download Station), QNAP, Portainer, Dockge, Komodo, Kubernetes, Docker (container stats), Nomad, ESXi, XCP-ng / Xen Orchestra, Glances, Netdata, Beszel, Scrutiny, Uptime Kuma, Gatus, Healthchecks, Uptime Robot, [+] Incus/LXD, [+] Harvester, [+] Coolify, [+] Dokploy, [+] Ceph, [+] native ZFS (zpool status)

### Monitoring & observability
Prometheus, Grafana, Uptime Kuma, Gatus, Healthchecks.io, Netdata, Glances, Beszel, [+] Loki, [+] Alertmanager (active alerts), [+] VictoriaMetrics, [+] InfluxDB, [+] Zabbix, [+] Checkmk, [+] ClickHouse/ClickStack

### Backups
Proxmox Backup Server, Duplicati, [+] Restic (REST server), [+] Borg/Borgmatic, [+] Kopia, [+] Backrest, [+] Velero, [+] Urbackup

### Home automation & IoT
Home Assistant, ESPHome, Homebridge, Frigate, Zigbee2MQTT, Node-RED, Scrypted, [+] Generic MQTT, [+] Shelly, [+] Tasmota, [+] Hubitat, [+] Domoticz, [+] OpenHAB, [+] Victron/Enphase (solar energy), [+] NUT/PeaNUT UPS, [+] Tesla/Teslamate

### Productivity & documents
Nextcloud, Paperless-ngx, Mealie, Tandoor, Grocy, Vikunja, Linkwarden, Wallabag, Miniflux, FreshRSS, Kavita, Gitea, Forgejo, GitLab, Gitlab CI, Gotify, ntfy, Changedetection.io, [+] Karakeep (Hoarder), [+] Linkding, [+] Outline, [+] BookStack, [+] Wiki.js, [+] Immich, [+] Actual Budget, [+] Firefly III, [+] Ghostfolio, [+] Stirling PDF, [+] Seafile

### Development & CI
Gitea, Forgejo, GitLab, GitHub (via API), Jenkins, Drone/Woodpecker, ArgoCD, [+] Harbor, [+] Renovate, [+] Sentry, [+] SonarQube

### Games
Minecraft, GameDig (CS, Valheim, Ark…), Pterodactyl, Crafty Controller, [+] AMP, [+] Romm, [+] Steam (stats)

### Miscellaneous
Speedtest Tracker, MySpeed, PeaNUT, NUT, Moonraker/Klipper, OctoPrint, PrusaLink, Calendar, Evcc, Mailcow, Stalwart, [+] Syncthing, [+] Ollama / Open WebUI (loaded models), [+] LiteLLM, [+] Frigate stats, [+] Docker registry

### Generic widgets (replace 80% of the missing integrations)
- **Custom API** (JSON) + field mapping — P0
- **Prometheus / PromQL** — P1
- **Ping / HTTP / TCP / DNS / TLS** — P0
- **iframe / Markdown / HTML** — P1
- **WASM plugin** — P2

---

## 5. Proposed roadmap

1. **MVP (P0)**: axum server, validated YAML config + hot reload, groups/services/bookmarks, HTTP/ping status, system resources, date/time, weather, search, custom API widget, light/dark theme, Docker image.
2. **v0.2**: Docker/K8s discovery, ~20 most-used widgets (Jellyfin, Plex, *arr, qBittorrent, Pi-hole, AdGuard, Proxmox, TrueNAS, Home Assistant, Uptime Kuma, Immich, Nextcloud, Traefik, Portainer…), SQLite history, command palette.
3. **v0.3**: OIDC/forward-auth + per-group visibility, multi-page, themes, calendar/RSS, Prometheus.
4. **v0.4**: alerting, visual editor, public status page.
5. **v1.0**: WASM plugins, the remaining integrations, i18n, PWA.
