# Challenger — Le coéquipier qui te dit la vérité

> Un dev seul ne voit pas ses propres angles morts. Plus il code longtemps,
> plus il perd du recul. Le système Challenger est le garde-fou automatique
> qui dit « stop, fais entrer quelqu'un d'autre ».

## Le problème

Quand tu codes depuis 3 heures :
- Tu ne vois plus les erreurs évidentes
- Tu répètes la même approche sans t'en rendre compte
- Tu fais de l'architecture sans la valider
- Tu ajoutes des features sans mettre à jour la vitrine
- Tu tournes en rond sur un bug depuis 30 min

Aucun dev ne va spontanément s'arrêter et dire *« j'ai besoin d'un deuxième regard »*.
C'est pour ça que le Challenger le fait automatiquement.

## Les 5 triggers

Le système Challenger détecte 5 situations où un coéquipier devrait
intervenir. Chaque trigger est implémenté dans `guard-review-auto.sh`
(PostToolUse sur `*git commit*`).

### 1. Volume — 100+ lignes sans review

```
🔍 [CHALLENGER] 150 lignes modifiées depuis la dernière review.
   Tu as la tête dans le guidon — un coéquipier verrait ce que tu ne vois plus.
   → /review-copilot ou /angle-mort
```

**Pourquoi :** Au-delà de 100 lignes, la probabilité d'angle mort augmente
exponentiellement. Un humain ne reviewerait jamais 200 lignes d'un coup.

### 2. Feature/Refactor terminé — Challenge obligatoire

```
🎯 [CHALLENGER] Feature détectée : "feat: agent Isaac — satellite npm publish"
   C'est le moment de challenger AVANT de continuer.
   → /angle-mort ou /review-copilot
   📋 README : vérifier que cette feature est dans le README
```

**Pourquoi :** Une feature terminée sans challenge, c'est un produit livré
sans QA. Le commit `feat:` ou `refactor:` est le signal naturel.

### 3. Endurance — 10 commits sans regard extérieur

```
⏰ [CHALLENGER] 10 commits sans challenge externe.
   Trop longtemps seul — les angles morts s'accumulent.
   → /angle-mort ou /review-copilot
```

**Pourquoi :** Même sans feature majeure, l'accumulation de petits commits
crée de la dette invisible. 10 commits = signal d'alerte.

### 4. Architecture — Fichier structurant créé

```
🏗️  [CHALLENGER] Fichier architecturaux créés : hooks/guard-new.sh src/stacks/new.md
   Un choix d'architecture mérite un deuxième regard.
   → /review-copilot pour validation
```

**Détection automatique** : tout nouveau fichier dans `src/stacks/`, `hooks/`,
`src/skills/*/SKILL.md`, `.github/workflows/`, `src/fr/ecosystem/`,
`src/fr/orchestration/`.

**Pourquoi :** Un fichier d'architecture mal conçu contamine tout le projet.
C'est le moment le plus rentable pour une review.

### 5. Bug bloquant — 3+ tentatives échouées

```
🚨 [CHALLENGER] 3 tentatives échouées sur la même commande.
   STOP. Tu tournes en rond.
   → /review-copilot avec le contexte d'erreur
   → Changer d'approche complètement
```

**Pourquoi :** Si tu as essayé 3 fois la même chose, tu n'as pas compris
le problème. Un autre LLM ou un humain verra ce que tu ne vois pas.

## Le workflow Challenger

```text
Code normal
    ↓
Trigger détecté (commit, échec, volume...)
    ↓
[CHALLENGER] message dans le contexte
    ↓
Option A : /angle-mort (auto-review interne, rapide)
    ↓
Option B : /review-copilot → handoff .md → Copilot/GPT répond
    ↓
/integrate-review → trier les retours → actions
    ↓
Retour au code, angles morts supprimés
```

## Quand utiliser quoi

| Situation | Action | Pourquoi |
| --- | --- | --- |
| Feature terminée | `/angle-mort` | Auto-review rapide avant de continuer |
| 100+ lignes modifiées | `/review-copilot` | Trop de code pour s'auto-review |
| Bug qui résiste | `/review-copilot` + contexte erreur | Un autre œil voit le problème |
| Choix d'archi | `/review-copilot` | Valider avant d'investir plus |
| Spec/plan créé | `/review-copilot` | Challenger les hypothèses |
| 10 commits d'affilée | `/angle-mort` | Pause de recul minimum |

## Ce que le Challenger n'est PAS

- **Pas un bloqueur** — il propose, il ne bloque pas (pas d'`exit 2`)
- **Pas un juge** — il ne dit pas si le code est bon ou mauvais
- **Pas un substitut** — il rappelle de demander de l'aide, il ne l'apporte pas

Le Challenger est un **superviseur bienveillant** qui dit :
*« Tu bosses bien, mais là tu as besoin d'un regard extérieur. Fais-le. »*

## Hooks associés

| Hook | Fichier | Trigger |
| --- | --- | --- |
| Challenger complet | `guard-review-auto.sh` | PostToolUse `*git commit*` |
| Anti-boucle | `guard-anti-loop.sh` | PostToolUse (toute commande bash) |
| Rappel README | Intégré dans `guard-review-auto.sh` | Commit `feat:` / `refactor:` |
| Détection archi | Intégré dans `guard-review-auto.sh` | Nouveau fichier structurant |
