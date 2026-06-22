# Rapport de remontée — Diagnostic SSH WAN NAS/Freebox

**Date :** 2026-03-02  
**Auteur :** Malik + GitHub Copilot  
**Objectif :** remonter à Claude l’ensemble du diagnostic et des actions menées pour l’accès SSH distant au NAS.

---

## 1) Contexte

- NAS Debian (VM Freebox), IP LAN : `192.168.0.83`
- Mac local : `192.168.0.70`
- IP publique Freebox : `82.67.95.8`
- Objectif :
  1. diagnostiquer la capture réseau (`tcpdump`) côté NAS,
  2. valider la connectivité SSH en LAN,
  3. valider l’accès SSH depuis WAN (4G),
  4. identifier la cause du blocage sur le port 22.

---

## 2) Symptômes initiaux

- Sur NAS :
  - `tcpdump: command not found`
  - erreurs de commande liées à placeholder (`<interface>`) et saisie shell incomplète (`>`)
- Sur Mac (LAN) :
  - `ping` NAS OK
  - `nc -vz 192.168.0.83 22` OK
- Sur Mac (4G/WAN) :
  - échecs intermittents/`Network is unreachable` vers `82.67.95.8:22`

---

## 3) Diagnostic détaillé

### A. `tcpdump` non disponible sur NAS

Vérifications :
- `dpkg -l tcpdump` → paquet absent
- `apt-cache policy tcpdump` → candidate visible
- `command -v tcpdump` → vide

Cause :
- VM en Debian **buster** avec dépôts obsolètes/404 + sources en doublon.

### B. Correction APT et installation

Actions :
- backup des sources APT,
- nettoyage des doublons,
- bascule vers `archive.debian.org` (buster/security/updates),
- paramètres APT pour archive (`Check-Valid-Until false`, etc.),
- installation `libpcap0.8` + `tcpdump`.

Validation :
- `tcpdump --version` OK
- capture sur `eth0` OK.

### C. Validation LAN

Depuis Mac :
- `ping -c 3 192.168.0.83` → OK (0% perte)
- `nc -vz 192.168.0.83 22` → OK

Conclusion :
- réseau local + SSH local fonctionnels.

### D. Investigation WAN (4G)

Constats :
- connectivité IPv4 générale OK (`ping 1.1.1.1`, `nc 1.1.1.1:443`, `nc 8.8.8.8:53`)
- route par défaut macOS correcte (`en0` / gateway `172.20.10.1`)
- mais port WAN `22` problématique.

Hypothèse retenue :
- blocage/traitement spécifique du port 22 (amont/opérateur/routage), plutôt qu’un défaut LAN/NAS.

---

## 4) Correctif réseau retenu

### Redirection Freebox

Ajout d’une redirection NAT/PAT :
- **TCP WAN `2222` → LAN `22`** vers `192.168.0.83`.

### Validation finale

Depuis Mac en 4G :
- `nc -4 -vz 82.67.95.8 2222` → **succeeded**
- `ssh -p 2222 root@82.67.95.8` → **connexion réussie**

Conclusion :
- accès SSH WAN opérationnel via port WAN alternatif `2222`.

---

## 5) Sécurité / hygiène appliquée

- Suppression des redirections WAN SMB : `137/139/445` (surface d’attaque réduite).
- Recommandé :
  - conserver `2222 -> 22`,
  - réactiver firewall IPv6 Freebox si coupé durant les tests,
  - SIP ALG : **désactivé** (inutile pour SSH),
  - durcir SSH (clé publique, mot de passe root désactivé, idéalement `PermitRootLogin prohibit-password`).

---

## 6) État final

✅ `tcpdump` installé et fonctionnel  
✅ Connectivité LAN validée  
✅ NAT WAN validé  
✅ SSH distant validé sur `82.67.95.8:2222`  
✅ Exposition WAN inutile (137/139/445) supprimée

---

## 7) Commandes de validation clés (référence)

### LAN
- `ping -c 3 192.168.0.83`
- `nc -vz 192.168.0.83 22`

### WAN
- `nc -4 -vz 82.67.95.8 2222`
- `ssh -p 2222 root@82.67.95.8`

### Capture NAS
- `tcpdump -ni eth0 -vv -l 'tcp port 22 or tcp port 2222'`

---

## 8) RCA (Root Cause Analysis)

- **Cause primaire 1 :** outil de diagnostic absent (`tcpdump`) + dépôts Debian buster non maintenus (404), ce qui a retardé le troubleshooting.
- **Cause primaire 2 :** accès WAN sur port 22 non fiable/non atteignable selon le chemin amont.
- **Résolution :** rétablissement APT + installation `tcpdump`, puis bascule du point d’entrée SSH WAN de `22` vers `2222`.

---

## 9) Décision opérationnelle

Conserver en production :
- `WAN 2222 -> 192.168.0.83:22`
- filtrage minimal et durcissement SSH.

Document prêt pour transmission à Claude.
