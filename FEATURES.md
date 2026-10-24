# Homepage-rs : liste des fonctionnalités

Un dashboard de homelab en Rust inspiré de [gethomepage.dev](https://gethomepage.dev), mais plus propre, plus customisable et plus complet.

Légende : **[H]** = existe dans Homepage · **[+]** = nouveauté / amélioration · **P0/P1/P2** = priorité (P0 = MVP)

---

## 1. Socle technique

| Brique | Choix proposé | Pourquoi |
|---|---|---|
| Serveur HTTP | `axum` + `tokio` | standard, rapide, middleware tower |
| Rendu | SSR (`askama` ou `maud`) + `htmx` + SSE | pas de build JS, page rendue en <50 ms, live updates sans SPA |
| Client HTTP | `reqwest` (rustls) | appels aux API des services |
| Config | `serde` + `serde_yaml` / TOML | typée, erreurs précises |
| Docker | `bollard` | API Docker / Podman |
| Kubernetes | `kube-rs` | discovery + métriques |
| Système | `sysinfo` | CPU/RAM/disque/réseau/températures |
| Stockage | `sqlite` (`rusqlite`/`sqlx`) | historique, uptime, état |
| Plugins | `wasmtime` (WASI) | widgets tiers sandboxés |
| Distribution | binaire statique unique (assets embarqués via `rust-embed`) + image Docker distroless multi-arch | |

---

## 2. Cœur de l'application

### Configuration — P0
- [H] Fichiers YAML : services, widgets, bookmarks, settings
- [+] Un seul format au choix : YAML **ou** TOML
- [+] Validation au démarrage avec erreurs précises (fichier, ligne, colonne, suggestion « vouliez-vous dire… »)
- [+] Schéma JSON généré automatiquement → autocomplétion dans VS Code
- [+] Commande `homepage check` (valide la config sans lancer le serveur, utile en CI)
- [H] Hot reload de la config (watcher `notify`), sans redémarrage
- [H] Substitution de variables `{{HOMEPAGE_VAR_X}}`
- [+] Secrets depuis fichier (`_FILE`), Docker secrets, Vault, sops/age
- [+] Includes / fragments (`!include services/media.yaml`) pour découper la config
- [+] Templates / ancres réutilisables (ex. même auth pour 5 services)

### Layout & UI — P0/P1
- [H] Groupes de services, colonnes, lignes, groupes repliables
- [H] Onglets (tabs)
- [+] **Plusieurs pages** (`/media`, `/infra`, `/network`) avec routes dédiées
- [+] Grille libre type Grafana (positions x/y/w/h), tailles de widget S/M/L/XL
- [+] **Éditeur visuel** (drag & drop, édition en place) qui réécrit la config — optionnel, config-as-code reste la source de vérité
- [+] Layout responsive spécifique mobile / tablette / TV / kiosque
- [+] Mode kiosque (plein écran, rotation automatique des pages)
- [H] Icônes : dashboard-icons, Material Design, Simple Icons, URL perso
- [+] Icônes selfh.st, emojis, Lucide ; cache local des icônes (pas de requêtes externes au runtime)
- [H] Fond d'écran perso, flou, opacité
- [+] Fonds dynamiques (Unsplash, dossier local, selon l'heure/météo)

### Thèmes & personnalisation — P1
- [H] Clair/sombre, palettes de couleurs
- [+] Système de thèmes complet par variables CSS (fichier `theme.toml`)
- [+] Suivi automatique du thème système + bascule planifiée (jour/nuit)
- [+] CSS et JS personnalisés injectés (déjà dans H, mais avec rechargement à chaud)
- [+] Thèmes communautaires prêts : Catppuccin, Nord, Dracula, Gruvbox, Tokyo Night, Solarized
- [+] Polices personnalisées (auto-hébergées)
- [+] Personnalisation par carte : couleur, bordure, transparence, style compact/détaillé

### Navigation & recherche — P1
- [H] Barre de recherche web (Google, DDG, Bing, SearXNG, perso)
- [H] Quick launch (tape pour filtrer les services)
- [+] **Command palette** `Ctrl+K` : services, bookmarks, actions, recherche web, commandes (restart container…)
- [+] Raccourcis clavier configurables, navigation 100 % clavier
- [+] Bangs façon DuckDuckGo (`!gh`, `!yt`, bangs perso)
- [+] Recherche floue (fuzzy) sur nom, description, tags

### Bookmarks — P0
- [H] Groupes de liens avec icône/abréviation
- [+] Import depuis le navigateur (HTML Netscape) / Linkwarden / Linkding
- [+] Tags, favoris, liens les plus utilisés en premier (compteur de clics local)

### Découverte automatique — P1
- [H] Docker labels (`homepage.group`, `homepage.name`…)
- [H] Kubernetes (annotations Ingress, IngressRoute Traefik, HTTPRoute)
- [+] Podman, Docker Swarm, multi-hôtes Docker (socket, TCP+TLS, SSH)
- [+] Proxmox (VMs/LXC auto), Traefik/Caddy/NPM (routes → services auto)
- [+] Nomad, Consul
- [+] Fusion intelligente découverte + config manuelle (override par nom)

### Statut des services — P0
- [H] Ping / siteMonitor (HTTP)
- [H] Statut container Docker / pod K8s
- [+] Checks TCP, ICMP, DNS, TLS (expiration certificat), gRPC health, commande custom
- [+] Codes HTTP attendus, regex sur le body, timeouts configurables
- [+] **Historique** (SQLite) : sparkline de latence, % uptime 24h/7j/30j
- [+] Page de statut publique optionnelle (façon Uptime Kuma, en lecture seule)

### Alerting & notifications — P2
- [+] Alerte sur service down, cert qui expire, disque plein, update dispo
- [+] Canaux : ntfy, Gotify, Discord, Slack, Telegram, Matrix, email (SMTP), webhook générique, Apprise
- [+] Fenêtres de maintenance, anti-flapping, regroupement

### Sécurité & multi-utilisateurs — P1
- [+] Authentification intégrée : local (argon2), **OIDC** (Authelia, Authentik, Keycloak, Pocket ID), forward-auth (en-têtes `Remote-User` / `Remote-Groups`)
- [+] Visibilité par utilisateur/groupe (groupes, services, widgets, pages)
- [+] Dashboards personnels par utilisateur
- [+] Les clés API ne sont **jamais** envoyées au navigateur (tout passe par le backend) — Homepage le fait aussi, on le garde strict
- [+] Liste blanche des hôtes autorisés (`allowedHosts`), CSP stricte, en-têtes de sécurité
- [+] Actions protégées (restart container, wake-on-LAN) derrière un rôle + confirmation
- [+] Journal d'audit des actions

### Performance & robustesse — P0
- [+] Cache par widget avec TTL configurable, rafraîchissement en arrière-plan (stale-while-revalidate)
- [+] Live updates en SSE au lieu de polling côté client
- [+] Timeouts, retry avec backoff, circuit breaker par service (un service lent ne bloque pas la page)
- [+] Démarrage < 100 ms, RAM < 20 Mo
- [+] Endpoints `/healthz`, `/metrics` (Prometheus) pour monitorer le dashboard lui-même

### Extensibilité — P2
- [+] **Widget générique API** (custom API de H en mieux) : URL, auth, JSONPath/JMESPath, mapping de champs, formatage (octets, %, durée, date), seuils de couleur
- [+] Widget générique Prometheus (requête PromQL → valeur/graphique)
- [+] Widget générique HTML/iframe/Markdown
- [+] Widget « commande shell » (sortie d'une commande, en lecture seule, opt-in)
- [+] **Plugins WASM** : widgets tiers sandboxés, sans recompiler
- [+] API REST/JSON pour lire l'état (intégration Home Assistant, scripts)
- [+] i18n (fr, en, de, es… via fichiers Fluent)
- [+] PWA installable, mode hors ligne (dernière valeur connue)

---

## 3. Widgets d'information (en-tête / globaux)

| Widget | Détails | Prio |
|---|---|---|
| Date / heure | formats, fuseaux multiples | P0 |
| Greeting | message selon l'heure, nom de l'utilisateur | P0 |
| Logo | logo perso | P0 |
| Recherche | voir section navigation | P0 |
| Ressources système | CPU, RAM, disque, uptime, température, réseau, load | P0 |
| Météo | Open-Meteo (sans clé), OpenWeatherMap | P0 |
| Glances (info) | ressources d'un hôte distant | P1 |
| Kubernetes (info) | CPU/RAM du cluster et des nœuds | P1 |
| Longhorn | volumes | P2 |
| UniFi console | stats réseau | P1 |
| Stocks / crypto | Finnhub, CoinGecko | P2 |
| **[+] Calendrier** | iCal/CalDAV (Nextcloud, Google), agenda du jour | P1 |
| **[+] Todo** | CalDAV VTODO, Vikunja, Todoist | P2 |
| **[+] Flux RSS/Atom** | liste d'articles avec cache | P1 |
| **[+] Notes / Markdown** | bloc de texte libre | P1 |
| **[+] Compte à rebours** | événements, dates clés | P2 |
| **[+] Lever/coucher du soleil, lune** | calcul local | P2 |
| **[+] Qualité de l'air / pollen** | Open-Meteo | P2 |
| **[+] Graphique Prometheus** | requête libre | P1 |
| **[+] IP publique / VPN** | IP WAN, statut VPN, géoloc | P1 |
| **[+] Speedtest local** | historique de débit | P2 |
| **[+] Certificats TLS** | jours restants par domaine | P1 |
| **[+] Mises à jour** | Diun / WUD / images Docker obsolètes | P1 |
| **[+] Wake-on-LAN** | bouton par machine | P2 |
| **[+] Lecteur en cours** | Spotify, Jellyfin, Plex « now playing » | P2 |
| **[+] Iframe / embed** | Grafana, cam, autre | P1 |
| **[+] Caméra** | snapshot MJPEG/RTSP via Frigate/go2rtc | P2 |

---

## 4. Widgets de services (intégrations API)

Homepage en a ~150. Liste complète ci-dessous, triée par catégorie. Les **[+]** n'existent pas (ou partiellement) dans Homepage.

### Médias — serveurs & lecture
Jellyfin, Emby, Plex, Tautulli, Jellystat, Navidrome, Audiobookshelf, Komga, Kavita, Calibre-web, Stash, Immich, PhotoPrism, Owncast, Channels DVR, Tdarr, Unmanic, [+] Lyrion/LMS, [+] Funkwhale, [+] Pinchflat, [+] Tube Archivist

### Médias — *arr & requêtes
Sonarr, Radarr, Lidarr, Readarr, Bazarr, Prowlarr, Jackett, Mylar, Whisparr, Overseerr, Jellyseerr, Ombi, Autobrr, Kometa, [+] Huntarr, [+] Recyclarr (statut sync), [+] Maintainerr, [+] Cleanuparr, [+] Profilarr

### Téléchargement
qBittorrent, Transmission, Deluge, rTorrent/ruTorrent, Flood, NZBGet, SABnzbd, Pyload, JDownloader, Aria2, [+] Unpackerr

### Réseau & DNS
Pi-hole, AdGuard Home, Technitium DNS, NextDNS, Blocky, UniFi, Omada, OPNsense, pfSense, MikroTik, OpenWRT, Fritz!Box, Tailscale, Headscale, NetAlertX, Gluetun, WireGuard (wg-easy), [+] Netbird, [+] CrowdSec, [+] Zerotier, [+] Firewalla, [+] Unbound

### Reverse proxy & sécurité
Traefik, Caddy, Nginx Proxy Manager, Authentik, Authelia (statut), Cloudflare Tunnels, CrowdSec, Fail2ban (via API), Vaultwarden (statut), [+] Zoraxy, [+] Pangolin, [+] Pocket ID, [+] Keycloak

### Infra & virtualisation
Proxmox, Proxmox Backup Server, TrueNAS, Unraid, OpenMediaVault, Synology (Disk Station, Download Station), QNAP, Portainer, Dockge, Komodo, Kubernetes, Docker (stats container), Nomad, ESXi, XCP-ng / Xen Orchestra, Glances, Netdata, Beszel, Scrutiny, Uptime Kuma, Gatus, Healthchecks, Uptime Robot, [+] Incus/LXD, [+] Harvester, [+] Coolify, [+] Dokploy, [+] Ceph, [+] ZFS natif (zpool status)

### Monitoring & observabilité
Prometheus, Grafana, Uptime Kuma, Gatus, Healthchecks.io, Netdata, Glances, Beszel, [+] Loki, [+] Alertmanager (alertes actives), [+] VictoriaMetrics, [+] InfluxDB, [+] Zabbix, [+] Checkmk, [+] ClickHouse/ClickStack

### Sauvegardes
Proxmox Backup Server, Duplicati, [+] Restic (REST server), [+] Borg/Borgmatic, [+] Kopia, [+] Backrest, [+] Velero, [+] Urbackup

### Domotique & IoT
Home Assistant, ESPHome, Homebridge, Frigate, Zigbee2MQTT, Node-RED, Scrypted, [+] MQTT générique, [+] Shelly, [+] Tasmota, [+] Hubitat, [+] Domoticz, [+] OpenHAB, [+] Victron/Enphase (énergie solaire), [+] onduleur NUT/PeaNUT, [+] Tesla/Teslamate

### Productivité & documents
Nextcloud, Paperless-ngx, Mealie, Tandoor, Grocy, Vikunja, Linkwarden, Wallabag, Miniflux, FreshRSS, Kavita, Gitea, Forgejo, GitLab, Gitlab CI, Gotify, ntfy, Changedetection.io, [+] Karakeep (Hoarder), [+] Linkding, [+] Outline, [+] BookStack, [+] Wiki.js, [+] Immich, [+] Actual Budget, [+] Firefly III, [+] Ghostfolio, [+] Stirling PDF, [+] Seafile

### Développement & CI
Gitea, Forgejo, GitLab, GitHub (via API), Jenkins, Drone/Woodpecker, ArgoCD, [+] Harbor, [+] Renovate, [+] Sentry, [+] SonarQube

### Jeux
Minecraft, GameDig (CS, Valheim, Ark…), Pterodactyl, Crafty Controller, [+] AMP, [+] Romm, [+] Steam (stats)

### Divers
Speedtest Tracker, MySpeed, PeaNUT, NUT, Moonraker/Klipper, OctoPrint, PrusaLink, Calendar, Evcc, Mailcow, Stalwart, [+] Syncthing, [+] Ollama / Open WebUI (modèles chargés), [+] LiteLLM, [+] Frigate stats, [+] Docker registry

### Widgets génériques (remplacent 80 % des intégrations manquantes)
- **Custom API** (JSON) + mapping de champs — P0
- **Prometheus / PromQL** — P1
- **Ping / HTTP / TCP / DNS / TLS** — P0
- **iframe / Markdown / HTML** — P1
- **Plugin WASM** — P2

---

## 5. Roadmap proposée

1. **MVP (P0)** : serveur axum, config YAML validée + hot reload, groupes/services/bookmarks, statut HTTP/ping, ressources système, date/heure, météo, recherche, custom API widget, thème clair/sombre, image Docker.
2. **v0.2** : Docker/K8s discovery, ~20 widgets les plus utilisés (Jellyfin, Plex, *arr, qBittorrent, Pi-hole, AdGuard, Proxmox, TrueNAS, Home Assistant, Uptime Kuma, Immich, Nextcloud, Traefik, Portainer…), historique SQLite, command palette.
3. **v0.3** : auth OIDC/forward-auth + visibilité par groupe, multi-pages, thèmes, calendrier/RSS, Prometheus.
4. **v0.4** : alerting, éditeur visuel, page de statut publique.
5. **v1.0** : plugins WASM, le reste des intégrations, i18n, PWA.
