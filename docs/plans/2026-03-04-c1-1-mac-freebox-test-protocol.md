# C1.1 — Protocole de test terrain (MacBook Air ↔ Freebox)

Objectif: valider le durcissement startup relay en conditions proches prod, avec checks de santé et connectivité e2e.

## Pré-requis

- Freebox/NAS accessible en SSH
- Binaire `tom-relay` disponible sur la Freebox
- MacBook Air avec repo ToM et `cargo` fonctionnel
- Port relay ouvert/redirigé (ex: 3340) + metrics (ex: 9090)

## 1) Validation config fail-fast (locale)

Exécuter localement (Mac) les tests C1.1:

- `cargo test -p tom-relay test_validate_startup`
- `cargo test -p tom-relay`

Attendu:
- erreurs explicites en cas de config invalide
- démarrage OK pour config valide

## 2) Sanity Freebox: démarrage relay + santé

### 2.1 Déploiement config

Préparer deux configs:

1. `relay-dev.toml` (HTTP local)
2. `relay-prod.toml` (TLS + metrics)

### 2.2 Startup checks

- Démarrer relay sur Freebox avec la config choisie
- Vérifier endpoint santé:
  - `GET /healthz` (et `/health` si activé)
- Vérifier metrics:
  - `GET /metrics`

Attendu:
- HTTP 200 pour santé
- métriques Prometheus lisibles

## 3) E2E Mac -> Freebox relay

Depuis Mac:

- lancer un scénario `tom-stress` pointé sur le relay Freebox
- tester envoi/réception 1-to-1

Attendu:
- messages livrés
- pas de crash relay

## 4) Test de robustesse startup

### 4.1 Config invalide volontaire

Injecter successivement:

- aucun service activé
- TLS manuel sans cert/key
- conflit de ports (relay/metrics)

Attendu:
- process refuse de démarrer
- message d’erreur clair (fail-fast)

### 4.2 Config valide restaurée

Relancer avec config saine.

Attendu:
- démarrage immédiat
- santé OK

## 5) Test de continuité (rolling/restart)

- connecter au moins 2 clients
- redémarrer le relay
- observer reconnexion clients

Attendu:
- reconnexion automatique
- service restauré sans intervention manuelle lourde

## 6) Compte-rendu attendu

Noter dans un fichier de session:

- version binaire `tom-relay`
- config utilisée (dev/prod)
- timestamps de démarrage/arrêt
- résultats santé/metrics
- résultats tom-stress
- erreurs observées + logs associés

## Critères de succès C1.1 terrain

- [ ] startup refuse proprement les configs invalides
- [ ] startup accepte les configs valides
- [ ] endpoint santé stable
- [ ] métriques accessibles
- [ ] e2e Mac↔Freebox passe
- [ ] redémarrage relay ne bloque pas la reprise
