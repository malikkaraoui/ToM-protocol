# Sécurité & modèle de menace

Cette page explicite ce que ToM protège aujourd’hui (et ce qu’il ne protège pas encore), en se basant sur les mécanismes présents dans le code.

## Objectif de sécurité

ToM vise à permettre un transport P2P où :

- le **contenu** n’est lisible que par l’émetteur et le destinataire (E2E)
- les relais ne sont que des **transmetteurs** (pas des serveurs de stockage)
- le protocole reste utilisable dans un monde où les relais et le bootstrap sont **non fiables**

## Modèle de menace (pratique)

Acteurs potentiellement adverses :

- un relai malveillant (observe / drop / retarde / rejoue)
- un pair malveillant (spam, messages forgés)
- un bootstrap (signaling) compromis (metadata, injection, clé publique de chiffrement falsifiée)

## Ce que l’E2E protège

Quand le payload est chiffré (`tweetnacl.box`) :

- **confidentialité** : un relai ne peut pas lire le contenu
- **intégrité/authenticité du ciphertext** : la modification du ciphertext fait échouer le déchiffrement

Le payload E2E est transporté comme `EncryptedPayload` :

- `ciphertext` (hex)
- `nonce` (hex)
- `ephemeralPublicKey` (hex)

## Ce qui fuit (incompressible aujourd’hui)

Même en E2E, les métadonnées de l’enveloppe restent visibles des relais :

- `from`, `to`
- `via` (chemin)
- `type`
- `timestamp` (+ éventuellement `hopTimestamps`)

C’est un choix explicite : ToM n’est pas (encore) un système d’anonymat.

## Limites actuelles (à afficher franchement)

### Signaling = bootstrap non fiable

Aujourd’hui, le signaling sert à :

- annoncer sa présence
- récupérer la liste des participants
- échanger des informations (dont la clé publique de chiffrement)

Tant qu’un mécanisme d’authentification/attestation n’est pas en place, un signaling compromis peut :

- mentir sur les participants
- injecter de fausses clés de chiffrement (risque de MITM sur l’E2E)

### Relais best-effort

Un relai peut drop/retarder : ToM vise la résilience par sélection automatique + multi-relai + rerouting, mais pas une garantie “toujours livrable”.

## Protections déjà présentes (concrètes)

- **Anti-replay ACK/read-receipt** : cache court + clé composite (messageId, sender, type)
- **Dedup messages** : cache d’ID composite (`messageId:from`) pour éviter la redélivrance
- **Borne de profondeur** sur les chaînes de relais (`via`) via une profondeur maximale
- **Clamping de `readAt`** (pas de timestamp futur / trop ancien)

## Recommandations d’intégration (app)

- Traiter `onStatus` comme une source de debug (et non comme une API stable de produit)
- Ne pas afficher d’assertions “livré” tant que l’ACK destinataire n’est pas reçu
- Appeler `markAsRead` uniquement quand l’UI a réellement présenté le message

## Roadmap sécurité (sections à compléter au fur et à mesure)

- authentification du bootstrap / multi-seeds
- mécanisme d’échange de clés robuste (attestation / signatures / pinning)
- anti-sybil à grande échelle
- suppression du bootstrap au profit d’une découverte autonome (DHT)

## Sources

- E2E : https://github.com/malikkaraoui/ToM-protocol/blob/main/packages/core/src/crypto/encryption.ts
- Routeur (anti-replay, dedup, read receipt clamp) : https://github.com/malikkaraoui/ToM-protocol/blob/main/packages/core/src/routing/router.ts
- Enveloppe : https://github.com/malikkaraoui/ToM-protocol/blob/main/packages/core/src/types/envelope.ts
