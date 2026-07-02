# SKILL — /review-oracle

> Contre-pouvoir indépendant. Tu n'es pas juge et partie.
> Pose `/tmp/claude-atelier-review-done` → déverrouille le prochain `git push`.

## Déclenchement

- Manuel : `/review-oracle`
- Automatique : `guard-review-auto.sh` bloque `git push` si diff ≥ 50 lignes sans flag

## Protocole

### 1. Préparer le diff

```bash
git diff "@{u}" HEAD --stat 2>/dev/null || git diff HEAD~1 HEAD --stat
git diff "@{u}" HEAD 2>/dev/null || git diff HEAD~1 HEAD
```

### 2. Lancer 4 agents EN PARALLÈLE

**AGENT-DOCTRINE** — Gardien des décisions
- Lit `vault/20-decisions.md` + `.claude/CLAUDE.md`
- Question : le diff viole-t-il §5 (anti-hallucination), §22 (secrets), §13 (commits FR), une décision verrouillée ?

**AGENT-CODE** — Bugs et régressions
- Analyse le diff ligne par ligne
- Cherche : imports cassés, logique inversée, edge cases manqués, effets de bord silencieux

**AGENT-SÉCURITÉ** — Surface d'attaque
- Cherche : secrets en dur, injection de commande, exposition API, gate pré-push court-circuité

**AGENT-TESTS** — Couverture réelle
- Lance `npm run lint && npm test` — rapporte la sortie exacte (pas "ça marche")
- Chaque modification logique sans test = finding MAJEUR

### 3. Arbitrer

Tu es l'arbitre final. Finding sans ancrage dans le diff réel (fichier:ligne) → ignoré (§5).

### 4. Verdict consolidé

| Niveau | Signification | Flag |
|--------|--------------|------|
| RATIFIÉ | Aucun bloquant | ✅ poser le flag |
| MAJEUR | Problème sérieux, non bloquant | ✅ poser le flag + lister fixes |
| BLOQUANT | Défaut critique | ❌ ne pas poser le flag, exposer le problème |

### 5. Poser le flag (si RATIFIÉ ou MAJEUR)

```bash
touch /tmp/claude-atelier-review-done
```

## Règles non négociables

- Un finding = un fichier, une ligne, un fix proposé. Jamais de constat sec.
- BLOQUANT → ne PAS poser le flag. Corriger, puis `/review-oracle` de nouveau.
- Tests vérifiés par `npm test` réel, pas par inspection visuelle.
