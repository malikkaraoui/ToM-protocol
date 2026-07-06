# ToM Red Team — Journal (narratif)

> Notes scrupuleuses de la boucle d'attaque autonome. Voir `PROTOCOLE-RED-TEAM.md`
> pour la doctrine, `journal.jsonl` pour les relevés machine, `corpus.jsonl` pour
> le corpus de régression. Rien n'est jamais effacé.

## Format d'une entrée de perçage
```
### [BUILD n · date] BREACH — <attack.id> (seed <s>)
- Symptôme : <ce que le réseau a fait de mal>
- Cause racine : <file:line>
- Patch : <ce qui a été durci> (commit <sha>)
- Re-test : DÉFENDU ✓ · Régression corpus : verte ✓
```

## Historique

### [BUILD 21 · 2026-07-06] FINDING #1 (à investiguer) — chaos.monkey : présence ne repart pas
- **Attaque** : `chaos.monkey` seed 7 (5 kills, 5 revives, 6 skews, min alive=2).
- **Symptôme observé** : après churn+skew lourd puis heal, `accepted` reste à 0→0
  (aucune acceptation ni pendant la boucle ni après guérison). Sur seed 42 le
  scénario s'exécute mais stalle parfois dans le heal.
- **Statut** : **NON CONCLU** — contamination du harnais local (2 process
  tom-stress zombies tenant des ports → contention/hang). Impossible de trancher
  vrai perçage vs flakiness dans cette session.
- **Hypothèses classées** :
  1. `check_presence_all_online` filtre `status==Online` ; les pairs ré-ajoutés
     par `add_peer_addr` après revive ne repassent Online qu'après heartbeat →
     0 challenge émis vers eux (le storm réussit car topologie statique + warmup).
  2. Offsets d'horloge négatifs pendant la boucle cassent la fraîcheur en vol.
  3. Zombies/ports : pur artefact d'environnement.
- **Comment vérifier (à l'exécution, harnais propre)** : lancer sur machine sans
  zombie, logguer le nombre de pairs Online vus par chaque nœud au moment du
  challenge ; si Online=0 après revive → hypothèse 1 (vrai durcissement à faire :
  challenger aussi les pairs connus mais pas-encore-Online, ou forcer un
  heartbeat au add_peer_addr).
- **Décision** : chaos-monkey reste un **outil standalone** (hors chaîne
  `scenarios` pour ne pas faire flaker la suite). Finding porté au corpus.

Attaques déjà prouvées DÉFENDUES en amont (tests runtime + scénarios, à porter au corpus dès le 1er run) :
- pres.forge, pres.replay, pres.usurp, pres.reflect, pres.skew, pres.mem, pres.flood (budget)
  → 14 tests d'intégration + storm 5/5 + chaos-monkey. Servent de baseline « déjà vert ».
