# Service widgets

A widget shows live stats from a service's API on its card. The built-in types, with their
options and fields, are listed in [widget-types.md](widget-types.md).

```yaml
groups:
  - name: Media
    services:
      - name: Sonarr
        url: https://sonarr.lan
        widget:
          type: sonarr
          key: ${SONARR_API_KEY}     # or ${file:/run/secrets/sonarr}
          # url: http://sonarr:8989  # API address if it differs from the service url
          # fields: [Wanted, Queued] # which stats to show, in order (default: all)
```

API calls are made by the server: keys never reach the browser. Stats refresh with the
status checks (`check_interval`).

From Docker labels or Kubernetes annotations, use the same names as gethomepage.dev:
`homepage.widget.type`, `homepage.widget.key`… or `gethomepage.dev/widget.type`…

## Writing a widget type

Built-in types live in [`widgets/*.yaml`](../widgets). You can add your own (or override a
built-in one) in the config under `widget_types`, with the same format:

```yaml
widget_types:
  myapp:
    needs: [key]                       # options the user must set (besides url)
    headers: { Authorization: "Bearer {key}" }
    requests:                          # name: path, appended to the widget url
      stats: /api/stats
      users: /api/users?active=true
    fields:                            # label + jq over {"stats": …, "users": …}
      - { label: Users, value: ".users | length" }
      - { label: Storage, value: .stats.bytes_used, format: bytes }
```

### Placeholders

Paths, headers, form and JSON bodies can use `{option}` placeholders:

- `{url}`: the widget url, without trailing slash (added automatically before paths).
- any option the user sets on the widget: `{key}`, `{username}`, `{password}`, `{node}`…
- `{basic}`: base64 of `username:password`, for `Authorization: "Basic {basic}"`.
- `{token}`: the result of the login step. In a type without a login, `token` is an ordinary
  option the user sets.
- `{item}`: the current element in a [dependent request](#dependent-requests).

In `path`, values are percent-encoded (except `{url}`), so a key with `&` or spaces is safe.
In headers, query, form and JSON bodies they are inserted as is (query and form are encoded by
the HTTP client). Only `{simple_name}` (letters, digits, `_`) is a placeholder: other braces, as
in JSON or GraphQL text, are left alone.

A header whose placeholders are not all set is not sent. This lets a type accept
alternatives, e.g. API key **or** basic auth. A list gives alternatives for the same header,
the first one whose placeholders are all set wins:

```yaml
headers:
  NC-Token: "{key}"
  Authorization: ["Bearer {token}", "Basic {basic}"]
```

`computed` adds placeholders computed with jq (input `null`, `$options` available), handy
for dates:

```yaml
computed:
  month_start: 'now | strftime("%Y-%m-01")'
  today: 'now | strftime("%Y-%m-%d")'
```

Options are validated: an option the type never uses is reported as ignored, so every option a
type supports must appear in a placeholder, in `needs` or in `defaults`.

```yaml
defaults: { site: default, days: "30" }   # used when the user leaves them out
```

`defaults` can also hold a fixed `url`, for services whose API lives at a fixed address
(`url: https://api.tailscale.com`): it is used when the widget sets no `url`, before the service
url (which then points at the app, not its API).

### Requests

A request is either a path (GET) or an object:

```yaml
requests:
  torrents:
    path: /transmission/rpc
    method: POST                        # GET (default) or POST
    json: { method: torrent-get, arguments: { fields: [status, rateDownload] } }
    # form: { key: value }              # form-encoded body instead of JSON
    # query: { apikey: "{key}" }        # query parameters
    # headers: { X-Extra: "{key}" }     # per-request headers
    # header: X-Total-Count             # use this response header (a string) instead of the body
    # optional: true                    # on failure the response is null instead of failing the widget
                                        # (auth failures too, unless there is a login to retry)
```

Like headers, `query`, `form` and JSON object entries whose placeholders are not all set are
left out, so a body can carry alternatives (`{"token": "{key}", "password": "{password}"}`).
A missing placeholder in `path` is an error.

#### Dependent requests

`each` is jq over the responses of the plain requests (those without `each`) and must give
an array. The request is sent once per element, with `{item}` set to it, and its response is
the array of results, in the same order:

```yaml
requests:
  sections: /library/sections
  counts:
    path: /library/sections/{item}/all
    query: { X-Plex-Container-Size: "0" }
    each: "[.sections.MediaContainer.Directory[].key]"
fields:
  - label: Movies
    value: >-
      [.sections.MediaContainer.Directory, .counts] | transpose
      | map(select(.[0].type == "movie") | .[1].MediaContainer.totalSize) | add
```

Responses that are not JSON are exposed as a string. A 401, 403 or 409 response triggers
a new login (when there is one) and a single retry.

### Login

For services that hand out a session. Exactly one of `token`, `cookie`, `header` says where
the session comes from; it is then available as `{token}` and cached until rejected.

The login runs when its `path` is complete and, if its query or body use placeholders, at least
one of those entries is complete. Incomplete entries are left out, as in any request, so one
login can take alternatives:

```yaml
# API key, or username + password
login: { path: /api/Account/login, method: POST, json: { apiKey: "{key}", username: "{username}", password: "{password}" }, token: .token }
```

A service usable with or without auth (or with an API key instead of a login) just leaves the
options out: the login is skipped, `{token}` is then unset and headers using it are skipped.

The token can also come from a plain-text body: jq then sees the body as a string
(`token: 'capture("Auth=(?<t>.+)").t'`).

Some services answer an expired session with a 200 and an error body. `expired` is jq
evaluated on each response; when it is true, the engine logs in again and retries once:

```yaml
login: { …, expired: '.error.code? == 1' }
```

```yaml
# token from the JSON response (jq)
login: { path: /api/auth, method: POST, json: { password: "{key}" }, token: .session.sid }
headers: { X-FTL-SID: "{token}" }

# session cookie: {token} is the whole name=value pair; `*` wildcards in the name
login: { path: /api/v2/auth/login, method: POST, form: { username: "{username}", password: "{password}" }, cookie: "*SID*" }
headers: { Cookie: "{token}" }

# response header, whatever the status (Transmission answers 409 with it)
login: { path: /transmission/rpc, method: POST, header: X-Transmission-Session-Id }
headers: { X-Transmission-Session-Id: "{token}" }
```

### Fields

`value` is a [jq](https://jqlang.org/manual/) expression over the object of all responses;
its first output is shown. Errors, `null` or no output show `–`. Widget options (with
defaults applied) are available as `$options`, e.g. `.ifaces[] | select(.name == $options.wan)`.

A field can also set:

- `key`: its gethomepage name (`blocked_percent`), so `fields` lists written for gethomepage
  select it. `fields` matches labels and keys, ignoring case.
- `hidden: true`: only shown when the user lists it in `fields`.
- `when`: jq over the responses, like `value` (`$options` available, values are strings).
  When it gives false or null the field is hidden as above: `when: '$options.version == "2"'`
  for a field that only exists in one API version, `when: '.job.state == "Printing"'` for one
  that only makes sense in one state.

`format` is one of:

| format | input | example |
|---|---|---|
| `number` (default) | number | `12 345`, `1.5` |
| `bytes` | bytes | `1.5 GiB` |
| `rate` | bytes per second | `2.0 MiB/s` |
| `bitrate` | bits per second | `250 Mbps` |
| `percent` | 0–100 | `42%` |
| `duration` | seconds | `3d 4h` |
| `ago` | unix timestamp (s) | `2h 5m ago` — from ISO dates: `.date[:19] + "Z" \| fromdateiso8601` |
| `text` | anything | as is |

Booleans show as `yes` / `no`. A string that parses as a number is formatted as one (except with `text`); any other
string is shown as is. Whole numbers get a thin-space thousands separator, others one decimal.

Useful jq: `length`, `[.items[] | select(.active)] | length`, `[.items[].size] | add`,
`.a / .b * 100`, `.x // 0` (default when missing), `(.list // []) | length`.
