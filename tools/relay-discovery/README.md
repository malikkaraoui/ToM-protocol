# Relay Discovery Service (MVP)

Service HTTP minimal pour Option C.

## Endpoints

- `GET /health` → état du service discovery
- `GET /relays` → liste des relays healthy au format:

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
