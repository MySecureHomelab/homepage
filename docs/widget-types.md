# Built-in widget types

Generated from [`widgets/*.yaml`](../widgets) by `UPDATE_DOCS=1 cargo test widget_types_doc`, do not edit by hand. How widgets work and how to write new types: [widgets.md](widgets.md).

Every type also accepts `url` (the API address, when it differs from the service url) and `fields` (which fields to show, in order). Fields are listed in their default display order.

## Media servers, libraries, photos and transcoding

| Type | Required options | Optional options | Fields | Notes |
|---|---|---|---|---|
| `audiobookshelf` | `key` |  | Podcasts, Podcast Duration, Books, Book Duration |  |
| `calibreweb` | `username`, `password` |  | Books, Authors, Categories, Series |  |
| `channelsdvrserver` |  |  | Shows, Recordings, Scheduled, Passes |  |
| `emby` | `key` |  | Playing, Movies, Series, Episodes, Songs |  |
| `fileflows` | `key` |  | Queue, Processing, Processed, Failed | FileFlows >= 26.10 via the "Fenrus Dashboard Endpoint" (enable it in settings; its token is the key). |
| `immich` | `key` |  | Users, Photos, Videos, Storage | Immich >= v1.118 (needs an API key with the server.statistics permission). |
| `jellyfin` | `key` |  | Playing, Movies, Series, Episodes, Songs, Albums |  |
| `jellystat` | `key` | `days` (default `"30"`) | Songs, Movies, Episodes, Other | Views over the last `days` days. |
| `kavita` | `key` |  | Series, Files |  |
| `komga` |  | `key`, `password`, `username` | Libraries, Series, Books | Basic auth (username + password) or API key. |
| `navidrome` | `username`, `password` |  | Now Playing | Subsonic API: the password goes in the query string (token/salt auth cannot be computed here). |
| `photoprism` |  | `key`, `password`, `username` | Albums, Photos, Videos, People | Authenticates with an app password / access token (`key`) or username + password. |
| `plex` | `key` |  | Active Streams, Albums, Movies, TV Shows |  |
| `romm` |  |  | Platforms, Games, Saves, States, Screenshots, Total Size |  |
| `stash` |  | `key` | Scenes, Scenes Played, Total Plays, Time Watched, Scenes Size, Scenes Duration, Images, Images Size, Galleries, Performers, Studios, Movies, Tags, O Count | `key` is optional (only when Stash has login credentials). |
| `tautulli` | `key` |  | Playing, Transcoding, Bitrate | Tautulli takes its key in the query string. |
| `tdarr` |  | `key` | Queue, Processed, Errored, Saved | `key` is optional. |
| `tubearchivist` | `key` |  | Queue, Videos, Channels, Playlists |  |
| `unmanic` |  |  | Active Workers, Total Workers, Queue Length |  |

## *arr stack and request managers

| Type | Required options | Optional options | Fields | Notes |
|---|---|---|---|---|
| `autobrr` | `key` |  | Approved, Rejected, Filters, Indexers |  |
| `bazarr` | `key` |  | Missing Episodes, Missing Movies |  |
| `jackett` | `key` | `password` | Configured, Errored | The key must stay in the query string. With an admin password set, a session cookie is used. |
| `jellyseerr` | `key` |  | Pending, Approved, Available, Completed, Processing, Open Issues |  |
| `lidarr` | `key` |  | Wanted, Queued, Artists |  |
| `maintainerr` |  |  | Items Handled, Episodes Handled, Movies Handled, Shows Handled, Seasons Handled, Reclaimable, Movie Reclaimable, Show Reclaimable, Total Capacity | Maintainerr has no authentication. |
| `medusa` | `key` |  | Wanted, Queued, Series | Medusa puts the key in the path. |
| `mylar` | `key` |  | Series, Issues, Wanted | Mylar only accepts the key in the query string. |
| `ombi` | `key` |  | Pending, Approved, Available |  |
| `overseerr` | `key` |  | Pending, Processing, Approved, Available |  |
| `prowlarr` | `key` |  | Grabs, Queries, Fail Grabs, Fail Queries |  |
| `radarr` | `key` |  | Wanted, Missing, Queued, Movies |  |
| `readarr` | `key` |  | Wanted, Queued, Books |  |
| `seerr` | `key` |  | Pending, Approved, Available, Completed, Processing, Open Issues |  |
| `sonarr` | `key` |  | Wanted, Queued, Series |  |
| `sportarr` | `key` |  | Wanted, Queued, Leagues |  |

## Download clients (torrent, usenet, direct download)

| Type | Required options | Optional options | Fields | Notes |
|---|---|---|---|---|
| `deluge` | `password` |  | Leech, Download, Seed, Upload |  |
| `downloadstation` | `username`, `password` |  | Leech, Download, Seed, Upload | Synology DSM web API; login uses API version 6 (DSM 6 and 7). |
| `flood` |  | `password`, `username` | Leech, Download, Seed, Upload | Without username/password the login is skipped (no-auth mode). |
| `nzbget` |  | `password`, `username` | Rate, Remaining, Downloaded |  |
| `pyload` |  | `key`, `password`, `username` | Speed, Active, Queue, Total | pyLoad-ng (0.5+) only: API key or basic auth; legacy session login is not supported. |
| `qbittorrent` |  | `key`, `password`, `username` | Leech, Download, Seed, Upload |  |
| `qui` | `key` | `instance` (default `""`) | Leech, Download, Seed, Upload, Total, Errored, Ratio, Free | All qBittorrent instances, or one when `instance` (its id in qui) is set. |
| `rutorrent` |  | `password`, `username` | Active, Upload, Download | Rows of the httprpc "list" answer are arrays: 3 = state, 11 = up rate, 12 = down rate. |
| `sabnzbd` | `key` |  | Rate, Queue, Time Left | kbpersec is the same speed as the human readable `speed`, in KiB/s. |
| `slskd` | `key` |  | Network, Downloads, Uploads, Files |  |
| `transmission` |  | `password`, `username` | Leech, Download, Seed, Upload |  |

## DNS filters, routers and firewalls, VPN and reverse proxies

| Type | Required options | Optional options | Fields | Notes |
|---|---|---|---|---|
| `adguard` |  | `password`, `username` | Queries, Blocked, Filtered, Latency |  |
| `authentik` | `key` |  | Users, Logins (24h), Failed Logins (24h), Auths (24h) | authentik API token (`key`), events volume API (authentik 2025.x+). |
| `caddy` |  |  | Upstreams, Current requests, Failed requests | Caddy admin API (default port 2019). |
| `cloudflared` | `key`, `accountid`, `tunnelid` |  | Status, Origin IP | Cloudflare Tunnel via the Cloudflare API: set `url: https://api.cloudflare.com`. `key` is an API token (Account > Cloudflare Tunnel > Read), plus `accountid` and `tunnelid`. |
| `crowdsec` | `username`, `password` |  | Alerts, Bans | CrowdSec local API: the machine `username` / `password` (cscli machines add). |
| `gluetun` |  | `key` | Public IP, Region, Country | Gluetun control server (port 8000). `key` is optional (only if the control server has auth). The forwarded port is left out: its endpoint errors when port forwarding is off. |
| `headscale` | `key`, `nodeId` |  | Name, Address, Last Seen, Status | Headscale API key and node id (numeric id of a node). |
| `mikrotik` | `username`, `password` |  | Uptime, CPU Load, Memory Used, Leases | RouterOS 7 REST API (/rest) with an admin `username` / `password`. RouterOS returns numbers as strings. |
| `netalertx` | `key` |  | Total, Connected, New Devices, Down Alerts | NetAlertX v2 API (`key` is the API token). |
| `netalertx1` | `key` |  | Total, Connected, New Devices, Down Alerts | NetAlertX legacy (PHP) API. |
| `netbird` | `key` |  | Online Peers, Users, Login Expired | NetBird management API with a personal access token (`key`). |
| `nextdns` | `key`, `profile` |  | Default, Blocked, Allowed, Relayed | Set the widget url to https://api.nextdns.io. `profile` is the profile ID. |
| `npm` | `username`, `password` |  | Enabled, Disabled, Total | Nginx Proxy Manager: an admin `username` (email) / `password`. |
| `openwrt` | `username`, `password` |  | Uptime, CPU Load Avg (5m) | OpenWrt over ubus (rpcd, needs uhttpd-mod-ubus and an rpcd user): system stats. |
| `openwrt_interface` | `username`, `password`, `interfaceName` |  | Status, Transmitted, Received | OpenWrt network interface counters; `interfaceName` is e.g. wan or eth0. |
| `opnsense` | `username`, `password` | `wan` (default `"wan"`) | CPU Load, Active Memory, WAN Upload, WAN Download | OPNsense: `username` / `password` are the API key / secret. `wan` is the interface to show (default wan). |
| `pangolin` | `key`, `org` |  | Sites, Resources, Targets, Traffic, In, Out | Pangolin integration API: `key` is an API key, `org` the organization id. |
| `pfsense` |  | `key`, `password`, `username`, `wan` (default `"wan"`) | Load Avg, Mem Usage, Temp, WAN Status, WAN IP, Disk Usage | pfSense with the pfSense REST API package v2: an API key (`key`) or admin `username` / `password`. `wan` is the interface to show, by name or hardware id such as igb0 (default wan). |
| `pihole` | `key` |  | Queries, Blocked, Blocked %, Gravity | Pi-hole v6 (session login with the web password as `key`). |
| `pihole5` | `key` |  | Queries, Blocked, Blocked %, Gravity | Pi-hole v5 (legacy API, `key` is the API token). The key can only go in the query string. |
| `tailscale` | `key`, `deviceid` |  | Address, Last Seen, Expires, User, Hostname, OS, Client Version, Update Available | Tailscale API (https://api.tailscale.com, so set `url: https://api.tailscale.com`). `key` is an API access token, `deviceid` the device (node) ID. |
| `technitium` | `key` | `node`, `range` (default `"LastHour"`) | Queries, Success, Failures, NX Domains, Refused, Authoritative, Recursive, Cached, Blocked, Dropped, Clients | `key` is an API token; optional `range` (LastHour, LastDay...) and cluster `node`. |
| `traefik` |  | `password`, `username` | Routers, Services, Middleware | Traefik API (api@internal). `username` / `password` only if it sits behind basic auth. |
| `unifi` | `key` | `site` (default `"default"`) | Uptime, WAN, LAN Users, WLAN Users | UniFi OS console (UDM, Cloud Key...) with an API key. Uses the site named `site` (default "default"; name or description). Older standalone controllers (login + CSRF) are not supported. |
| `wgeasy` | `password` | `threshold` (default `"2"`) | Connected, Enabled, Disabled, Total | wg-easy up to v14: `password` is the web UI password. "Connected" = handshake within `threshold` minutes (default 2). |
| `wgeasy2` | `username`, `password` | `threshold` (default `"2"`) | Connected, Enabled, Disabled, Total | wg-easy v15+: `username` / `password` of an admin, same `threshold`. |

## Infrastructure: hypervisors, NAS, container managers, monitoring and backup tools

| Type | Required options | Optional options | Fields | Notes |
|---|---|---|---|---|
| `arcane` | `env`, `key` |  | Running, Stopped, Total, Image Updates | env = Arcane environment id |
| `argocd` | `key` |  | Apps, Synced, Out Of Sync, Healthy |  |
| `backrest` |  | `password`, `username` | Plans, Succeeding, Failing, Successes, Failures, Bytes Added |  |
| `beszel` | `username`, `password` | `systemId` (default `""`) | Systems, Up, Down, CPU, Memory, Disk, Network | system id or name; empty = all systems |
| `checkmk` | `site`, `username`, `password` |  | Service issues, Host issues | site = monitoring site name; automation user |
| `diskstation` | `username`, `password` | `volume` (default `""`) | Uptime, CPU, Memory, Available | volume name (e.g. volume_1); empty = first volume |
| `dockhand` |  | `environment` (default `""`), `key` | Running, Total, CPU, Memory | Authenticates with an API key when set (no session login). · environment name or id; empty = all environments |
| `duplicati` | `password` |  | Jobs, Errors, Last Run, Next Run |  |
| `gatus` |  | `password`, `username` | Sites Up, Sites Down, Uptime |  |
| `glances` |  | `fs` (default `"/"`), `password`, `username`, `version` (default `"4"`) | CPU, Mem, Swap, Load, Disk Used, Temp | Glances API version (3 or 4); mount point for Disk Used |
| `grafana` |  | `alerts` (default `"grafana"`), `password`, `username` | Dashboards, Data Sources, Total Alerts, Alerts Triggered | Needs an account allowed to read /api/admin/stats (Grafana admin). · alertmanager to count: grafana or alertmanager |
| `healthchecks` | `key` | `uuid` (default `""`) | Up, Down, Status, Last Ping | check uuid for Status / Last Ping; empty = all checks |
| `komodo` | `key`, `secret` |  | Total, Running, Stopped, Unhealthy, Servers, Stacks |  |
| `kopia` |  | `password`, `snapshotHost` (default `""`), `snapshotPath` (default `""`), `username` | Status, Size, Last Run, Next Run | pick a snapshot source; empty = first one |
| `netdata` |  | `password`, `username` | Warnings, Criticals |  |
| `openmediavault` | `username`, `password` |  | Running, Stopped, Total |  |
| `peanut` | `key` | `password`, `username` | Battery Charge, UPS Load, UPS Status | UPS device name in PeaNUT |
| `portainer` | `env`, `key` |  | Running, Stopped, Total | env = Portainer environment (endpoint) id |
| `prometheus` |  | `password`, `username` | Targets Up, Targets Down, Total Targets |  |
| `proxmox` | `username`, `password` | `node` (default `""`) | VMs, LXC, CPU, MEM | API token: username = user@realm!tokenid, password = secret · limit to one node (name); empty = whole cluster |
| `proxmoxbackupserver` | `username`, `password` | `datastore` (default `""`) | Datastore, Failed Tasks 24h, CPU, Memory | API token: username = user@realm!tokenid, password = secret · limit the usage to one datastore; empty = all |
| `pulse` | `key` |  | Nodes, VMs, LXCs |  |
| `scrutiny` |  | `password`, `username` | Passed, Failed, Unknown |  |
| `truenas` |  | `key`, `password`, `pool` (default `""`), `username` | System Load, Uptime, Alerts, Pool Used, Pool Health | REST API; authenticate with an API key or with username + password. · limit the pool fields to one pool; empty = all pools |
| `unraid` | `key` | `pool` (default `"cache"`) | Status, CPU, Memory Used, Notifications, Array Used, Array Free, Pool Used, Pool Free | name of the pool shown by Pool Used / Pool Free |
| `uptimekuma` | `slug` | `password`, `username` | Sites Up, Sites Down, Uptime, Incident | status page slug |
| `uptimerobot` | `key` | `monitor` (default `""`) | Sites Up, Sites Down, Status, Uptime, Last Downtime, Downtime Duration | monitor id(s), dash separated; empty = all monitors |
| `watchtower` | `key` |  | Scanned, Updated, Failed |  |
| `whatsupdocker` |  | `key`, `password`, `username` | Monitoring, Updates | Authenticate with an API key (Bearer) or with username + password. |
| `zabbix` |  | `key`, `password`, `username` | Not classified, Information, Warning, Average, High, Disaster | API token (Bearer) or username + password. |

## Self-hosted apps: home automation, documents, recipes, dev tools, notes, 3D printing, energy

| Type | Required options | Optional options | Fields | Notes |
|---|---|---|---|---|
| `azuredevops` | `organization`, `project`, `definitionId`, `branchName`, `username`, `password` | `repositoryId` (default `""`), `userEmail` (default `""`) | Result, Total PRs, My PRs, Approved | Set the widget url to https://dev.azure.com. password: a personal access token; username: any value (it is ignored). Latest build of a pipeline and branch (branchName like refs/heads/main). Pull request stats need repositoryId and userEmail. |
| `booklore` | `username`, `password` |  | Libraries, Books, Reading, Finished |  |
| `changedetectionio` | `key` |  | Diffs Detected, Total Observed |  |
| `esphome` |  | `key`, `password`, `username` | Online, Offline, Unknown, Total | Auth is optional: username + password, or key (the "authenticated" cookie value). |
| `evcc` |  |  | Production, Grid, Consumption, Charger | Kilowatts, as in the gethomepage widget (grid and charger are clamped at 0). |
| `filebrowser` | `username`, `password` |  | Available, Used, Total |  |
| `firefly` | `key` |  | Net Worth, Budget | Current month. Net worth is the first currency reported by Firefly III. |
| `freshrss` | `username`, `password` |  | Unread, Subscriptions |  |
| `frigate` |  |  | Cameras, Uptime, Version | Unauthenticated Frigate API (port 5000). |
| `ghostfolio` | `key` |  | Today, One year, All time, Net Worth | key: the Ghostfolio security token's JWT (Bearer). |
| `gitea` | `key` |  | Notifications, Issues, Pull Requests, Repositories |  |
| `gitlab` | `key`, `user_id` |  | Groups, Issues, Merge Requests, Projects |  |
| `gotify` | `key` |  | Applications, Clients, Messages | key: a client token. Messages are counted over the latest 200. |
| `homeassistant` | `key` |  | People Home, Lights On, Switches On |  |
| `homebox` | `username`, `password` |  | Items, Locations, Total Value |  |
| `homebridge` | `username`, `password` |  | Status, Updates, Child Bridges |  |
| `karakeep` | `key` |  | Bookmarks, Favorites, Archived, Highlights |  |
| `linkwarden` | `key` |  | Links, Collections, Tags |  |
| `lubelogger` |  | `password`, `username` | Vehicles, Service Records, Reminders | Basic auth is optional (username + password). |
| `mastodon` |  |  | Users, Posts, Domains |  |
| `mealie` | `key` |  | Recipes, Users, Categories, Tags |  |
| `miniflux` | `key` |  | Unread, Read |  |
| `moonraker` |  | `key` | Layers, Progress, Print Status | key: Moonraker API key, only if authorization is enabled. |
| `myspeed` |  | `password` | Download, Upload, Ping | password: the MySpeed password, if one is set. Results are in Mbps. |
| `nextcloud` |  | `key`, `password`, `username` | Free Space, Active Users, Files, Shared Items |  |
| `ntfy` | `topic` | `key`, `password`, `username` | Title, Message, Priority, Last Received, Tags | Shows the latest message of a topic. Protected topics: key (access token) or username + password. |
| `octoprint` | `key` |  | Status, Tool temp, Bed temp, Completion |  |
| `opendtu` |  | `password`, `username` | Today, Power %, Power, Limit | Basic auth is optional (username + password). |
| `paperlessngx` |  | `key`, `password`, `username` | Inbox, Total | key: an API token, or username + password. |
| `pterodactyl` | `key` |  | Nodes, Servers |  |
| `speedtest` |  | `key` | Download, Upload, Ping | Speedtest Tracker (alexjustesen). key: API token. Results are in bytes per second. |
| `spoolman` |  |  | Spools, Remaining, Lowest | Summary of all spools (the per-spool blocks of gethomepage need dynamic labels). |
| `syncthing` | `key` |  | Connected, Synced, Errors, Storage |  |
| `tandoor` | `key` |  | Users, Recipes, Keywords |  |
| `trilium` | `key` |  | Version, Notes, Database Size | key: an ETAPI token. |
| `vikunja` | `key` |  | Active Projects, Tasks Due This Week, Overdue Tasks, Tasks In Progress | Vikunja 0.24+ (tasks endpoint); looks at the first 500 open tasks. Due dates are compared as UTC timestamps. |
| `wallos` | `key` |  | Subscriptions, Next Payment, Prev. Month, This Month, Next Month |  |
| `yourspotify` | `key` | `interval` (default `"week"`) | Songs, Time, Artists | interval: day, week (default), month (30 days), year or all. |
