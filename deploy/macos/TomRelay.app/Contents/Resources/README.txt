ToM Relay - macOS App Bundle
=============================

INSTALLATION
------------
1. Copie TomRelay.app dans /Applications (ou n'importe où)
2. Double-clic pour lancer

LANCEMENT
---------
Double-clic sur TomRelay.app
Le relay démarre sur:
- Port 3340 (relay)
- Port 9090 (metrics)

VÉRIFICATION
------------
# Vérifie que le relay répond
curl http://127.0.0.1:9090/metrics | head

# Vérifie le process
ps aux | grep tom-relay

LOGS
----
~/Library/Logs/TomRelay/tom-relay.log
~/Library/Logs/TomRelay/tom-relay-error.log

ARRÊT
-----
pkill -f "tom-relay --dev"

ou dans Activity Monitor, cherche "tom-relay" et Force Quit

SUPPRESSION
-----------
1. Arrête le relay (voir ci-dessus)
2. Supprime TomRelay.app
3. Optionnel: rm -rf ~/Library/Logs/TomRelay
