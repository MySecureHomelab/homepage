# homepage

Un dashboard de homelab en Rust (actix-web), inspiré de [gethomepage.dev](https://gethomepage.dev). La roadmap complète est dans [FEATURES.md](FEATURES.md).

## Lancer

```bash
cp config.example.yaml config.yaml
cargo run --release
```

Ouvre http://localhost:3000.

- `homepage check` : valide la config et quitte (code 1 en cas d'erreur).
- `HOMEPAGE_CONFIG` : chemin de la config (`config.yaml` par défaut).
- `HOMEPAGE_ADDR` : adresse d'écoute (`0.0.0.0:3000` par défaut).
- La config est rechargée à chaud. Si une modif est invalide, l'erreur s'affiche dans la page (avec la ligne fautive) et la dernière config valide reste servie. Les services ajoutés sont vérifiés tout de suite.
- `/` place le curseur dans la recherche, `Échap` l'en sort.

## Docker

```bash
docker build -t homepage .
docker run -p 3000:3000 -v ./config.yaml:/config/config.yaml:ro homepage
```

## Découverte automatique

Les services peuvent venir des conteneurs Docker et des Ingress Kubernetes. Les labels et annotations sont ceux de gethomepage.dev, donc une install existante marche telle quelle. La découverte tourne toutes les 15 s.

```yaml
docker:
  local: unix:///var/run/docker.sock
  nas: tcp://nas.lan:2375
kubernetes: true
```

### Docker

Un conteneur apparaît s'il a le label `homepage.name`. Labels optionnels : `homepage.group` (par défaut `Docker`), `homepage.href` (ou `homepage.url`), `homepage.icon`, `homepage.description`, `homepage.check`.

```yaml
services:
  jellyfin:
    image: jellyfin/jellyfin
    labels:
      homepage.name: Jellyfin
      homepage.group: Media
      homepage.href: https://jellyfin.lan
      homepage.icon: jellyfin
```

Sans `homepage.check`, le service est « up » si son conteneur tourne et n'est pas `unhealthy`. Ce check est aussi utilisable à la main dans la config : `check: docker://nas/jellyfin`.

Un service déclaré dans la config avec le même nom et le même groupe a priorité sur celui découvert. Si un hôte Docker ne répond plus, ses services restent affichés en « Down ».

L'image tourne en non-root et ne peut pas lire `docker.sock` directement. Le plus propre (et le plus sûr) est de passer par un proxy en lecture seule, par exemple [docker-socket-proxy](https://github.com/Tecnativa/docker-socket-proxy) avec `CONTAINERS=1`, puis de le déclarer avec `tcp://socket-proxy:2375`.

### Kubernetes

Un Ingress apparaît s'il a l'annotation `gethomepage.dev/enabled: "true"`. Annotations optionnelles : `gethomepage.dev/name` (par défaut le nom de l'Ingress), `group` (par défaut `Kubernetes`), `href`, `icon`, `description`, `check`.

Sans `href`, l'URL est déduite de la première règle : son host et son path, en https si le TLS couvre ce host. Cette URL sert aussi de check.

Le compte de service doit pouvoir lister les Ingress :

```yaml
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRole
metadata:
  name: homepage
rules:
  - apiGroups: ["networking.k8s.io"]
    resources: ["ingresses"]
    verbs: ["list"]
```

## Endpoints

- `/` : le dashboard
- `/api/state` : statuts et ressources en JSON
- `/healthz` : renvoie `ok`
