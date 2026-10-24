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

## Endpoints

- `/` : le dashboard
- `/api/state` : statuts et ressources en JSON
- `/healthz` : renvoie `ok`
