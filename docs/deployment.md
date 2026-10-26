# Deployment

Homepage is a single binary (HTML, CSS, JS and widget definitions are embedded) that reads one
YAML file. It writes nothing to disk and keeps no state: it can be restarted, moved or run
several times without care.

- [Docker image](#docker-image)
- [Docker Compose](#docker-compose)
- [Kubernetes](#kubernetes)
- [Binary](#binary)
- [Reverse proxy and authentication](#reverse-proxy-and-authentication)
- [Migrating from gethomepage](#migrating-from-gethomepage)
- [Upgrading](#upgrading)
- [Troubleshooting](#troubleshooting)
- [CI/CD and releases](#cicd-and-releases)

## Docker image

Images are published to GitHub Container Registry, for `linux/amd64` and `linux/arm64`:

| Tag | Content |
|---|---|
| `ghcr.io/mysecurehomelab/homepage:latest` | Latest published build of `main`. |
| `ghcr.io/mysecurehomelab/homepage:X.Y.Z` | The version in `Cargo.toml`. The next published build of the same version replaces it. |
| `ghcr.io/mysecurehomelab/homepage:X.Y` | Latest `X.Y.*` version. |
| `ghcr.io/mysecurehomelab/homepage:sha-<commit>` | One exact commit, immutable. |

For reproducible deployments, pin `X.Y.Z` or `sha-…`.

About the image:

- base `gcr.io/distroless/cc-debian12:nonroot`: no shell, no package manager;
- runs as uid/gid **65532** (non-root);
- listens on port **3000**;
- reads its config from **`/config/config.yaml`** (`HOMEPAGE_CONFIG`);
- works with a read-only filesystem (`--read-only`), nothing is written.

```bash
docker run -d --name homepage -p 3000:3000 \
  -v ./config:/config:ro \
  -e SONARR_API_KEY=... \
  ghcr.io/mysecurehomelab/homepage:latest
```

Mount the **directory** rather than the file alone (`-v ./config:/config:ro`): many editors
replace the file on save, and a single-file bind mount keeps pointing at the old one, which
breaks hot reload.

Validate the config without starting the server:

```bash
docker run --rm -v ./config:/config:ro ghcr.io/mysecurehomelab/homepage:latest check
```

The image has no `curl`, so a Docker `HEALTHCHECK` cannot run inside the container. Monitor
`/healthz` from outside (reverse proxy, Uptime Kuma, Kubernetes probes).

## Docker Compose

A complete example: Homepage, container discovery through a read-only socket proxy, and secrets.

```yaml
services:
  homepage:
    image: ghcr.io/mysecurehomelab/homepage:latest
    restart: unless-stopped
    read_only: true
    ports:
      - "3000:3000"
    volumes:
      - ./config:/config:ro
    environment:
      SONARR_API_KEY: ${SONARR_API_KEY}      # from compose's .env
    secrets:
      - proxmox_token
    depends_on:
      - socket-proxy

  socket-proxy:
    image: tecnativa/docker-socket-proxy
    restart: unless-stopped
    environment:
      CONTAINERS: 1                          # container list only, read-only
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock:ro

secrets:
  proxmox_token:
    file: ./secrets/proxmox_token
```

`config/config.yaml`:

```yaml
title: Homelab
docker:
  local: tcp://socket-proxy:2375

groups:
  - name: Infra
    services:
      - name: Proxmox
        url: https://pve.lan:8006
        icon: proxmox
        check: tcp://pve.lan:8006            # tcp: works with a self-signed certificate
        widget:
          type: proxmox                      # needs a trusted certificate on the API url
          username: homepage@pam!homepage    # API token id
          password: ${file:/run/secrets/proxmox_token}
```

Why a socket proxy: the image runs as non-root and cannot read `docker.sock` directly, and full
socket access is root on the host anyway. The proxy only exposes reading containers.

For more Docker hosts, run a `socket-proxy` on each and add them:

```yaml
docker:
  local: tcp://socket-proxy:2375
  nas: tcp://nas.lan:2375
```

To mount the socket directly instead (`unix:///var/run/docker.sock`), run the container with
the socket's group: `group_add: ["<gid of docker.sock>"]` (`stat -c %g /var/run/docker.sock`).

## Kubernetes

A complete manifest: namespace, a service account allowed to list Ingresses, config in a
ConfigMap, secrets, a hardened Deployment and a Service.

```yaml
apiVersion: v1
kind: Namespace
metadata:
  name: homepage
---
apiVersion: v1
kind: ServiceAccount
metadata:
  name: homepage
  namespace: homepage
---
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRole
metadata:
  name: homepage
rules:
  - apiGroups: ["networking.k8s.io"]
    resources: ["ingresses"]
    verbs: ["list"]
---
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRoleBinding
metadata:
  name: homepage
roleRef:
  apiGroup: rbac.authorization.k8s.io
  kind: ClusterRole
  name: homepage
subjects:
  - kind: ServiceAccount
    name: homepage
    namespace: homepage
---
apiVersion: v1
kind: ConfigMap
metadata:
  name: homepage
  namespace: homepage
data:
  config.yaml: |
    title: Homelab
    kubernetes: true
    groups:
      - name: Media
        services:
          - name: Sonarr
            url: https://sonarr.example.lan
            icon: sonarr
            widget:
              type: sonarr
              url: http://sonarr.media.svc:8989
              key: ${SONARR_API_KEY}
---
apiVersion: v1
kind: Secret
metadata:
  name: homepage
  namespace: homepage
stringData:
  SONARR_API_KEY: change-me
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: homepage
  namespace: homepage
spec:
  replicas: 1
  selector:
    matchLabels: { app: homepage }
  template:
    metadata:
      labels: { app: homepage }
    spec:
      serviceAccountName: homepage
      securityContext:
        runAsNonRoot: true
        runAsUser: 65532
        seccompProfile: { type: RuntimeDefault }
      containers:
        - name: homepage
          image: ghcr.io/mysecurehomelab/homepage:0.1.0
          ports:
            - { name: http, containerPort: 3000 }
          envFrom:
            - secretRef: { name: homepage }
          volumeMounts:
            - { name: config, mountPath: /config, readOnly: true }
          readinessProbe:
            httpGet: { path: /healthz, port: http }
          livenessProbe:
            httpGet: { path: /healthz, port: http }
          resources:
            requests: { cpu: 10m, memory: 32Mi }
            limits: { memory: 128Mi }
          securityContext:
            allowPrivilegeEscalation: false
            readOnlyRootFilesystem: true
            capabilities: { drop: [ALL] }
      volumes:
        - name: config
          configMap: { name: homepage }
---
apiVersion: v1
kind: Service
metadata:
  name: homepage
  namespace: homepage
spec:
  selector: { app: homepage }
  ports:
    - { name: http, port: 80, targetPort: http }
```

Notes:

- Mount the ConfigMap as a **directory** (as above), not with `subPath`: Kubernetes never
  updates a `subPath` mount. Without `subPath`, ConfigMap edits reach the pod within about a
  minute and are hot-reloaded.
- Environment variables (`envFrom`) are only read when the pod starts: after changing the
  Secret, restart (`kubectl rollout restart -n homepage deploy/homepage`). For hot-reloaded
  secrets, mount the Secret as a volume and use `${file:/secrets/…}`.
- For your apps, add `gethomepage.dev/*` annotations to their Ingresses (see
  [configuration.md](configuration.md#kubernetes-annotations)).
- One replica is enough; several work too (each one runs its own checks).

## Binary

Needs a recent stable Rust toolchain (edition 2024).

```bash
git clone https://github.com/MySecureHomelab/homepage
cd homepage
cargo build --release
HOMEPAGE_CONFIG=config.example.yaml ./target/release/homepage check
./target/release/homepage
```

systemd service (`/etc/systemd/system/homepage.service`):

```ini
[Unit]
Description=Homepage
After=network-online.target
Wants=network-online.target

[Service]
ExecStart=/usr/local/bin/homepage
Environment=HOMEPAGE_CONFIG=/etc/homepage/config.yaml
Environment=HOMEPAGE_ADDR=127.0.0.1:3000
EnvironmentFile=-/etc/homepage/env
DynamicUser=yes
ProtectSystem=strict
ProtectHome=yes
NoNewPrivileges=yes
Restart=on-failure

[Install]
WantedBy=multi-user.target
```

`/etc/homepage/env` holds the secrets (`SONARR_API_KEY=…`, `chmod 600`). To discover local
containers, add `SupplementaryGroups=docker` and `docker: { local: unix:///var/run/docker.sock }`
(full socket access: prefer a proxy when you can).

## Reverse proxy and authentication

Homepage has no authentication. Behind a reverse proxy, protect it with your SSO.

Caddy + Authelia:

```caddyfile
home.example.com {
	forward_auth authelia:9091 {
		uri /api/authz/forward-auth
		copy_headers Remote-User Remote-Groups
	}
	reverse_proxy homepage:3000
}
```

Traefik (Docker labels):

```yaml
labels:
  traefik.enable: "true"
  traefik.http.routers.homepage.rule: Host(`home.example.com`)
  traefik.http.routers.homepage.middlewares: authelia@docker
  traefik.http.services.homepage.loadbalancer.server.port: "3000"
```

The proxy needs nothing special: no WebSocket, the page polls `/api/state` every 5 s. Homepage
can live under a sub-path (`/home/`, with the trailing slash) as long as the proxy strips the
prefix: links to `app.css`, `app.js` and `api/state` are relative.

To listen on localhost only, behind the proxy: `HOMEPAGE_ADDR=127.0.0.1:3000`.

## Migrating from gethomepage

Docker labels (`homepage.*`) and Kubernetes annotations (`gethomepage.dev/*`) work as is, and
both listen on port 3000. Service entries are understood too (`href`, `siteMonitor`,
`server` + `container`, `widgets`), and widget `fields` lists can use gethomepage's field names.
The config folder itself differs: gethomepage splits it into several files, Homepage reads one
`config.yaml`.

[`scripts/import-gethomepage.sh`](../scripts/import-gethomepage.sh) converts the folder. It
needs `jq`, and `yq` or Docker (it then runs the `mikefarah/yq` image).

```bash
scripts/import-gethomepage.sh /path/to/gethomepage/config > config/config.yaml
docker run --rm -v ./config:/config:ro --env-file .env ghcr.io/mysecurehomelab/homepage:latest check
```

What it converts:

| gethomepage | Homepage |
|---|---|
| `settings.yaml` `title`, `theme` | `title`, `theme` |
| `widgets.yaml` `datetime`, `search`, `resources` | `widgets: [clock, search, resources]`, `search` (custom `url` gets `{q}` appended) |
| `docker.yaml` `socket` / `host` + `port` | `docker: { name: unix://… / tcp://host:port }` |
| `kubernetes.yaml` `mode: cluster` or `default` | `kubernetes: true` |
| `services.yaml` groups and services | `groups` |
| `href`, `description` | `url`, `description` |
| `icon: name.png`, `mdi-x`, `si-x`, `sh-x`, URL | dashboard-icons name, or the matching CDN URL |
| `siteMonitor` | `check: <url>` |
| `server` + `container` | `check: docker://server/container` |
| `widget` (or the first of `widgets`) | `widget`, options as strings, `fields` kept |
| `bookmarks.yaml` | `bookmarks` |
| `{{HOMEPAGE_VAR_X}}` | `${HOMEPAGE_VAR_X}`: the same environment variables keep working |
| `{{HOMEPAGE_FILE_X}}` | `${file:<path>}` when `HOMEPAGE_FILE_X` is set while converting, otherwise a `CHANGE_ME` path to fix |

Everything else is reported on stderr as a warning and left out: other settings (colors,
layout…), other info widgets, `ping` (no ICMP here: use `check: tcp://host:port`), nested
groups, other service keys (`showStats`, `statusStyle`…) and list or object widget options.

Widget options that only exist in gethomepage (`enableQueue`…) are kept. The dashboard ignores
them and lists them in its problems banner; `homepage check` names each one with the options
the type accepts, so you can delete them.

## Upgrading

- Docker: `docker compose pull && docker compose up -d`.
- Kubernetes: change the image tag, or `kubectl rollout restart` when following `latest`.
- Binary: `git pull && cargo build --release`, then restart the service.

The config has no migrations: if a new version rejects a key, the error names it. Run
`homepage check` with the new version before switching.

## Troubleshooting

| Symptom | Likely cause |
|---|---|
| Red "Config error" banner | The file is unreadable or not valid YAML. The banner gives the line; the last valid config (or an empty page) is shown until it is fixed. |
| Orange "config problems" banner | Something is left out (a bad check, an unknown widget option, an unset `${VAR}`…). Open it for the list; everything else works. |
| Edits are not picked up | Single file mounted (not the directory), or ConfigMap mounted with `subPath`. |
| Service down although it works | Self-signed certificate (use `tcp://`), a 5xx answer, or the Homepage container cannot resolve or reach the host (test from the same Docker network). |
| Widget "HTTP 401" / "login failed" | Wrong key or credentials, or the type expects another option: see [widget-types.md](widget-types.md). |
| Widget "error sending request" | Host unreachable from Homepage, or certificate rejected. Give the widget an internal `url`. |
| No Docker services | `discovery: docker …` in the logs: unreadable socket (non-root) or unreachable proxy. Only containers with `homepage.name` show up. |
| No Kubernetes services | Missing RBAC (`list ingresses`), or `gethomepage.dev/enabled` is not `"true"` (quoted). |
| `resources` empty | First seconds after startup; the first sample comes within 5 s. |

## CI/CD and releases

The [`.github/workflows/ci.yml`](../.github/workflows/ci.yml) workflow runs on demand only:
*Actions → CI → Run workflow* (or `gh workflow run ci.yml --ref main`).

1. **check**: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test` (which also
   validates `config.example.yaml`, every widget definition and that
   [widget-types.md](widget-types.md) is up to date), then `cargo build --release` and
   `homepage check` on the example config.
2. **image**: builds the Docker image for `amd64` and `arm64` (native runners, no emulation). Run
   on another branch, the image is only built, to prove the build works. Run on `main`, it is pushed to
   GHCR as `X.Y.Z`, `X.Y`, `latest` and `sha-<commit>`, where `X.Y.Z` is read from
   `Cargo.toml`.

Releasing a version:

1. Bump `version` in `Cargo.toml` (and run `cargo build` to update `Cargo.lock`).
2. Merge to `main`, then run the workflow on `main`: the `X.Y.Z` image is published.

The first time, the GHCR package is private: make it public under the repository's (or the
organization's) *Packages* settings so it can be pulled without logging in.
