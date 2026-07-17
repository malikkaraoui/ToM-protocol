# Historique tokens / clôtures loop-master

> Tenu par le Chef Projet à chaque clôture de pipeline.

| Date | Cible | Sprints | Verdict Relecteur | Issue |
|------|-------|---------|-------------------|-------|
| 2026-07-17 | Fix réception flotte (régression Codable Swift, build 96) | 1 (fix déjà implémenté+validé en réel avant pipeline ; Relecteur subagent ~65k tokens) | RATIFIÉ | Livré : init(from:) explicite + test contrat + os.log public + harnais FFI ; flotte Mac/iPad/AppleTV validée, iPhone à redéployer |
| 2026-07-17 | Cycle de vie background iOS (#48, build 98) | 1 (implémentation + 2 tests devicectl réels + oracle 4 agents ~205k tokens) | 3× RATIFIÉ + 1 MAJEUR arbitré (2 vrais findings corrigés, 2 rejetés sur pièces, 1 faux « double-free » réfuté la veille au même endroit) | Livré : grâce 18 s + arrêt propre + hold teardown + BGAppRefresh + MetricKit ; validé terrain (0 échec flotte pendant absence) ; nouveau bug terrain caractérisé → chantier « start borné » (prompt de reprise livré) |
