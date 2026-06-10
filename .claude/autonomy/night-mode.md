---
kind: autonomy
name: night-mode
loads_from: src/fr/CLAUDE.md §23
status: canonical
replaces: src/fr/autonomy/_legacy.md + src/fr/orchestration/_legacy.md (dedoublonnage)
---

# Autonomy — Night mode

> **Source canonique unique.** Avant P3, ce pattern était dupliqué entre
> `autonomy/_legacy.md` et `orchestration/_legacy.md`. Ce fichier est
> désormais la seule référence.

## Principe

Le **night mode** permet de laisser Claude travailler en autonomie sur
une tâche longue (build, refactor massif, série de tests, migration)
pendant que l'utilisateur est absent. Il repose sur :

1. Des **specs claires** écrites avant de lancer
2. Un **mode de permission** `acceptEdits` avec allow/deny listes strictes
3. Un **disjoncteur budget** (`maxBudgetUsd`)
4. Une **review manuelle le matin** avant tout push

## Pré-requis non négociables

- `.claudeignore` configuré (cf. `../security/secrets-rules.md` en P3.d)
- `.gitignore` à jour et committé
- `settings.json` chargé avec `defaultMode: acceptEdits` + allow/deny
  (cf. `../../templates/settings.json`)
- `maxBudgetUsd` **toujours défini**
- `git push` **toujours** en `deny`
- `sudo` et `rm -rf` **toujours** en `deny`

## Procédure

### 1. Préparer les specs avant de fermer

```bash
# Ecrire les specs dans un fichier partage avec Claude
vim docs/specs.md
# ou /docs/night-<date>.md pour garder un historique
```

Format minimal de specs :

```markdown
# Objectif
<une phrase>

# Contexte
<fichiers concernes, contraintes, tests a passer>

# Critere de reussite
<conditions pour considerer la tache faite>

# Hors scope
<ce qu'il ne faut PAS toucher>
```

### 2. Lancer Claude avec le bon mode

```bash
claude --permission-mode acceptEdits \
  "Implementer selon /docs/specs.md. \
   Ecrire les tests. \
   Committer chaque etape de facon atomique. \
   Ne pas pusher. \
   Mettre a jour §0 de CLAUDE.md si necessaire."
```

### 3. Review le matin

```bash
# 1. Review des commits
git log --oneline

# 2. Diff cumule depuis le dernier push
git diff origin/main..HEAD

# 3. Gate pre-push complete
bash scripts/pre-push-gate.sh

# 4. Push si tout est vert
git push
```

## Pièges classiques

- **Specs trop vagues** : Claude divergera vers une interprétation qu'on
  ne voulait pas. Toujours inclure un « Hors scope ».
- **Pas de `maxBudgetUsd`** : boucle infinie possible → portefeuille vidé.
  Ne jamais oublier.
- **`git push` autorisé** : du code non relu arrive en prod pendant la
  nuit. Jamais.
- **Tests manquants** : sans tests, aucune garantie de non-régression.
  Inclure « écrire les tests » dans le prompt de lancement.
- **Review expéditive le matin** : lire vraiment le diff, pas juste
  checker que la gate passe.

## Cas d'usage typiques

- Implémentation d'un plan BMAD déjà validé
- Refactor mécanique (rename, restructuration de fichiers)
- Migration de dépendances avec codemod
- Génération + itération sur une suite de tests
- Bug difficile laissé en exploration autonome avec hypothèses

## Supervision — Tâche planifiée Cowork

Le watchdog utilise les **tâches planifiées Cowork** (app Claude desktop),
pas un script bash custom. La tâche planifiée tourne en parallèle de
Claude Code dans VSCode, sur la même machine.

### Comment ça marche

| Composant | Rôle |
| --- | --- |
| Claude Code (VSCode) | Travaille en `acceptEdits` sur la tâche |
| Tâche planifiée Cowork | Vérifie périodiquement que le repo bouge |
| Dispatch (mobile) | Reçoit les notifications push si crash détecté |

### Créer le watchdog

**App Claude desktop → Programmé → + Nouvelle tâche :**

- **Nom** : `night-watchdog`
- **Description** : Surveille l'activité Claude Code sur le projet courant
- **Prompt** (v4, testé et validé 2026-04-12 — diagnostic visuel + auto-clic) :

```text
Tu es un watchdog pour une session Claude Code de nuit.

Etape 0 — Verifie si Claude Code est actif :
Lance `pgrep -f "claude" > /dev/null 2>&1` pour chercher un processus claude.
Si aucun processus claude n'est trouve : termine silencieusement.
Personne ne travaille, pas besoin de surveiller.

Etape 1 — Verifie l'activite git :
Lance `git -C "<CHEMIN_DU_REPO>" log -1 --format='%ci %s'` pour la
date et le message du dernier commit.
Lance `date '+%Y-%m-%d %H:%M:%S'` pour l'heure actuelle.
Calcule la difference en minutes.

Si le dernier commit date de moins de 15 minutes : termine silencieusement.

Etape 2 — Diagnostic visuel (si inactivite > 15 min) :
Prends un screenshot de l'ecran.
Analyse ce que tu vois :
- Est-ce que VSCode est au premier plan ?
- Est-ce qu'il y a une boite de dialogue de permission Claude Code
  (bouton "Allow", "Yes" ou "Always allow" visible) ?
- Est-ce que Claude Code semble en cours de traitement (spinner) ?
- Est-ce que la session semble figee (aucun signe d'activite) ?

Etape 3 — Action selon le diagnostic :

CAS A — Demande de permission visible :
Clique sur le bouton pour donner l'acces.
Envoie un iMessage a <TON_NUMERO> : "Watchdog: Claude Code etait bloque
par une demande de permission sur <NOM_PROJET>. J'ai clique Allow.
La session devrait reprendre."

CAS B — Claude Code semble actif (spinner, traitement en cours) :
Ne fais rien. Termine silencieusement.

CAS C — Session figee (pas de spinner, pas de dialogue) :
Envoie un iMessage a <TON_NUMERO> : "ALERTE: Claude Code semble
completement fige sur <NOM_PROJET>. Aucun commit depuis [N] minutes.
La session est probablement crashee. Tu dois intervenir manuellement."

CAS D — VSCode non visible / app fermee :
Envoie un iMessage a <TON_NUMERO> : "ALERTE: VSCode ne semble pas
ouvert. Claude Code ne peut pas tourner. Verifie ta machine."

CAS E — Panneau "session limit" / "quota" / "resets at" / "You've hit your limit" visible :
C'est normal. NE PAS alerter l'utilisateur. NE PAS envoyer d'iMessage.
Note l'heure de reset si visible sur l'ecran.
Termine silencieusement.
A la PROCHAINE execution planifiee (1h plus tard) :
  1. Prends un screenshot
  2. Si le panneau quota est toujours la : termine silencieusement, reessaie a la prochaine execution
  3. Si Claude Code est disponible (champ de saisie visible, pas de panneau quota) :
     a. Identifie le champ de saisie de Claude Code sur le screenshot
     b. Clique dessus (outil clic, coordonnees du champ)
     c. Utilise l'outil type pour taper : REPRISE suite a la limite de quota
     d. Utilise l'outil key pour appuyer sur Return
     e. Termine silencieusement — Claude Code reprend tout seul

Ne modifie aucun fichier du projet. Ne committe rien.
```

> Remplacer `<CHEMIN_DU_REPO>`, `<TON_NUMERO>`, `<NOM_PROJET>`.
> Le prompt se base sur git + screenshot (computer use). Dispatch notifie
> automatiquement via push quand la tâche répond (pas besoin de
> l'instruire explicitement).

- **Fréquence** : Horaire
- **Dossier de travail** : n'importe lequel (le chemin git est en dur)
- **Modèle** : Claude Haiku 4.5
- **Connecteurs** : Read and Send iMessages
- **Computer use** : activé (Settings → General → Computer use → ON)

### Capacités du watchdog v4 (testées)

| Situation | Action du watchdog | Résultat |
| --- | --- | --- |
| Activité récente (< 15 min) | Termine silencieusement | Zéro coût |
| Bouton permission visible | Screenshot → clic auto → iMessage | Session débloquée |
| Spinner actif | Termine silencieusement | Faux positif évité |
| Session crashée | iMessage alerte | Intervention humaine requise |
| VSCode fermé | iMessage alerte | Intervention humaine requise |
| Quota limit atteint | Silence → attend 1h → clic + type + Return | Session relancée sans alerte |

### 3 méthodes de scheduling comparées

| Méthode | Tourne sur | Machine allumée ? | Accès fichiers locaux | Intervalle min |
| --- | --- | --- | --- | --- |
| **Desktop (Cowork)** | Ta machine | Oui + app ouverte | Oui | 1 min |
| **Cloud** (claude.ai) | Cloud Anthropic | **Non** | Non (clone GitHub) | 1 heure |
| **`/loop`** | Ta machine | Oui + session ouverte | Oui | 1 min |

**Desktop = le bon choix pour le night-mode** : ta machine est allumée
(sinon Claude Code ne tournerait pas), l'app Claude est ouverte, et la
tâche a accès aux fichiers locaux.

Cloud = utile si tu veux vérifier le repo GitHub même machine éteinte
(ex : un CI overnight). Intervalle minimum 1 heure.

### Ce que Dispatch apporte

Dispatch = un fil de conversation persistant entre ton iPhone et le
desktop. Quand la tâche planifiée détecte un problème :

1. **Notification push sur iPhone** via l'app Claude
2. Tu peux **répondre depuis ton tel** (ex : « relance la session »)
3. Dispatch route ta demande vers Cowork sur le desktop
4. Claude agit (relance, diagnostic, rapport)

### Procédure night-mode complète

```bash
# 1. Soir — préparer les specs
vim docs/specs.md

# 2. Créer le watchdog dans l'app Claude desktop
#    (Programmé → + Nouvelle tâche → config ci-dessus)

# 3. Lancer Claude Code
claude --permission-mode acceptEdits \
  "Implementer selon docs/specs.md. Committer chaque etape. Ne pas pusher."

# 4. Aller dormir — le watchdog surveille

# 5. Matin — review
git log --oneline
bash scripts/pre-push-gate.sh
git push
```

### Note

Les scripts bash watchdog (`night-watchdog.sh`, `night-launch.sh`) ont été
retirés du repo. La tâche planifiée Cowork les remplace entièrement :
processus indépendant, notification iMessage/Dispatch, pas de script à
maintenir.

## Protocole REPRISE (relance après quota)

Quand Claude Code reçoit le message `REPRISE suite à la limite de quota` :

1. **Ne pas demander d'explication** — la cause est connue (token limit)
2. **Lire le TodoWrite** pour identifier les tâches pending `[ ]` ou en cours `[→]`
3. **Lire `git log -5`** pour voir le dernier commit et comprendre le contexte
4. **Reprendre** la tâche en cours exactement là où elle était
5. **Signaler** : `[REPRISE] Relancé après quota. Reprise depuis : <dernière tâche>`

Si aucun TodoWrite ni tâche identifiable : répondre `[REPRISE] Aucune tâche en cours identifiée. Qu'est-ce que je reprends ?`

## Cas où ce mode est une mauvaise idée

- Décision architecturale ouverte (pas assez de cadrage)
- Tâche avec inconnues métier (Claude va improviser)
- Refactor sur du code flou non documenté
- Première session sur un projet inconnu (pas de `§0` rempli)
