---
name: atelier-help
description: "Affiche l'état du projet et les commandes disponibles. Utiliser quand l'utilisateur dit /atelier-help, demande de l'aide sur l'atelier, ou commence une nouvelle session."
figure: Maître d'atelier
---

# Atelier Help

> L'atelier est ouvert. Voyons où on en est et ce qu'il y a à faire.

Oracle de **claude-atelier**. État du projet + commandes disponibles.

## À chaque invocation

### Étape 1 — Diagnostic rapide

Vérifie silencieusement (sans afficher les commandes) :

1. `.claude/CLAUDE.md` existe ? → §0 rempli (Projet courant ≠ "—") ?
2. `.claudeignore` existe à la racine ?
3. `scripts/pre-push-gate.sh` existe et est exécutable ?
4. `settings.json` dans `.claude/` contient `Bash(*)` dans allow ?
5. Dernier handoff dans `docs/handoffs/` : date ?
6. Dernier commit : date et message ?

### Étape 2 — Afficher l'état

```text
╔══════════════════════════════════════════════════╗
║  🔧 claude-atelier                               ║
╠══════════════════════════════════════════════════╣
║  Projet : [nom depuis §0 ou "non configuré"]    ║
║  Stack  : [stack depuis §0]                      ║
║  Santé  : [✅ HEALTHY | ⚠️ X problèmes]          ║
║  Dernier commit : [date] [message court]         ║
║  Dernier review : [date du dernier handoff]      ║
╚══════════════════════════════════════════════════╝
```

### Étape 3 — Afficher les commandes

Lis `atelier-help.csv` (dans le même dossier que ce skill) et affiche :

```text
Commandes disponibles :

  [SU] /atelier-setup     Setup & onboarding interactif
  [RC] /review-copilot    Lancer une review Copilot/GPT
  [AM] /angle-mort        Chercher les angles morts via Copilot
  [AS] /audit-safe        Audit sécurité complet
  [NL] /night-launch      Préparer le night-mode
  [DR] /atelier-doctor    Diagnostic complet (27 checks)
  [TR] /token-routing     Configurer le routing des modèles

Tape un code (ex: SU) ou le nom complet de la commande.
```

### Étape 4 — Recommandation contextuelle

Si §0 n'est pas rempli → recommander `/atelier-setup` en priorité.
Si aucun handoff depuis > 2h et > 100 lignes modifiées → recommander `/review-copilot`.
Si pre-push-gate absent → recommander `/audit-safe`.
Sinon → "Tout est en ordre. Bon dev !"

## Règles

- Ne modifie aucun fichier
- Réponse courte et visuelle (tableau, pas de prose)
- Si l'utilisateur tape un code (SU, RC, AM...), invoquer le skill correspondant
