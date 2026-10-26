# Changelog

## 0.1.1

### Never crashes on config mistakes

- The server starts even when the config is missing or invalid: the page shows the error with
  its line and reloads as soon as the file is fixed.
- A mistake in one service no longer rejects the whole file. Bad checks, unknown widget types,
  missing or unknown options, unknown fields, unset `${VAR}`, invalid `accent`/`search`/
  `check_interval`, bad Docker endpoints and broken `widget_types` are reported in an orange
  banner (with their location) and left out; everything else works. A widget that cannot work
  shows why on its own card.
- `homepage check` stays strict: any problem exits with code 1.

### gethomepage compatibility

- Service entries accept gethomepage's keys: `href`, `siteMonitor`, `server` + `container`,
  `widgets`. Other keys are reported and ignored.
- Widget options may be numbers or booleans; gethomepage-only display options (`enableQueue`…)
  are ignored with a warning instead of an error.
- `fields` accepts gethomepage's field names (`wanted`, `blocked_percent`…), ignoring case.
- Every built-in widget was checked against gethomepage's source: endpoints, auth (API key or
  username/password alternatives), API versions (`version` option for Pi-hole, pfSense,
  NetAlertX, wg-easy, Authentik, Immich, Komga, FileFlows, Mealie, Vikunja, Speedtest, Gluetun,
  Jellyfin…), fields, labels and default fields. Many widgets were fixed; see the commit diff
  of `widgets/*.yaml` for details.
- **Behaviour change:** types with a `version` option default to the version gethomepage
  defaults to (usually the older API), like gethomepage. Set `version` for the newer API, e.g.
  `version: 6` for Pi-hole 6.
- `scripts/import-gethomepage.sh` converts a gethomepage config folder (see
  [deployment.md](docs/deployment.md#migrating-from-gethomepage)).

### Widget engine

- Fields can set `key` (gethomepage name), `hidden` (shown only when listed) and `when` (jq
  condition on the options and responses).
- A login runs when at least one of its credentials is set, so one login can accept an API key
  or a username/password.
- `defaults: { url: … }` gives a type a fixed API address (NextDNS, Tailscale, Cloudflare,
  Azure DevOps).
- Optional requests refused with 401/403/409 give `null` when there is no login to retry.
- `token` is an ordinary option in types without a login step.

### CI

- The CI workflow runs on demand (`workflow_dispatch`).

## 0.1.0

First release.
