# Relay Discovery

## Vue d’ensemble

Le client peut récupérer dynamiquement une liste de relays via un service discovery HTTP (`/relays`).

Le payload attendu:

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

## Fallback Relays

Si le discovery service est inaccessible, le client utilise automatiquement une liste de relays publics hardcodés:

- `https://relay-eu.tom-protocol.org`
- `https://relay-us.tom-protocol.org`
- `https://relay-asia.tom-protocol.org`

Cette liste est maintenue dans `crates/tom-transport/src/config.rs` (`DEFAULT_RELAY_URLS`).

## Override fallback list

Pour utiliser vos propres relays en priorité:

```rust
TomNodeConfig::new()
    .relay_urls(vec![
        "https://mon-relay.example.com".parse().unwrap(),
    ])
    .bind().await?;
```

## Priorité de résolution

1. Relays statiques fournis via `relay_urls` / `TOM_RELAY_URLS`
2. Relays discovery (`relay_discovery_url`)
3. Fallback hardcodé si la liste reste vide

Le fallback est donc un filet de sécurité pour éviter un blocage total quand discovery est indisponible.
