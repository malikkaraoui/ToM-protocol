# Relay Discovery Service (MVP)

Service HTTP minimal pour Option C.

## Endpoints

- `GET /health` → état du service discovery
- `GET /relays` → liste des relays healthy au format:
- `GET /metrics` → compteurs opérationnels (JSON)
- `GET /status` → snapshot agrégé (relays + cache + compteurs)

```json
{
  "relays": [
    {
      "url": "https://relay-eu.tom-protocol.org",
      "region": "eu-west",
      "load": 0.3,
      "latency_hint_ms": 50
    }
  ],
  "ttl_seconds": 300
}
```

## Catalogue des relays

Par défaut, le service lit:

- `tools/relay-discovery/relays.example.json`

Variable d’environnement pour pointer un fichier custom:

- `RELAY_DISCOVERY_RELAYS_FILE=/path/to/relays.json`

## Démarrage

- `node tools/relay-discovery/server.mjs`

Variables d’environnement optionnelles:

- `RELAY_DISCOVERY_PORT` (défaut `8080`)
- `RELAY_DISCOVERY_CHECK_TIMEOUT_MS` (défaut `2500`)
- `RELAY_DISCOVERY_CACHE_TTL_MS` (défaut `30000`)
- `RELAY_DISCOVERY_RESPONSE_TTL_SECONDS` (défaut `300`)

## Santé relay

Le service teste chaque relay via:

1. `GET <relay>/health`
2. fallback `GET <relay>/healthz`

Le relay est publié uniquement si une réponse healthy est obtenue.

## Métriques discovery

`GET /metrics` expose un JSON léger pour supervision locale:

- volume de requêtes (`requests_total`, `health_requests`, `relays_requests`)
- checks de santé relay (`relay_checks_total`)
- efficacité cache (`cache_hits`, `cache_misses`)
- état cache courant (`cached_relays`, `last_refresh_at`)
- dernière erreur (`last_error`)

## Snapshot agrégé

`GET /status` renvoie une vue unique prête pour dashboard minimal :

- liste des relays healthy actuels
- `relay_count`
- TTL de publication (`ttl_seconds`)
- état cache (âge, hits/misses, last refresh)
- compteurs principaux
