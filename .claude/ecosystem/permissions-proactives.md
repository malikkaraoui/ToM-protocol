# Permissions Proactives — Arrête de me demander

> Tu ne demandes pas la permission à ton marteau pour planter un clou.
> L'atelier fonctionne pareil : les actions courantes sont autorisées
> d'office. Les actions dangereuses restent bloquées. Le reste s'apprend.

## Le problème

Claude Code en mode `default` demande une confirmation pour chaque action
non triviale. Au bout de 3 approbations identiques, c'est du bruit :

```
Claude veut éditer settings.json → [Approuver] ← 1re fois, OK
Claude veut éditer settings.json → [Approuver] ← 2e fois, agaçant
Claude veut éditer settings.json → [Approuver] ← 3e fois, flow cassé
```

L'utilisateur approuve systématiquement → il fait confiance. Pourquoi
continuer à demander ?

## La solution

### 1. Permissions par défaut larges

L'atelier inclut dans `settings.json` des permissions pré-approuvées :

```json
{
  "permissions": {
    "allow": [
      "Read", "Edit", "Write", "Glob", "Grep",
      "Bash(npm run *)", "Bash(npm test *)", "Bash(npm version *)",
      "Bash(git add *)", "Bash(git commit *)", "Bash(git push*)",
      "Bash(git diff *)", "Bash(git status*)", "Bash(git log *)",
      "Bash(gh run *)", "Bash(gh pr *)",
      "Bash(ls *)", "Bash(chmod *)", "Bash(cd *)"
    ]
  }
}
```

### 2. Détection proactive (comportement Claude)

Quand l'utilisateur approuve la même permission 3+ fois dans une session,
Claude doit proposer :

> *« Tu m'as donné 3 fois la permission pour [action].
> Veux-tu que je l'ajoute dans settings.json pour ne plus te déranger ? »*

Si l'utilisateur dit oui → ajouter la règle dans `permissions.allow`.

**Important :** après modification de `settings.json`, il faut **fermer et
rouvrir VS Code** pour que les nouvelles permissions soient chargées.
`Cmd+Shift+P → Claude Code: Restart` peut ne pas suffire pour les permissions.

### 3. Ce qui reste bloqué — toujours

Certaines actions restent interdites, peu importe le niveau de confiance.
C'est la deny list, non négociable :

```json
{
  "permissions": {
    "deny": [
      "Bash(rm -rf:*)",
      "Bash(sudo:*)",
      "Bash(git push --force:*)",
      "Bash(git reset --hard:*)",
      "Bash(git filter-branch:*)",
      "Bash(curl * | sh:*)",
      "Bash(wget * | bash:*)"
    ]
  }
}
```

## Architecture des permissions

```text
Action demandée
    ↓
deny list → BLOQUÉ (exit 2, jamais autorisé)
    ↓
allow list → AUTORISÉ (pas de prompt)
    ↓
ni deny ni allow → PROMPT UTILISATEUR
    ↓
3 approbations → proposer ajout permanent
```

## Le bon équilibre

| Niveau | Actions | Comportement |
| --- | --- | --- |
| **Autorisé** | Read, Edit, Write, git, npm, gh | Aucun prompt |
| **Prompt** | Actions inconnues, nouveaux outils | Demande une fois |
| **Proactif** | Permission donnée 3+ fois | Propose ajout permanent |
| **Bloqué** | rm -rf, sudo, push --force, curl\|sh | Jamais autorisé |

## Pourquoi c'est important

- **Flow** : chaque interruption coûte du contexte mental
- **Confiance** : si tu approuves toujours, tu fais confiance — formalise-le
- **Sécurité** : les deny restent en place, peu importe le reste
- **Apprentissage** : les permissions s'adaptent au projet au fil du temps
