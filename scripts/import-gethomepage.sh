#!/usr/bin/env bash
# Converts a gethomepage.dev config folder into a single config.yaml for this project.
#
#   scripts/import-gethomepage.sh /path/to/gethomepage/config > config.yaml
#
# Reads services.yaml, bookmarks.yaml, settings.yaml, widgets.yaml, docker.yaml and
# kubernetes.yaml (all optional). The config goes to stdout, warnings about what has no
# equivalent go to stderr. Then run `homepage check`: it lists anything left to fix. (Homepage
# also reads most gethomepage service keys directly; this script handles the other files.)
#
# Needs jq, and yq (mikefarah) or Docker (falls back to the mikefarah/yq image).
set -euo pipefail

dir=${1:?usage: $0 <gethomepage config dir> > config.yaml}
[ -d "$dir" ] || { echo "$dir: not a directory" >&2; exit 1; }
command -v jq >/dev/null || { echo "jq is required" >&2; exit 1; }

yq_() {
  if command -v yq >/dev/null; then
    yq "$@"
  else
    docker run --rm -i mikefarah/yq "$@"
  fi
}

# Each file as JSON, null when missing or empty.
json() {
  if [ -s "$dir/$1" ]; then yq_ -o=json '.' < "$dir/$1"; else echo null; fi
}

input=$(jq -n \
  --argjson services "$(json services.yaml)" \
  --argjson bookmarks "$(json bookmarks.yaml)" \
  --argjson settings "$(json settings.yaml)" \
  --argjson widgets "$(json widgets.yaml)" \
  --argjson docker "$(json docker.yaml)" \
  --argjson kubernetes "$(json kubernetes.yaml)" \
  '$ARGS.named')

result=$(jq '
  def warn($msg): .warnings += [$msg];

  # {{HOMEPAGE_VAR_X}} -> ${HOMEPAGE_VAR_X}: the same environment variables keep working.
  # {{HOMEPAGE_FILE_X}} holds a file path in the environment; it is resolved now when set.
  def secrets:
    gsub("{{(?<n>HOMEPAGE_VAR_[A-Za-z0-9_]+)}}"; "${\(.n)}")
    | gsub("{{(?<n>HOMEPAGE_FILE_[A-Za-z0-9_]+)}}";
        if $ENV[.n] then "${file:\($ENV[.n])}" else "${file:/run/secrets/CHANGE_ME_\(.n)}" end);

  # mdi-, si- and sh- icons become CDN URLs; "name.png" becomes the dashboard-icons name.
  def icon:
    sub("-#[0-9a-fA-F]{3,8}$"; "")
    | if startswith("mdi-") then "https://cdn.jsdelivr.net/npm/@mdi/svg/svg/\(.[4:]).svg"
      elif startswith("si-") then "https://cdn.jsdelivr.net/npm/simple-icons/icons/\(.[3:]).svg"
      elif startswith("sh-") then "https://cdn.jsdelivr.net/gh/selfhst/icons/svg/\(.[3:] | sub("\\.(png|svg|webp)$"; "")).svg"
      elif test("^https?://") then .
      else sub("\\.(png|svg|webp)$"; "") end;

  # Every input is checked for shape: anything unexpected is skipped, never a crash.
  def single: if type == "object" then to_entries[0] else empty end;
  def items: if type == "array" then .[] else empty end;
  def obj: if type == "object" then . else {} end;

  . as $in
  | {config: {}, warnings: []}

  # settings.yaml
  | ($in.settings | obj) as $settings
  | if $settings != {} then
      .config.title = ($settings.title | if type == "string" then . else null end)
      | if ($settings.theme | IN("dark", "light")) then .config.theme = $settings.theme else . end
      | ([$settings | keys[] | select(IN("title", "theme") | not)]) as $skip
      | if ($skip | length) > 0 then warn("settings.yaml: ignored \($skip | join(", "))") else . end
    else . end

  # widgets.yaml: datetime, search and resources have an equivalent
  | if ($in.widgets | type) == "array" then
      reduce ($in.widgets | items | single) as $w (.config.widgets = [];
        if $w.key == "datetime" then .config.widgets += ["clock"]
        elif $w.key == "resources" then .config.widgets += ["resources"]
        elif $w.key == "search" then
          .config.widgets += ["search"]
          | ($w.value | obj) as $o
          | ($o.provider | if type == "array" then .[0] else . end) as $p
          | if ($p | IN("duckduckgo", "google", "bing")) then .config.search = $p
            elif ($o.url | type) == "string" then .config.search = "\($o.url){q}"
            else warn("widgets.yaml: search provider \($p) has no equivalent, using duckduckgo") end
        else warn("widgets.yaml: no \($w.key) widget") end)
    else . end

  # docker.yaml
  | if ($in.docker | type) == "object" then
      reduce ($in.docker | to_entries[] | .value |= obj) as $d (.;
        if $d.value.socket then .config.docker[$d.key] = "unix://\($d.value.socket)"
        elif $d.value.host then
          .config.docker[$d.key] = "tcp://\($d.value.host):\($d.value.port // 2375)"
          | if $d.value.tls then warn("docker.yaml: \($d.key): TLS client certificates are not supported") else . end
        else warn("docker.yaml: \($d.key): no host or socket") end)
    else . end
  | ($in.docker | obj | keys) as $hosts

  # kubernetes.yaml
  | if (($in.kubernetes | obj).mode // "disabled") != "disabled" then .config.kubernetes = true else . end

  # services.yaml: [{Group: [{Service: {...}} | {SubGroup: [...]}]}]
  | reduce ($in.services | items | single) as $g (.;
      reduce ($g.value | items | single) as $s (.;
        if ($s.value | type) == "array" then
          warn("services.yaml: nested group \($g.key) / \($s.key) is not supported, skipped")
        else
          ($s.value | obj) as $v
          | "services.yaml: \($g.key) / \($s.key)" as $at
          | ({name: $s.key}
             + (if $v.href then {url: $v.href} else {} end)
             + (if $v.description then {description: $v.description} else {} end)
             + (if ($v.icon | type) == "string" then {icon: ($v.icon | icon)} else {} end)) as $svc
          # status: siteMonitor, else the container
          | (if $v.siteMonitor then {check: $v.siteMonitor}
             elif $v.container and ($hosts | index($v.server // "")) then {check: "docker://\($v.server)/\($v.container)"}
             else {} end) as $check
          | if $v.ping then warn("\($at): no ICMP ping, use check: tcp://host:port") else . end
          | if $v.container and ($check.check // "" | startswith("docker://") | not) and ($v.siteMonitor | not)
            then warn("\($at): docker server \($v.server // "?") is not in docker.yaml, no check") else . end
          # widget: scalars become strings, fields use other names here
          | (($v.widget // ([$v.widgets | items][0])) | obj) as $w
          | if ([$v.widgets | items] | length) > 1 then warn("\($at): only the first widget is kept") else . end
          | (if $w.type then
              {widget: (($w | with_entries(select(.key != "fields" and (.value | type | IN("string", "number", "boolean")))
                                           | .value |= tostring))
                        + (if ($w.fields | type) == "array" then {fields: [$w.fields[] | tostring]} else {} end))}
             else {} end) as $widget
          | ([$w | to_entries[] | select(.value | type | IN("array", "object")) | .key | select(. != "fields")]) as $complex
          | if ($complex | length) > 0 then warn("\($at): widget options dropped: \($complex | join(", "))") else . end
          | ([$v | keys[] | select(IN("href", "description", "icon", "siteMonitor", "ping", "server", "container", "widget", "widgets") | not)]) as $skip
          | if ($skip | length) > 0 then warn("\($at): ignored \($skip | join(", "))") else . end
          | .groups[$g.key] += [$svc + $check + $widget]
        end))
  | .config.groups = [(.groups // {}) | to_entries[] | {name: .key, services: .value}]
  | del(.groups)

  # bookmarks.yaml: [{Group: [{Link: [{href, icon, abbr}]}]}]
  | .config.bookmarks = [$in.bookmarks | items | single
      | {name: .key, links: [.value | items | single
          | (.value | if type == "array" then .[0] else . end | obj) as $l
          | {name: .key, url: $l.href}
            + (if ($l.icon | type) == "string" then {icon: ($l.icon | icon)} else {} end)]}]

  | if [.config.bookmarks[].links[] | select(.url == null)] | length > 0
    then warn("bookmarks.yaml: links without href dropped")
         | .config.bookmarks[].links |= map(select(.url != null)) else . end
  | .config |= (with_entries(select(.value != null and .value != [] and .value != {})) | walk(if type == "string" then secrets else . end))
' <<< "$input") || { echo "conversion failed: please open an issue with the jq error above" >&2; exit 1; }

jq -r '.warnings[] | "warning: " + .' <<< "$result" >&2
if grep -q 'CHANGE_ME_' <<< "$result"; then
  echo "warning: some {{HOMEPAGE_FILE_*}} variables are not set here: fix the \${file:/run/secrets/CHANGE_ME_*} paths" >&2
fi
echo "# Imported from $dir by scripts/import-gethomepage.sh"
jq '.config' <<< "$result" | yq_ -P -p=json '.'
