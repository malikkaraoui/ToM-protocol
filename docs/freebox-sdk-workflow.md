# Freebox API — Workflow d'intégration

> Référence extraite de `crates/tom-gateway/`. Réutilisable pour tout projet tiers.
> Toutes les requêtes HTTP, l'ordre d'appels, la gestion des tokens.

---

## Vue d'ensemble

```
1. Discover       GET /api_version                     → base_url + api_base
2. Authorize      POST /login/authorize/               → app_token + track_id  (1x)
3. Poll approval  GET /login/authorize/{track_id}      → attendre "granted"    (1x)
4. Save token     ~/.tom/freebox_token.json (0600)     → persistance locale
                  ──────── session par session ────────
5. Open session   GET /login/ → challenge
                  POST /login/session/ → session_token
6. API calls      Toute route avec X-Fbx-App-Auth: {session_token}
```

---

## Étape 1 — Discovery

```
GET http://mafreebox.freebox.fr/api_version
```

Réponse :
```json
{
  "api_base_url": "/api/",
  "api_version": "14.0",
  "device_name": "Freebox Server",
  "uid": "...",
  "device_type": "..."
}
```

Construction du `api_base` :
```
major = api_version.split('.')[0]        // "14"
api_base = api_base_url + "v" + major    // "/api/v14"
```

Toutes les routes suivantes : `http://mafreebox.freebox.fr{api_base}{route}`

---

## Étape 2 — Autorisation (une seule fois)

```
POST {base_url}{api_base}/login/authorize/
Content-Type: application/json

{
  "app_id": "com.monapp",
  "app_name": "Mon Application",
  "app_version": "1.0.0",
  "device_name": "mon-hostname"
}
```

Réponse :
```json
{
  "success": true,
  "result": {
    "app_token": "xxxxxxxxxxxxx",
    "track_id": 42
  }
}
```

**L'utilisateur doit appuyer sur le bouton ✔ sur l'écran LCD de la Freebox.**

---

## Étape 3 — Polling de l'approbation

```
GET {base_url}{api_base}/login/authorize/{track_id}
```

Réponse :
```json
{
  "success": true,
  "result": {
    "status": "pending"   // pending | granted | denied | timeout
  }
}
```

Algorithme : poll toutes les 2s, max 60s.

| Status | Action |
|--------|--------|
| `pending` | Continuer à poller |
| `granted` | Sauvegarder `app_token`, continuer |
| `denied` | Erreur — l'utilisateur a refusé |
| `timeout` | Erreur — pas de réponse LCD |

---

## Étape 4 — Persistance du token

Fichier : `~/.tom/freebox_token.json`  
Permissions : **0600** (lecture/écriture owner uniquement — obligatoire)

Structure :
```json
{
  "app_id": "com.monapp",
  "app_token": "xxxxxxxxxxxxx",
  "freebox_url": "http://mafreebox.freebox.fr"
}
```

> `freebox_url` est stocké car le domaine `mafreebox.freebox.fr` ne résout que depuis le LAN de la box.
> Pour une utilisation distante, stocker l'IP LAN de la Freebox.

---

## Étape 5 — Ouverture de session (à chaque exécution)

### 5a. Récupérer le challenge

```
GET {base_url}{api_base}/login/
```

Réponse :
```json
{
  "success": true,
  "result": {
    "challenge": "VzhbtpR4r8CLaJle2QgJCS0sJBWahHZl",
    "logged_in": false
  }
}
```

### 5b. Calculer le mot de passe

```
password = HMAC-SHA1(key=app_token, message=challenge)
         → hex lowercase (40 caractères)
```

Exemple Python :
```python
import hmac, hashlib
password = hmac.new(app_token.encode(), challenge.encode(), hashlib.sha1).hexdigest()
```

Exemple Node.js (ESM) :
```javascript
import { createHmac } from 'crypto';
const password = createHmac('sha1', app_token).update(challenge).digest('hex');
```

### 5c. Ouvrir la session

```
POST {base_url}{api_base}/login/session/
Content-Type: application/json

{
  "app_id": "com.monapp",
  "password": "a3f4c2..."
}
```

Réponse :
```json
{
  "success": true,
  "result": {
    "session_token": "35JYdQSvkcBYK84IFMU/RvHPCGm7nhN2",
    "challenge": "...",
    "permissions": { ... }
  }
}
```

---

## Étape 6 — Appels API authentifiés

Chaque requête porte le header :

```
X-Fbx-App-Auth: {session_token}
```

Format de réponse standard :
```json
{
  "success": true | false,
  "result": { ... },
  "msg": "Description de l'erreur",
  "error_code": "auth_required | ..."
}
```

Si `"error_code": "auth_required"` → session expirée → répéter l'étape 5.

---

## Endpoints disponibles

### Informations de connexion

```
GET /connection/
```
```json
{
  "state": "up",
  "ipv4": "82.67.95.8",
  "ipv6": "2a01:...",
  "bandwidth_down": 200000000,
  "bandwidth_up": 50000000
}
```

### Liste des appareils LAN

```
GET /lan/browser/pub/
```
```json
[
  {
    "primary_name": "nas-debian",
    "active": true,
    "reachable": true,
    "l2ident": { "id": "A0:78:17:AD:92:6F", "type": "mac_address" },
    "l3connectivities": [
      { "addr": "192.168.0.83", "af": "ipv4", "active": true }
    ]
  }
]
```

### Règles NAT (redirection de ports)

**Lister :**
```
GET /fw/redir/
```

**Créer :**
```
POST /fw/redir/
Content-Type: application/json

{
  "enabled": true,
  "ip_proto": "udp",          // "udp" | "tcp"
  "wan_port_start": 3340,
  "wan_port_end": 3340,
  "lan_ip": "192.168.0.83",
  "lan_port": 3340,
  "src_ip": "",               // "" = toutes sources
  "comment": "Mon service (udp)"
}
```

Réponse : la règle créée avec son `id`.

**Supprimer :**
```
DELETE /fw/redir/{id}
```

---

## Pattern complet — Gestion NAT

Pour exposer un service (ex: port 3340 UDP+TCP) depuis le LAN vers internet :

```python
# 1. Récupérer les règles existantes
rules = GET /fw/redir/

# 2. Pour UDP et TCP :
for proto in ["udp", "tcp"]:
    existing = [r for r in rules if r.wan_port_start == port and r.ip_proto == proto]
    
    if existing:
        if existing[0].lan_ip == lan_ip:
            continue  # Déjà correct
        elif force:
            DELETE /fw/redir/{existing[0].id}
        else:
            print(f"ATTENTION: conflit sur {proto}:{port}")
            continue
    
    POST /fw/redir/ with proto, port, lan_ip

# 3. Afficher l'IP publique
conn = GET /connection/
print(f"NAT: {conn.ipv4}:{port} → {lan_ip}:{port}")
```

---

## Auto-détection de l'IP LAN

Pattern utilisé pour trouver automatiquement l'IP d'un serveur sur le LAN :

```python
hosts = GET /lan/browser/pub/
keywords = ["debian", "nas", "vm", "freebox-server"]

candidates = []
for host in hosts:
    name = host.primary_name.lower()
    if any(kw in name for kw in keywords):
        for conn in host.l3connectivities:
            if conn.af == "ipv4" and conn.active:
                candidates.append((host.primary_name, conn.addr))

if len(candidates) == 1:
    return candidates[0].addr
elif len(candidates) > 1:
    raise "Plusieurs candidats, spécifier --lan-ip"
else:
    raise "Aucun NAS détecté, spécifier --lan-ip"
```

---

## Gestion des erreurs

| `error_code` | Cause | Action |
|---|---|---|
| `auth_required` | Session expirée ou révoquée | Répéter l'étape 5 |
| `insufficient_rights` | Permission manquante | Vérifier permissions dans l'app Freebox |
| `invalid_token` | Token invalide ou révoqué | Répéter l'étape 2-4 |
| `resource_not_found` | Ressource inexistante (ex: règle NAT) | Vérifier l'id |

---

## CLI tom-gateway (référence)

```bash
# 1. Autoriser l'application (1x, appuyer sur LCD Freebox)
tom-gateway auth

# 2. Créer les règles NAT pour le relay
tom-gateway setup --port 3340

# 3. Vérifier l'état NAT + IP publique
tom-gateway status

# 4. Lister les appareils LAN
tom-gateway lan

# Options globales
tom-gateway --freebox-url http://192.168.0.254  # IP directe si mafreebox.freebox.fr non résolu
tom-gateway --token-file /chemin/vers/token.json
```

---

## Notes d'implémentation

- Le token `app_token` est **permanent** — ne pas le régénérer à chaque session
- Le `session_token` est **éphémère** — valide le temps de la session HTTP
- `mafreebox.freebox.fr` résout uniquement **depuis le LAN** — stocker l'IP pour usage externe
- Les permissions (accès NAT, LAN browser) sont configurables dans l'interface Freebox → Paramètres → Applications
- L'`app_id` doit être **stable** — c'est l'identifiant de l'application côté Freebox
