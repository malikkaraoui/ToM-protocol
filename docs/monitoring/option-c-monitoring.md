# Option C — Monitoring MVP (Prometheus + alertes)

Date: 2026-03-05

## Objectif

Fournir une observabilité minimale exploitable pour la flotte relay:

- état up/down des relays
- dérive de latence (proxy via scrape p95)
- hausse du taux d'erreur (drop rate)

## Fichiers

- `deploy/monitoring/prometheus.yml`
- `deploy/monitoring/alerts.yml`
- `deploy/monitoring/grafana-dashboard-option-c.json`

## Principes de labels

Autorisés (basse cardinalité):

- `relay`
- `region`

Interdits (haute cardinalité):

- `node_id`
- `message_id`
- `user_id`

## Déploiement rapide

1. Copier `deploy/monitoring/prometheus.yml` dans votre instance Prometheus.
2. Copier `deploy/monitoring/alerts.yml` et référencer le fichier dans `rule_files`.
3. Vérifier que chaque relay expose bien ses métriques sur `:9090/metrics`.
4. Recharger Prometheus.
5. Importer `deploy/monitoring/grafana-dashboard-option-c.json` dans Grafana.

## Dashboard Grafana (C3.2)

Le dashboard **ToM Option C — Relay Monitoring** inclut:

- Relays UP (count)
- Drop rate global (5m)
- Scrape p95 (5m)
- Trafic relay (bytes/s in/out)
- Drop rate par relay
- Churn de connexions (accepts/disconnects)
- Paquets recv vs dropped
- Top relays par throughput

Variables:

- `relay` (multi-select)
- `region` (multi-select)

## Validation

- Vérifier la target `tom-relay-metrics` en état `UP`.
- Simuler une panne d’un relay et observer `TomRelayDown` (>2m).
- Générer de la charge / perturbation et observer `TomRelayDropRateHigh`.

## Notes importantes

- L’alerte latence est une approximation (`scrape_duration_seconds` p95).
- Pour une latence réseau/transport plus fidèle, ajouter une histogram metric côté relay.
- Discovery expose actuellement un endpoint JSON (`/metrics`) et non un format Prometheus natif.
  Un blackbox exporter est proposé en commentaire dans `prometheus.yml`.
