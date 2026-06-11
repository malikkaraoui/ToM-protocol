# Spécifications du protocole ToM

Spécifications normatives permettant d'implémenter ToM sans lire le code Rust, et de **vérifier** une implémentation contre des test vectors.

| Document | Contenu |
|---|---|
| [`tom-wire-v1.md`](tom-wire-v1.md) | Format wire : Envelope MessagePack, signature, types de message, TTL, règles de relais |
| [`tom-crypto-v1.md`](tom-crypto-v1.md) | Cryptographie : Ed25519/X25519, HKDF, XChaCha20-Poly1305, ordre chiffre-puis-signe, sender keys |
| [`vectors/tom-vectors-v1.json`](vectors/tom-vectors-v1.json) | Test vectors (octets attendus, valeurs intermédiaires) |

## Test vectors

Générés et **auto-vérifiés** contre l'implémentation de référence :

```bash
cargo run -p tom-protocol --example gen_test_vectors > docs/spec/vectors/tom-vectors-v1.json
```

Le générateur panique (et n'émet rien) si un vector ne correspond plus à l'implémentation. Toute évolution du wire format exige une nouvelle version de spec (`v2`) — la v1 est figée.

## Hors périmètre v1

Discovery (gossip, PeerAnnounce, DHT, relais) : prévu dans `tom-discovery-v1.md` (backlog S3.4). Les invariants wire hérités d'iroh (`_iroh`, ALPN, SNI) sont documentés dans `docs/FORK-GOVERNANCE.md`.
