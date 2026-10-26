# Built-in widget types

Generated from [`widgets/*.yaml`](../widgets) by `UPDATE_DOCS=1 cargo test widget_types_doc`, do not edit by hand. How widgets work and how to write new types: [widgets.md](widgets.md).

Every type also accepts `url` (the API address, when it differs from the service url) and `fields` (which fields to show, in order). In `fields`, a field can be named by its label or by its gethomepage key (in parentheses), ignoring case. Fields in _italics_ are only shown when listed in `fields`; the others are shown by default, in this order.

## Media servers, libraries, photos and transcoding

| Type | Required options | Optional options | Fields | Notes |
|---|---|---|---|---|
| `audiobookshelf` | `key` |  | Podcasts, Podcast Duration (`podcastsDuration`), Books, Book Duration (`booksDuration`) |  |
| `calibreweb` |  | `password`, `username` | Books, Authors, Categories, Series | username + password (Basic auth), unless Calibre-Web allows anonymous browsing. |
| `channelsdvrserver` |  |  | Shows, Recordings, Scheduled, Passes |  |
| `emby` | `key` |  | _Movies_, _Series_, _Episodes_, _Songs_, Playing | Count fields are hidden unless listed in `fields` (gethomepage: enableBlocks). |
| `fileflows` |  | `key`, `version` (default `"1"`) | Queue, Processing, Processed, Time, Failed | `version: 2` for FileFlows >= 26.10: uses the "Fenrus Dashboard Endpoint" (enable it; its token is the key). |
| `immich` | `key` | `version` (default `"1"`) | Users, Photos, Videos, Storage | `version: 2` for Immich >= v1.118 (API key with the server.statistics permission). |
| `jellyfin` | `key` | `deviceId` (default `"homepage"`), `version` (default `"1"`) | _Movies_, _Series_, _Episodes_, _Songs_, _Albums_, Playing | Count fields are hidden unless listed in `fields` (gethomepage: enableBlocks). Works with Jellyfin < and >= 10.12 whatever `version` is (the un-prefixed endpoints exist in both). |
| `jellystat` | `key` | `days` (default `"30"`) | Songs, Movies, Episodes, Other | Views over the last `days` days. |
| `kavita` |  | `key`, `password`, `username` | Series (`seriesCount`), Files (`totalFiles`) | key (API key), or username + password. |
| `komga` |  | `key`, `password`, `username`, `version` (default `"1"`) | Libraries, Series, Books | Basic auth (username + password) or API key. `version: 2` for Komga >= v2. |
| `navidrome` | `user` | `password`, `salt`, `token` | Now Playing | Subsonic API: `user` + `token` (md5(password + salt)) + `salt`, or `user` + `password`. |
| `photoprism` |  | `key`, `password`, `username` | Albums, Photos, Videos, People | Authenticates with an app password / access token (`key`) or username + password. |
| `plex` | `key` |  | Active Streams (`streams`), Albums, Movies, TV Shows (`tv`) |  |
| `romm` |  | `password`, `username` | Platforms, Games (`totalRoms`), Saves, States, _Screenshots_, _Total Size (`totalfilesize`)_ | username + password (Basic auth) if RomM requires a login. |
| `stash` |  | `key` | Scenes, _Scenes Played (`scenesPlayed`)_, _Total Plays (`playCount`)_, _Time Watched (`playDuration`)_, _Scenes Size (`sceneSize`)_, _Scenes Duration (`sceneDuration`)_, Images, _Images Size (`imageSize`)_, _Galleries_, _Performers_, _Studios_, _Movies_, _Tags_, _O Count (`oCount`)_ | `key` is optional (only when Stash has login credentials). |
| `tautulli` | `key` |  | Playing, Transcoding, Bitrate | Tautulli takes its key in the query string. |
| `tdarr` |  | `key` | Queue, Processed, Errored, Saved | `key` is optional. |
| `tubearchivist` | `key` |  | Queue (`downloads`), Videos, Channels, Playlists |  |
| `unmanic` |  |  | Active Workers (`active_workers`), Total Workers (`total_workers`), Queue Length (`records_total`) |  |

## *arr stack and request managers

| Type | Required options | Optional options | Fields | Notes |
|---|---|---|---|---|
| `autobrr` | `key` |  | Approved (`approvedPushes`), Rejected (`rejectedPushes`), Filters, Indexers |  |
| `bazarr` | `key` |  | Missing Episodes (`missingEpisodes`), Missing Movies (`missingMovies`) |  |
| `jackett` |  | `key`, `password` | Configured, Errored | Set password when Jackett has an admin password (session cookie); key is optional. |
| `jellyseerr` | `key` |  | Pending, Approved, _Available_, Completed, _Processing_, _Open Issues (`issues`)_ | Jellyseerr is now Seerr: same API, same fields as seerr. |
| `lidarr` | `key` |  | Wanted, Queued, Artists |  |
| `maintainerr` |  |  | Items Handled (`itemsHandled`), Episodes Handled (`episodesHandled`), Movies Handled (`moviesHandled`), _Shows Handled (`showsHandled`)_, _Seasons Handled (`seasonsHandled`)_, Reclaimable, _Movie Reclaimable (`movieReclaimable`)_, _Show Reclaimable (`showReclaimable`)_, _Total Capacity (`totalCapacity`)_ | Maintainerr has no authentication. |
| `medusa` | `key` |  | Wanted, Queued, Series | Medusa puts the key in the path. |
| `mylar` | `key` |  | Series, Issues, Wanted | Mylar only accepts the key in the query string. |
| `ombi` | `key` |  | Pending, Approved, Available |  |
| `overseerr` | `key` |  | Pending, Processing, Approved, Available |  |
| `prowlarr` | `key` |  | Grabs (`numberOfGrabs`), Queries (`numberOfQueries`), Fail Grabs (`numberOfFailGrabs`), Fail Queries (`numberOfFailQueries`) |  |
| `radarr` | `key` |  | Wanted, Missing, Queued, Movies |  |
| `readarr` | `key` |  | Wanted, Queued, Books |  |
| `seerr` | `key` |  | Pending, Approved, _Available_, Completed, _Processing_, _Open Issues (`issues`)_ |  |
| `sonarr` | `key` |  | Wanted, Queued, Series |  |
| `sportarr` | `key` |  | Wanted, Queued, Leagues |  |

## Download clients (torrent, usenet, direct download)

| Type | Required options | Optional options | Fields | Notes |
|---|---|---|---|---|
| `deluge` | `password` |  | Leech, Download, Seed, Upload |  |
| `downloadstation` | `username`, `password` |  | Leech, Download, Seed, Upload | Synology DSM web API; login uses API version 6 (DSM 6 and 7). Not compatible with 2FA. The task list is asked on both DownloadStation/task.cgi and entry.cgi; the one that answers is used. |
| `flood` |  | `password` (default `""`), `username` (default `""`) | Leech, Download, Seed, Upload | username/password only if Flood auth is enabled (auth=none still needs the session cookie). |
| `nzbget` |  | `password`, `username` | Rate, Remaining, Downloaded | ControlUsername / ControlPassword (basic auth). |
| `pyload` |  | `key`, `password`, `username` | Speed, Active, Queue, Total | pyLoad-ng (0.5+) only: API key, or username/password (basic auth). Legacy session login is not supported. |
| `qbittorrent` |  | `key`, `password`, `username` | Leech, Download, Seed, Upload | Username/password, or API key (`key`, qBittorrent 5.2+). |
| `qui` | `key` | `instance` (default `""`) | Leech, Download, Seed, Upload, _Total_, _Errored_, _Ratio_, _Free (`freeSpace`)_ | All qBittorrent instances, or one when `instance` (its id in qui) is set. |
| `rutorrent` |  | `password`, `username` | Active, Upload, Download | Rows of the httprpc "list" answer are arrays: 3 = state, 11 = up rate, 12 = down rate. Needs the httprpc plugin. username/password optional (basic auth). |
| `sabnzbd` | `key` |  | Rate, Queue, Time Left (`timeleft`) |  |
| `slskd` | `key` |  | Network (`slskStatus`), _Update (`updateStatus`)_, Downloads, Uploads, Files (`sharedFiles`) |  |
| `transmission` |  | `password`, `username` | Leech, Download, Seed, Upload | Fixed RPC path /transmission/rpc: a custom `rpcUrl` is not supported. |

## DNS filters, routers and firewalls, VPN and reverse proxies

| Type | Required options | Optional options | Fields | Notes |
|---|---|---|---|---|
| `adguard` |  | `password`, `username` | Queries, Blocked, Filtered, Latency | `username` / `password` of the web interface. |
| `authentik` | `key` | `version` (default `"1"`) | Users, Logins (24h) (`loginsLast24H`), Failed Logins (24h) (`failedLoginsLast24H`), Auths (24h) (`authorizationsLast24H`) | authentik API token (`key`). `version: 2` for authentik 2025.8+ (default 1, as in gethomepage); Auths (24h) needs version 2. |
| `caddy` |  |  | Upstreams, Current requests (`requests`), Failed requests (`requests_failed`) | Caddy admin API (default port 2019). |
| `cloudflared` | `key`, `accountid`, `tunnelid` |  | Status, Origin IP (`origin_ip`) | Cloudflare Tunnel via the Cloudflare API (https://api.cloudflare.com, no url needed). `key` is an API token (Account > Cloudflare Tunnel > Read), plus `accountid` and `tunnelid`. |
| `crowdsec` | `username`, `password` |  | Alerts, Bans | CrowdSec local API: the machine `username` / `password` (cscli machines add). |
| `gluetun` |  | `key`, `version` (default `"1"`) | Public IP (`public_ip`), Region, Country, _Port Forwarded (`port_forwarded`)_ | Gluetun control server (port 8000). `key` is optional (only if the control server has auth). `version: 2` for Gluetun 3.40.1+ (default 1). Port Forwarded needs its route allowed in the auth config. |
| `headscale` | `key`, `nodeId` |  | Name, Address, Last Seen (`last_seen`), Status | Headscale API key and node id (numeric id of a node). |
| `mikrotik` | `username`, `password` |  | Uptime, CPU Load (`cpuLoad`), Memory Used (`memoryUsed`), Leases (`numberOfLeases`) | RouterOS 7 REST API (/rest) with an admin `username` / `password`. RouterOS returns numbers as strings. |
| `netalertx` | `key` | `version` (default `"1"`) | Total, Connected, New Devices (`new_devices`), Down Alerts (`down_alerts`) | NetAlertX: `key` is the API_TOKEN. `version: 2` for NetAlertX v26.1.17+ (default 1, as in gethomepage); version 2 uses the backend port. |
| `netalertx1` | `key` |  | Total, Connected, New Devices (`new_devices`), Down Alerts (`down_alerts`) | NetAlertX legacy (PHP) API. Same as `netalertx` with version 1. |
| `netbird` | `key` |  | Online Peers (`online`), Users, _Login Expired (`login_expired`)_ | NetBird management API with a personal access token (`key`). |
| `nextdns` | `key`, `profile` |  | Default, Blocked, Allowed, Relayed | NextDNS API (https://api.nextdns.io, no url needed). `profile` is the profile ID. |
| `npm` | `username`, `password` |  | Enabled, Disabled, Total | Nginx Proxy Manager: an admin `username` (email) / `password`. |
| `openwrt` | `username`, `password` | `interfaceName` | Uptime, CPU Load Avg (5m) (`cpuLoad`), Status, Transmitted (`bytesTx`), Received (`bytesRx`) | OpenWrt over ubus (rpcd, needs uhttpd-mod-ubus and an rpcd user). Without `interfaceName`: system stats; with it (e.g. eth0): that device's status and counters. |
| `openwrt_interface` | `username`, `password`, `interfaceName` |  | Status, Transmitted (`bytesTx`), Received (`bytesRx`) | OpenWrt network interface counters; `interfaceName` is e.g. wan or eth0. Same as `openwrt` with interfaceName. |
| `opnsense` | `username`, `password` | `wan` (default `"wan"`) | CPU Load (`cpu`), Active Memory (`memory`), WAN Upload (`wanUpload`), WAN Download (`wanDownload`) | OPNsense: `username` / `password` are the API key / secret. `wan` is the interface to show (default wan). |
| `pangolin` | `key`, `org` |  | Sites, Resources, Targets, Traffic, _In_, _Out_ | Pangolin integration API: `key` is an API key, `org` the organization id. |
| `pfsense` |  | `key`, `password`, `username`, `version` (default `"1"`), `wan` (default `"wan"`) | Load Avg (`load`), Mem Usage (`memory`), Temp, WAN Status (`wanStatus`), _WAN IP (`wanIP`)_, _Disk Usage (`disk`)_ | pfSense with the pfSense REST API package: `version: 2` for API v2 (default 1, as in gethomepage). Auth: admin `username` / `password`, or an API `key` (v2, sent as X-API-Key; gethomepage's `headers:` map is not supported). `wan` is the interface to show, by hardware id such as igb0 or name (default wan). |
| `pihole` |  | `key`, `version` (default `"5"`) | Queries, Blocked, _Blocked % (`blocked_percent`)_, Gravity | Pi-hole: set `version: 6` for Pi-hole v6 (default 5, as in gethomepage). `key` is optional: the API token on v5, the web or app password on v6. |
| `pihole5` |  | `key` | Queries, Blocked, _Blocked % (`blocked_percent`)_, Gravity | Pi-hole v5 (legacy API, `key` is the optional API token). Same as `pihole` with version 5. |
| `tailscale` | `key`, `deviceid` |  | Address, Last Seen (`last_seen`), Expires, _User_, _Hostname_, _Name_, _Client Version (`client_version`)_, _OS_, _Created_, _Authorized_, _Is External (`is_external`)_, _Update Available (`update_available`)_, _Tags_ | Tailscale API (https://api.tailscale.com, no url needed). `key` is an API access token, `deviceid` the device (node) ID. |
| `technitium` | `key` | `node`, `range` (default `"LastHour"`) | Queries (`totalQueries`), _Success (`totalNoError`)_, Failures (`totalServerFailure`), _NX Domains (`totalNxDomain`)_, _Refused (`totalRefused`)_, Authoritative (`totalAuthoritative`), _Recursive (`totalRecursive`)_, Cached (`totalCached`), _Blocked (`totalBlocked`)_, _Dropped (`totalDropped`)_, _Clients (`totalClients`)_ | `key` is an API token; optional `range` (LastHour, LastDay...) and cluster `node`. |
| `traefik` |  | `password`, `username` | Routers, Services, Middleware | Traefik API (api@internal). `username` / `password` only if it sits behind basic auth. |
| `unifi` |  | `key`, `password`, `site` (default `"default"`), `username` | Uptime, WAN, LAN Users (`lan_users`), LAN Devices (`lan_devices`), LAN, WLAN Users (`wlan_users`), WLAN Devices (`wlan_devices`), WLAN | UniFi OS console (UDM, Cloud Key...): an API `key`, or a local account `username` / `password`. `site` is the site name or description (default "default"). Older standalone controllers (no /proxy/network prefix) are not supported. |
| `wgeasy` | `password` | `threshold` (default `"2"`), `username`, `version` (default `"1"`) | Connected, Enabled, _Disabled_, Total | wg-easy: `version: 2` for wg-easy v15+ (default 1, as in gethomepage). v1: `password` of the web UI; v2: admin `username` / `password`. "Connected" = handshake within `threshold` minutes (default 2). |
| `wgeasy2` | `username`, `password` | `threshold` (default `"2"`) | Connected, Enabled, _Disabled_, Total | wg-easy v15+: `username` / `password` of an admin, same `threshold`. Same as `wgeasy` with version 2. |

## Infrastructure: hypervisors, NAS, container managers, monitoring and backup tools

| Type | Required options | Optional options | Fields | Notes |
|---|---|---|---|---|
| `arcane` | `env`, `key` |  | Running, Stopped, Total, _Images_, _Used (`images_used`)_, _Unused (`images_unused`)_, Image Updates (`image_updates`) | env = Arcane environment id (0 = local). Like gethomepage, only Running, Stopped, Total and Image Updates are shown unless listed in `fields`. |
| `argocd` | `key` |  | Apps, Synced, Out Of Sync (`outOfSync`), Healthy, _Degraded_, _Progressing_, _Missing_, _Suspended_ | Like gethomepage, only Apps, Synced, Out Of Sync and Healthy are shown unless listed in `fields`. |
| `backrest` |  | `password`, `username` | Plans (`num_plans`), Succeeding (`num_success_latest`), Failing (`num_failure_latest`), Successes (`num_success_30`), Failures (`num_failure_30`), Bytes Added (`bytes_added_30`) |  |
| `beszel` | `username`, `password` | `systemId` (default `""`) | Systems, Up, Name, Status, _Updated_, CPU, MEM (`memory`), _Disk_, _NET (`network`)_ | Logs in as a superuser (PocketBase >= 0.23, Beszel >= 0.9; gethomepage `version: 2`). The older admins login (`version: 1`) is not supported. systemId: system id or name. |
| `checkmk` | `site`, `username`, `password` |  | Service issues (`serviceErrors`), Host issues (`hostErrors`) | site = monitoring site name; username / password of an automation user. |
| `diskstation` | `username`, `password` | `volume` (default `""`) | Uptime, Available (`volumeAvailable`), CPU (`resources.cpu`), MEM (`resources.mem`) | volume: volume name (e.g. volume_1); the first volume when unset. API versions are fixed (Auth v7, Core.System v1) instead of discovered through SYNO.API.Info. |
| `dockhand` |  | `environment` (default `""`), `key`, `password`, `username` | Running, _Stopped_, _Paused_, _Pending Updates (`pending_updates`)_, Total, CPU, Memory, _Images_, _Volumes_, _Stacks_, _Events Today (`events_today`)_ | API key (Bearer) or username + password (session login). environment: name or id; all when unset. Like gethomepage, only Running, Total, CPU and Memory are shown unless listed in `fields`. |
| `duplicati` | `password` |  | Jobs, _Stored_, Last Run (`lastBackup`), Next Run (`nextRun`), _Running_, _Warnings_, Errors | Like gethomepage, only Jobs, Errors, Last Run and Next Run are shown unless listed in `fields`. |
| `gatus` |  | `password`, `username` | Sites Up (`up`), Sites Down (`down`), Uptime |  |
| `glances` |  | `metric` (default `"info"`), `password`, `username`, `version` (default `"3"`) | CPU, Mem, Swap, GPU, Used, Free, Total, RX, TX, Read, Write, Temp, Warn, Crit | metric (as in gethomepage): info, cpu, memory, network:<iface>, disk:<device>, sensor:<label>, gpu:<id>, fs:<mount point>. Current values only (no charts); process and containers lists are not supported. version: Glances API version (3 or 4). |
| `grafana` |  | `alerts` (default `"grafana"`), `password`, `username`, `version` (default `"1"`) | Dashboards, Data Sources (`datasources`), Total Alerts (`totalalerts`), Alerts Triggered (`alertstriggered`) | Needs an account allowed to read /api/admin/stats (Grafana admin). version 1 (default): legacy alerts, else Grafana alertmanager; version 2: the `alerts` alertmanager (grafana or alertmanager). |
| `healthchecks` | `key` | `uuid` (default `""`) | Up, Down, Status, Last Ping (`last_ping`) | uuid: one check (Status, Last Ping); all checks (Up, Down) when unset. |
| `komodo` | `key`, `secret` | `showStacks` (default `"false"`), `showSummary` (default `"false"`) | Total, Running, Stopped, Down, Unhealthy, _Unknown_, Servers, Stacks, Containers | showStacks: true counts stacks instead of containers in Total / Running / Unhealthy / Unknown. showSummary: true shows Servers / Stacks / Containers by default instead. |
| `kopia` |  | `password`, `snapshotHost` (default `""`), `snapshotPath` (default `""`), `username` | Status, Size, Last Run (`lastrun`), Next Run (`nextrun`) | snapshotHost / snapshotPath: pick a snapshot source; the first one when unset. |
| `netdata` |  | `password`, `username` | Warnings, Criticals |  |
| `openmediavault` | `username`, `password`, `method` |  | Running, Stopped, Downloading, Total, Passed, Failed | method: services.getStatus, smart.getListBg or downloader.getDownloadList (as in gethomepage). smart.getListBg is read with the synchronous smart.getList (no background job polling). |
| `peanut` | `key` | `password`, `username` | Battery Charge (`battery_charge`), UPS Load (`ups_load`), UPS Status (`ups_status`) | key: UPS device name in PeaNUT. |
| `portainer` | `env`, `key` | `kubernetes` (default `"false"`) | Running, Stopped, Total, Applications, Services, Namespaces | env = Portainer environment (endpoint) id. kubernetes: true for a Kubernetes environment. |
| `prometheus` |  | `password`, `username` | Targets Up (`targets_up`), Targets Down (`targets_down`), Total Targets (`targets_total`) |  |
| `proxmox` | `username`, `password` | `node` (default `""`) | VMs, LXC, CPU (`resources.cpu`), MEM (`resources.mem`) | username = API token id (user@realm!tokenid), password = token secret. node: limit to one node. |
| `proxmoxbackupserver` | `username`, `password` | `datastore` (default `""`) | Datastore (`datastore_usage`), Failed Tasks 24h (`failed_tasks_24h`), CPU (`cpu_usage`), Memory (`memory_usage`) | username = API token id (user@realm!tokenid), password = token secret. datastore: one datastore's usage; all datastores combined when unset or not found. |
| `pulse` | `key` | `version` (default `"1"`) | Nodes, VMs, LXCs | version: 2 for Pulse v6 and later (/api/state/summary), 1 (default) for /api/resources. |
| `scrutiny` |  | `password`, `username` | Passed, Failed, Unknown |  |
| `truenas` |  | `key`, `password`, `pool` (default `""`), `username` | System Load (`load`), Uptime, Alerts, _Pool Used_, _Pool Health_ | API key (Bearer) or username + password, over the REST API (v2.0). gethomepage's `version: 2` (websocket API) is not supported: the REST API is used whatever `version` is. Pool fields (gethomepage: enablePools) are hidden unless listed in `fields`; pool: limit to one pool. |
| `unraid` | `key` | `pool1` (default `""`), `pool2` (default `""`), `pool3` (default `""`), `pool4` (default `""`) | Status, _Memory Available (`memoryAvailable`)_, _Memory Used (`memoryUsed`)_, Memory Used % (`memoryPercent`), CPU, Notifications, _Array Used (`arrayUsedSpace`)_, _Array Free (`arrayFree`)_, _Array Used % (`arrayUsedPercent`)_, _Pool 1 Used (`pool1UsedSpace`)_, _Pool 1 Free (`pool1FreeSpace`)_, _Pool 1 Used % (`pool1UsedPercent`)_, _Pool 2 Used (`pool2UsedSpace`)_, _Pool 2 Free (`pool2FreeSpace`)_, _Pool 2 Used % (`pool2UsedPercent`)_, _Pool 3 Used (`pool3UsedSpace`)_, _Pool 3 Free (`pool3FreeSpace`)_, _Pool 3 Used % (`pool3UsedPercent`)_, _Pool 4 Used (`pool4UsedSpace`)_, _Pool 4 Free (`pool4FreeSpace`)_, _Pool 4 Used % (`pool4UsedPercent`)_ | pool1..pool4: pool names for the pool fields. Like gethomepage, only Status, CPU, Memory Used % and Notifications are shown unless listed in `fields`. |
| `uptimekuma` | `slug` | `password`, `username` | Sites Up (`up`), Sites Down (`down`), Uptime, Incident | slug: status page slug. |
| `uptimerobot` | `key` |  | Sites Up (`sitesUp`), Sites Down (`sitesDown`), Status, Uptime, Last Downtime (`lastDown`), Downtime Duration (`downDuration`) | Like gethomepage: Sites Up / Down for several monitors; Status, Uptime and the last downtime for an API key limited to one monitor. |
| `watchtower` | `key` |  | Scanned (`containers_scanned`), Updated (`containers_updated`), Failed (`containers_failed`) |  |
| `whatsupdocker` |  | `key`, `password`, `username` | Monitoring, Updates | API key (Bearer) or username + password. |
| `zabbix` |  | `key`, `password`, `username` | _Not classified (`unclassified`)_, _Information_, Warning, Average, High, Disaster | API token (Bearer) or username + password. Like gethomepage, Not classified and Information are hidden unless listed in `fields`. |

## Self-hosted apps: home automation, documents, recipes, dev tools, notes, 3D printing, energy

| Type | Required options | Optional options | Fields | Notes |
|---|---|---|---|---|
| `azuredevops` | `organization`, `project`, `definitionId` | `branchName`, `key` (default `""`), `password`, `repositoryId`, `userEmail` (default `""`), `username` | Result, Status, Total PRs (`totalPrs`), My PRs (`myPrs`), Approved | API at https://dev.azure.com (set url only to override). key: a personal access token (or username + password, the PAT as password). Latest build of definitionId (branchName optional). Pull request stats need repositoryId and userEmail. |
| `booklore` | `username`, `password` |  | Libraries, Books, Reading, Finished |  |
| `changedetectionio` | `key` |  | Diffs Detected (`diffsDetected`), Total Observed (`totalObserved`) |  |
| `esphome` |  | `key`, `password`, `username` | Online, Offline, Not Online (`offline_alt`), _Unknown_, Total | Auth is optional: username + password, or key (the "authenticated" cookie value). |
| `evcc` |  |  | Production (`pv_power`), Grid (`grid_power`), Consumption (`home_power`), Charger (`charge_power`) | Kilowatts, as in the gethomepage widget (grid and charger are clamped at 0). |
| `filebrowser` | `username`, `password` |  | Available, Used, Total | authHeader (proxy header auth) is not supported. |
| `firefly` | `key` |  | Net Worth (`networth`), Budget | Current month. Net worth is the first currency reported by Firefly III. |
| `freshrss` | `username`, `password` |  | Unread, Subscriptions |  |
| `frigate` |  | `password`, `username` | Cameras, Uptime, Version | Auth is optional: username + password (Frigate 0.14+ authenticated port). |
| `ghostfolio` | `key` |  | Today (`gross_percent_today`), One year (`gross_percent_1y`), All time (`gross_percent_max`), _Net Worth (`net_worth`)_ | key: the Ghostfolio security token's JWT (Bearer). |
| `gitea` | `key` |  | Notifications, Issues, Pull Requests (`pulls`), Repositories |  |
| `gitlab` | `key`, `user_id` |  | Groups, Issues, Merge Requests (`merges`), Projects |  |
| `gotify` | `key` |  | Applications (`apps`), Clients, Messages | key: a client token. Messages are counted over the latest page (100 by default), as in gethomepage. |
| `homeassistant` | `key` |  | People Home (`people_home`), Lights On (`lights_on`), Switches On (`switches_on`) | key: a long-lived access token. The `custom` queries of gethomepage are not supported. |
| `homebox` | `username`, `password` |  | Items, _With Warranty (`totalWithWarranty`)_, Locations, _Labels_, _Users_, Total Value (`totalValue`) |  |
| `homebridge` | `username`, `password` |  | Status, Updates, Child Bridges (`child_bridges`) |  |
| `karakeep` | `key` |  | Bookmarks, Favorites, Archived, Highlights, _Lists_, _Tags_ |  |
| `linkwarden` | `key` |  | Links, Collections, Tags |  |
| `lubelogger` |  | `password`, `username`, `vehicleID` (default `""`) | Vehicles, Vehicle, Service Records (`serviceRecords`), Reminders, Next Reminder (`nextReminder`) | Basic auth is optional (username + password). vehicleID: show a single vehicle (Vehicle and Next Reminder instead of Vehicles). |
| `mastodon` |  |  | Users (`user_count`), Posts (`status_count`), Domains (`domain_count`) |  |
| `mealie` | `key` | `version` (default `"1"`) | Recipes, Users, Categories, Tags | version: 1 (default, /api/groups/statistics) or 2 (Mealie v2+, /api/households/statistics). |
| `miniflux` | `key` |  | Unread, Read |  |
| `moonraker` |  | `key` | Printer State (`printer_state`), Layers, Progress (`print_progress`), Print Status (`print_status`) | key: Moonraker API key, only if authorization is enabled. |
| `myspeed` |  | `password` | Download, Upload, Ping | password: the MySpeed password, if one is set. Results are in Mbps. |
| `nextcloud` |  | `key`, `password`, `username` | _Cpu Load (`cpuload`)_, _Memory Usage (`memoryusage`)_, Free Space (`freespace`), Active Users (`activeusers`), Files (`numfiles`), Shared Items (`numshares`) | key (NC-Token), or username + password. |
| `ntfy` | `topic` | `key`, `password`, `username` | Title, Message, Priority, Last Received (`lastReceived`), _Tags_ | Shows the latest message of a topic. Protected topics: key (access token) or username + password. |
| `octoprint` | `key` |  | Status (`printer_state`), Tool temp (`temp_tool`), Bed temp (`temp_bed`), Completion (`job_completion`) | Completion is shown while printing (or paused). |
| `opendtu` |  | `password`, `username` | Today (`yieldDay`), Power % (`relativePower`), Power (`absolutePower`), Limit | Basic auth is optional (username + password). |
| `paperlessngx` |  | `key`, `password`, `username` | Inbox, Total | key: an API token, or username + password. |
| `pterodactyl` | `key` |  | Nodes, Servers |  |
| `speedtest` |  | `key`, `version` (default `"1"`) | Download, Upload, Ping | Speedtest Tracker (alexjustesen). version: 1 (default, /api/speedtest/latest, Mbps) or 2 (/api/v1/results/latest, bytes per second; needs key, an API token). |
| `spoolman` |  |  | Spools, Remaining, Lowest | Summary of all spools (the per-spool blocks of gethomepage need dynamic labels). |
| `syncthing` | `key` |  | Connected, Synced, Errors, Storage |  |
| `tandoor` | `key` |  | Users, Recipes, Keywords |  |
| `trilium` | `key` |  | Version, Notes (`notesCount`), Database Size (`dbSize`) | key: an ETAPI token. |
| `vikunja` | `key` | `version` (default `"1"`) | Active Projects (`projects`), Tasks Due This Week (`tasks7d`), Overdue Tasks (`tasksOverdue`), Tasks In Progress (`tasksInProgress`) | version: 1 (default, /tasks/all) or 2 (Vikunja 0.24+, /tasks). Looks at the first page of open tasks (the server's page size), as gethomepage does. Due dates are compared as UTC. |
| `wallos` | `key` |  | Subscriptions (`activeSubscriptions`), Next Payment (`nextRenewingSubscription`), _Prev. Month (`previousMonthlyCost`)_, This Month (`thisMonthlyCost`), Next Month (`nextMonthlyCost`) |  |
| `yourspotify` | `key` | `interval` (default `"week"`) | Songs, Time, Artists | interval: day, week (default), month (30 days), year or all. |
