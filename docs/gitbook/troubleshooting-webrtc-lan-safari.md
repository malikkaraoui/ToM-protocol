# Troubleshooting (LAN, WebRTC/WS, Safari)

Cette page est une checklist de diagnostic orientée démo + stack actuelle.

## Symptômes fréquents

- Deux appareils sur le même Wi‑Fi ne se voient pas (peer discovery OK mais pas de messages).
- Safari (iOS/macOS) ne se connecte pas, ou coupe la connexion au bout de quelques secondes.
- “Ça marche en localhost mais pas sur le téléphone”.
- En HTTPS, le signaling WebSocket ne se connecte pas.

## 0) Comprendre ce qui tourne réellement

Dans la démo, le signaling est construit ainsi :

- `ws://${window.location.hostname}:3001`

Donc :

- si vous servez la page en `http://` → `ws://` est cohérent
- si vous servez la page en `https://` → `ws://` devient **mixed content** (bloqué par les navigateurs, Safari inclus)

Source : https://github.com/malikkaraoui/ToM-protocol/blob/main/apps/demo/src/main.ts

## 1) Debug réseau LAN (le classique)

### Vérifier l’IP et le host

- Le serveur Vite doit écouter sur l’interface LAN (pas seulement `localhost`).
- Assurez-vous d’ouvrir la démo via une URL du type `http://<ip-lan>:<port>` depuis le téléphone.

### Pare-feu macOS

- Autoriser Node/Vite et le serveur de signaling (`3001`) dans le pare-feu.
- Test : le téléphone doit pouvoir joindre `http://<ip-lan>:3001/health` (si endpoint présent) ou au minimum ouvrir le TCP 3001.

### Wi‑Fi invités / isolation AP

Beaucoup de réseaux activent l’isolation client : les appareils ne peuvent pas se joindre entre eux.

- Symptôme : la démo charge mais aucun peer n’est joignable.
- Fix : désactiver l’isolation ou utiliser un hotspot.

## 2) WebSocket vs WSS (secure context)

### Cas `https://`

Si la page est en HTTPS, utilisez **WSS** côté signaling, sinon :

- Chrome/Firefox bloquent souvent
- Safari est particulièrement strict

Conséquences :

- page : `https://demo...`
- signaling : doit être `wss://demo...:3001` (ou derrière un reverse proxy 443 → 3001)

### Reverse proxy recommandé

En prod, mettez un reverse proxy (Caddy/Nginx) pour :

- TLS (certificats)
- même origin
- websockets upgrade

{% hint style="info" %}
Le plus simple : servir la démo et le signaling sur le même domaine/port (ou au moins même scheme https/wss).
{% endhint %}

## 3) Safari (iOS/macOS) : points d’attention

- Mixed content (HTTPS + WS) : bloqué.
- Réseau cellulaire : latence + NAT agressif.
- Mise en veille / verrouillage écran : Safari peut suspendre des sockets.

### Checklist Safari

1. Ouvrir la console (macOS Safari Web Inspector / iOS via Mac)
2. Vérifier erreurs de connexion WebSocket (`failed`, `blocked`, `security error`)
3. Vérifier que l’URL de signaling pointe vers un hostname résolvable depuis l’appareil

## 4) WebRTC : distinguer l’objectif et l’implémentation actuelle

Le projet mentionne WebRTC dans les objectifs (transport P2P), mais le code actuel côté transport est principalement orienté :

- chemin direct vs relai (conceptuellement)
- signaling WS pour la démo

Si vous ajoutez/activez un transport WebRTC DataChannel :

- prévoyez STUN/TURN
- testez Safari en premier (support WebRTC OK mais contraintes réseau fortes)

{% hint style="warning" %}
Sans TURN, beaucoup de réseaux “entreprise” / 4G / CGNAT vont échouer en P2P direct.
{% endhint %}

## 5) Symptôme → cause → fix rapide

- **La démo charge, mais le signaling ne connecte pas**
  - Cause : port 3001 bloqué / mauvais hostname / mixed content
  - Fix : ouvrir `http://<ip>:3001`, utiliser `wss://` en HTTPS, reverse proxy

- **Les peers se découvrent mais aucun message n’arrive**
  - Cause : relai offline / routage impossible / NAT / isolation AP
  - Fix : tester sur hotspot, vérifier relai, activer logs

- **Safari marche puis “freeze”**
  - Cause : suspension background
  - Fix : garder l’écran actif, tester sur macOS Safari avec inspector

## 6) Logs utiles

- Activer logs “transport / router / groups” dans la démo (si flags disponibles).
- Noter :
  - timestamp
  - `messageId`
  - relai sélectionné
  - status transitions (`pending → …`)

## Sources

- Démo: construction de `SIGNALING_URL` : https://github.com/malikkaraoui/ToM-protocol/blob/main/apps/demo/src/main.ts
- Concept direct path manager : https://github.com/malikkaraoui/ToM-protocol/blob/main/packages/core/src/transport/direct-path-manager.ts
