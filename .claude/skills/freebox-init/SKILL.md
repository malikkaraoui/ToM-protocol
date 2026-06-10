---
name: freebox-init
description: "Bootstrap l'autorisation d'une app Freebox : app_token acquisition (1x), persistance sécurisée, vérification de session. Déclenché pour tout nouveau projet Freebox ou token révoqué."
figure: Xavier
triggers: freebox, freebox-init, fbx, app_token, mafreebox, freebox token
---

# Freebox Init

> Un bouton LCD. Une seule pression. Un token permanent.
> Xavier appelle ça de la confiance. Toi, tu appelleras ça de l'auth.
> *— HMAC-SHA1 is the new handshake.*

Bootstrap complet de l'autorisation Freebox pour un nouveau projet ou token révoqué.

## Quand se déclencher

- Nouveau projet qui intègre l'API Freebox
- `invalid_token` ou `auth_required` persistant après retry session
- `/freebox-init` explicite
- Mention de "app_token", "autoriser l'application", "appuyer sur le LCD"

## Pré-requis

Récupérer auprès de l'utilisateur :

1. **`app_id`** — identifiant stable (ex: `com.monapp`, `fr.malik.tom-gateway`). Jamais changer après première auth.
2. **`app_name`** — nom affiché sur l'écran LCD Freebox (ex: `Mon Application`)
3. **`app_version`** — version de l'app (ex: `1.0.0`)
4. **`device_name`** — hostname de la machine (ex: `mac-malik`)
5. **`freebox_url`** — défaut `http://mafreebox.freebox.fr`, ou IP LAN directe si hors réseau Freebox
6. **`token_file`** — chemin de persistance (ex: `~/.config/monapp/freebox_token.json`)

## Procédure

### Étape 1 — Discovery

```http
GET {freebox_url}/api_version
```

Extraire et valider :
- `api_base_url` (ex: `/api/`)
- `api_version` (ex: `14.0`)

Construire `api_base` :
```
major = api_version.split('.')[0]
api_base = api_base_url + "v" + major    // "/api/v14"
base = freebox_url + api_base
```

Si la requête échoue (timeout, DNS) → demander l'IP LAN directe. `mafreebox.freebox.fr` ne résout que depuis le LAN.

### Étape 2 — Demande d'autorisation

```http
POST {base}/login/authorize/
Content-Type: application/json

{
  "app_id": "{app_id}",
  "app_name": "{app_name}",
  "app_version": "{app_version}",
  "device_name": "{device_name}"
}
```

Réponse attendue :
```json
{ "success": true, "result": { "app_token": "xxx", "track_id": 42 } }
```

Sauvegarder `app_token` et `track_id` en mémoire.

**Informer l'utilisateur :** "Appuyez sur ✔ sur l'écran LCD de votre Freebox maintenant."

### Étape 3 — Poll approbation LCD

```http
GET {base}/login/authorize/{track_id}
```

Algorithme : poll toutes les **2 secondes**, timeout **60 secondes**.

| `status` | Action |
|---|---|
| `pending` | Continuer à poller, afficher `⏳ En attente...` |
| `granted` | Continuer à l'étape 4 |
| `denied` | **Stopper** — "L'utilisateur a refusé sur l'écran LCD." |
| `timeout` | **Stopper** — "Pas de réponse. Relancer `/freebox-init`." |

### Étape 4 — Persistance sécurisée du token

Créer le répertoire parent si nécessaire, puis écrire :

```json
{
  "app_id": "{app_id}",
  "app_token": "{app_token}",
  "freebox_url": "{freebox_url}"
}
```

**Permissions obligatoires :**
```bash
chmod 0600 {token_file}
```

Vérifier avec `stat` que les permissions sont bien `600`. Si impossible → avertir et stopper.

### Étape 5 — Vérification end-to-end

Ouvrir une session complète pour valider le token :

```http
# 5a. Challenge
GET {base}/login/

# 5b. Password
password = HMAC-SHA1(key=app_token, message=challenge) → hex lowercase

# 5c. Session
POST {base}/login/session/
{ "app_id": "{app_id}", "password": "{password}" }
```

Si `success: true` → récupérer `session_token`.

Valider avec un appel réel :
```http
GET {base}/connection/
X-Fbx-App-Auth: {session_token}
```

Afficher l'IP publique en confirmation : "✅ Autorisation réussie. IP publique : {ipv4}"

### Étape 6 — .gitignore + .claudeignore

Vérifier/ajouter dans `.gitignore` et `.claudeignore` :
```gitignore
# Freebox token (contient app_token permanent)
**/freebox_token.json
*.freebox_token.json
```

### Étape 7 — Rapport final

```
✅ Freebox init terminé

app_id      : {app_id}
token_file  : {token_file} (0600)
api_base    : {base}
IP publique : {ipv4}

Prochaine étape : utiliser le stack `freebox` pour les appels API.
```

## Règles

- `app_id` doit être **identique** à chaque exécution — c'est l'identifiant côté Freebox
- Ne jamais régénérer `app_token` sans raison (chaque auth demande une validation LCD)
- Si `invalid_token` → répéter depuis l'étape 2 (pas besoin de refaire discovery)
- Pour les tests/CI : mocker le client Freebox, ne pas appeler la vraie box
- §5 prime : ne pas inventer de champs API non documentés

## Erreurs fréquentes

| Symptôme | Cause | Fix |
|---|---|---|
| DNS timeout | Hors réseau Freebox | Utiliser l'IP LAN directe |
| `status: denied` | Utilisateur a appuyé ✗ | Relancer étape 2 |
| `status: timeout` | LCD non surveillé | Relancer étape 2 |
| `invalid_token` au login | Token corrompu/révoqué | Répéter étapes 2-4 |
| Permissions refusées sur token_file | Répertoire inexistant ou `~` non résolu | Créer le répertoire, utiliser chemin absolu |
