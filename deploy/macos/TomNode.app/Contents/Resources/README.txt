ToM Node - Full Node avec UI
==============================

INSTALLATION
------------
1. Copie TomNode.app dans /Applications (ou n'importe où)
2. Double-clic pour lancer

LANCEMENT
---------
Double-clic sur TomNode.app
→ Ouvre un Terminal avec l'interface TUI
→ Username auto-détecté depuis le hostname

COMMANDES TUI
-------------
/help                  - Aide complète
/peers                 - Liste des peers connectés
/msg <peer_id> <texte> - Envoyer un message direct
/group create <name>   - Créer un groupe
/group join <id>       - Rejoindre un groupe
/group list            - Liste des groupes
/group msg <id> <text> - Envoyer dans un groupe
/quit                  - Quitter

MONITORING & STATUS
-------------------
# Dans le TUI, tu vois en direct:
- Messages entrants/sortants
- Peers découverts
- Statut des groupes
- Événements réseau

# Ou depuis un autre terminal:
ps aux | grep tom-tui           # Vérifie que le node tourne
lsof -i -P | grep tom-tui       # Ports utilisés

LOGS
----
Les logs apparaissent directement dans le Terminal du TUI.

ARCHITECTURE
------------
TomNode.app = Noeud complet ToM Protocol:
✅ Client (envoi/réception messages)
✅ Relay (forwarding pour autres nodes)
✅ Discovery (gossip, heartbeat)
✅ Groupes (création, encryption E2E)
✅ NAT Traversal (hole punching)

ARRÊT
-----
Dans le TUI: taper /quit
Ou: Cmd+W pour fermer le Terminal
Ou: pkill -f tom-tui

TIPS
----
- Premier lancement: patiente ~5s pour la découverte
- Les peers se découvrent automatiquement via gossip
- Direct upgrade: après relay, passe en direct QUIC
- 4G/CGNAT: fonctionne via hole punching
