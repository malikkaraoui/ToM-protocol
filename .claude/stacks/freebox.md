---
stack: freebox
applies_to: ["*freebox*", "*fbx*", "src/gateway/**", "src/freebox/**"]
loads_from: CLAUDE.md §0 (Contexte projet)
status: stable
---

# Stack — Freebox API

> Intégration de l'API Freebox OS (v14+).
> Workflow complet : discovery → autorisation → session → appels authentifiés.
> Référence source : `tom-protocol/docs/freebox-sdk-workflow.md`

---

## Variables d'environnement

```bash
FREEBOX_URL=http://mafreebox.freebox.fr   # ou IP LAN directe
FREEBOX_APP_ID=com.monapp                 # stable, jamais changer
FREEBOX_TOKEN_FILE=~/.config/monapp/freebox_token.json
```

> `mafreebox.freebox.fr` ne résout **que depuis le LAN** de la box.
> Pour usage externe ou CI, stocker l'IP LAN directe.

---

## Persistance du token

**Fichier** : `$FREEBOX_TOKEN_FILE` (défaut : `~/.config/<app>/freebox_token.json`)
**Permissions** : `0600` — obligatoire, refus d'écriture sinon.

```json
{
  "app_id": "com.monapp",
  "app_token": "xxxxxxxxxxxxx",
  "freebox_url": "http://mafreebox.freebox.fr"
}
```

Règles :
- `app_token` **permanent** — ne jamais régénérer sans raison
- Jamais committer le fichier token (`.gitignore` + `.claudeignore`)
- `app_id` doit être **stable** sur la durée de vie de l'app

---

## Cycle de vie session

```
1. Discovery    GET /api_version                     → construire api_base
2. Auth (1x)   POST /login/authorize/               → app_token + track_id
3. Poll (1x)   GET /login/authorize/{track_id}      → attendre "granted"
4. Save (1x)   écrire token.json (chmod 0600)
──────── répété à chaque exécution ────────
5. Challenge   GET /login/                           → challenge string
6. Session     POST /login/session/                 → session_token (éphémère)
7. API calls   X-Fbx-App-Auth: {session_token}
```

### Construction de l'api_base

```javascript
const major = apiVersion.split('.')[0]         // "14"
const apiBase = apiBaseUrl + 'v' + major       // "/api/v14"
const baseUrl = freeboxUrl + apiBase           // "http://mafreebox.../api/v14"
```

### Calcul du password (étape 6)

```javascript
// Node.js ESM
import { createHmac } from 'crypto'
const password = createHmac('sha1', app_token).update(challenge).digest('hex')
```

```python
# Python
import hmac, hashlib
password = hmac.new(app_token.encode(), challenge.encode(), hashlib.sha1).hexdigest()
```

```rust
// Rust (hmac + sha1 crates)
use hmac::{Hmac, Mac};
use sha1::Sha1;
let mut mac = Hmac::<Sha1>::new_from_slice(app_token.as_bytes())?;
mac.update(challenge.as_bytes());
let password = hex::encode(mac.finalize().into_bytes());
```

---

## Endpoints clés

| Endpoint | Méthode | Description |
|---|---|---|
| `/api_version` | GET | Discovery — api_base + version |
| `/login/` | GET | Challenge pour session |
| `/login/session/` | POST | Ouvrir session → session_token |
| `/login/authorize/` | POST | Demande autorisation app (1x) |
| `/login/authorize/{track_id}` | GET | Poll approbation LCD |
| `/connection/` | GET | IP publique + bande passante |
| `/lan/browser/pub/` | GET | Appareils LAN actifs |
| `/fw/redir/` | GET/POST | Règles NAT |
| `/fw/redir/{id}` | DELETE | Supprimer règle NAT |

---

## Format de réponse standard

```json
{
  "success": true,
  "result": { ... },
  "msg": "Description erreur",
  "error_code": "auth_required | insufficient_rights | invalid_token | resource_not_found"
}
```

Toujours vérifier `success` avant de lire `result`.

---

## Gestion des erreurs

| `error_code` | Cause | Action |
|---|---|---|
| `auth_required` | Session expirée | Répéter étapes 5-6 |
| `insufficient_rights` | Permission manquante | Paramètres Freebox → Applications |
| `invalid_token` | Token révoqué ou corrompu | Répéter étapes 2-4 (`freebox-init`) |
| `resource_not_found` | ID inexistant | Vérifier l'id (règle NAT, etc.) |

Pattern retry session :
```javascript
async function callApi(path, opts = {}) {
  let res = await fetch(baseUrl + path, withAuth(opts, sessionToken))
  if (!res.ok || (await res.json()).error_code === 'auth_required') {
    sessionToken = await openSession()
    res = await fetch(baseUrl + path, withAuth(opts, sessionToken))
  }
  return res
}
```

---

## Pattern NAT — exposer un service

```javascript
// Exposer port 3340 UDP+TCP depuis LAN vers Internet
for (const proto of ['udp', 'tcp']) {
  const rules = await GET('/fw/redir/')
  const existing = rules.find(r => r.wan_port_start === port && r.ip_proto === proto)

  if (existing) {
    if (existing.lan_ip === lanIp) continue    // déjà correct
    if (force) await DELETE(`/fw/redir/${existing.id}`)
    else { console.warn(`Conflit ${proto}:${port}`); continue }
  }

  await POST('/fw/redir/', { enabled: true, ip_proto: proto,
    wan_port_start: port, wan_port_end: port, lan_ip: lanIp, lan_port: port })
}
const conn = await GET('/connection/')
console.log(`NAT: ${conn.ipv4}:${port} → ${lanIp}:${port}`)
```

---

## Règles de sécurité (§22)

- `app_token` → jamais en dur dans le code, toujours lu depuis le fichier token
- Fichier token → `0600`, hors du repo (`**freebox_token.json` dans `.gitignore`)
- `.claudeignore` : ajouter le même pattern
- Pour les tests : mocker le client Freebox, jamais appeler la vraie box en CI
- En remote/VPN : utiliser l'IP LAN directe, pas `mafreebox.freebox.fr`

---

## Auto-détection IP LAN

```python
hosts = GET /lan/browser/pub/
keywords = ["debian", "nas", "vm", "server", "freebox-server"]

candidates = [
  (h.primary_name, c.addr)
  for h in hosts
  for c in h.l3connectivities
  if c.af == "ipv4" and c.active
     and any(kw in h.primary_name.lower() for kw in keywords)
]

if len(candidates) == 1:
    return candidates[0][1]
elif len(candidates) > 1:
    raise "Plusieurs candidats — spécifier --lan-ip"
else:
    raise "Aucun hôte détecté — spécifier --lan-ip"
```
