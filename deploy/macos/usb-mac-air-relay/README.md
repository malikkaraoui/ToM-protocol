# USB install — MacBook Air relay

Ce dossier permet d'installer `tom-relay` sur MacBook Air via clé USB (sans SSH).

## 1) Préparer le bundle sur MacBook Pro

Depuis la racine du repo:

- `AIR_USER` : optionnel (si connu à l'avance)
- `tom-stress` est inclus par défaut
- optionnel: `AIR_COPY_STRESS=0` pour ne copier que `tom-relay`

Exemple recommandé (inclut `tom-stress`) :

./scripts/prepare-usb-mac-air-relay.sh

Exemple si tu connais déjà le user:

AIR_USER=malik ./scripts/prepare-usb-mac-air-relay.sh

Sans `tom-stress` (optionnel) :

AIR_COPY_STRESS=0 ./scripts/prepare-usb-mac-air-relay.sh

Le bundle est généré dans:

- `target/usb-mac-air-relay-bundle/`

Optionnel : tu peux choisir un autre dossier de sortie, par exemple directement la clé USB :

`USB_OUT_DIR=/Volumes/<NOM_CLE>/tom-air-relay ./scripts/prepare-usb-mac-air-relay.sh`

## 2) Copier sur la clé USB

Copier le dossier `deploy/macos/usb-mac-air-relay/` sur la clé, puis brancher la clé au MacBook Air.

## 3) Installer sur MacBook Air

Sur le MacBook Air:

1. Ouvrir Terminal
2. Aller dans le dossier copié depuis la clé USB
3. Lancer:

./install-on-air.sh

## 4) Vérifier que le relay tourne

curl http://127.0.0.1:3341/health
curl http://127.0.0.1:9091/metrics | head

## 5) Commandes utiles (MacBook Air)

Arrêter:

launchctl unload ~/Library/LaunchAgents/com.tom.relay.air.plist

Relancer:

launchctl load ~/Library/LaunchAgents/com.tom.relay.air.plist

Logs:

- /tmp/tom-relay-air.log
- /tmp/tom-relay-air-error.log
