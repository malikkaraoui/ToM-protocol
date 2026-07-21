# Capstone Phase C — check-list d'exécution (préparée le 2026-07-21)

> Design source : `banc-roles-sous-charge.md` §3 (séquence 15 min, oracle global
> 3 verdicts). Prérequis DONE : P1 ✓, herméticité étage L ✓ (`237b9fe`),
> R4 isolé ✓ multi-host (`2cbf964`, baseline froide ~25 s).

## Reste à coder AVANT le run (petit)

1. **`tom-stress r4 --production-rendezvous`** (mutuellement exclusif avec
   `--namespace`) : l'inconnu du capstone rejoint le rendez-vous RÉEL de la
   flotte (design §3 : le namespace test ne sert qu'à l'étage isolé).
   Garde-fous : username `TEST_NODE_PREFIX`, TTL du run borné, et discipline
   anti-pollution — retrait du nœud + `/reset?level=network` si besoin en fin
   de capstone (jamais de fantôme laissé au carnet réel).
2. **Charge de fond** : réutiliser `tom-stress ping/burst` multi-host vers la
   flotte (le §3 du banc courbe sert de bruit, pas de juge) — vérifier les
   cibles pré-enregistrées AVANT le run (critères écrits d'abord, design §4).
3. Orchestrateur : `scripts/chaos/orchestrator.py` couvre déjà kill-hub
   (scenario_7_kill), relevés collecteur, seq_counts. À étendre d'une séquence
   `capstone` chronométrée (t+0 → t+15) qui appelle les briques existantes.

## Séquence du run (design §3, inchangée)

| t | Événement | Qui |
|---|---|---|
| t+0 | charge de fond démarre | orchestrateur |
| t+2 | kill du hub de groupe | orchestrateur (TEST-*) |
| t+4 | **extinction d'un device** + messages vers lui | **Malik** |
| t+6 | l'inconnu rejoint (r4 --production-rendezvous) | orchestrateur |
| t+8 | un headless spamme | orchestrateur |
| t+10 | **rallumage du device** | **Malik** |
| t+15 | fin, drain, relevés | orchestrateur |

## Conditions du jour J

- Flotte 5 nœuds UP en build 139 (Mac, iPad, iPhone Malik, iPhone Laura, NAS).
- ⚠️ NAS : IP dynamique — vérifier l'IP du jour (le 21/07 : `.83`, plus `.21` ;
  le relais public 82.67.95.8:3340 reste le témoin de vie). Binaire de banc
  déjà déployé : `/root/tom-stress` (md5 `ea18a1c14899`).
- Présence Malik ~20 min (deux actions : extinction t+4, rallumage t+10).
- Kill de tout process de stress résiduel avant ET après (runbook).

## Verdicts pré-enregistrés (design §3, à chiffrer avant le run)

1. Livraison de fond jamais sous la cible pré-enregistrée (à fixer depuis la
   baseline mesurée juste avant t+0).
2. Chaque mécanisme déclenché avec preuve au collecteur (élection hub,
   réplication backup, source d'amorçage de l'inconnu, throttle, livraison
   différée).
3. PoP vrai de bout en bout (`online_count` = réalité, pendant le chaos).
