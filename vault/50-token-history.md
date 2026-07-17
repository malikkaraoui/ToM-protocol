# Historique tokens / clôtures loop-master

> Tenu par le Chef Projet à chaque clôture de pipeline.

| Date | Cible | Sprints | Verdict Relecteur | Issue |
|------|-------|---------|-------------------|-------|
| 2026-07-17 | Start borné + anti-zombie (build 99) | 1 (TDD : repro → timeout FFI rc -2 → cause racine DHT SharedDht → backoff Swift ; Relecteur subagent ~80k tokens) | ACCEPTABLE — 1 « BLOQUANT » RÉFUTÉ sur pièces (tokio::spawn ne retourne pas de Result, 3ᵉ faux bloquant de la semaine), 1 mineur retenu (log ronde sautée debug→info), 7 PASS | Livré : start total 96 ms (vs gel indéfini), zéro survie au chemin d'erreur (abort+reaper), tests FFI réparés (pourrissaient hors CI — étape test ajoutée à check-ffi.sh) ; reste critère terrain 3G |
| 2026-07-17 | Fix réception flotte (régression Codable Swift, build 96) | 1 (fix déjà implémenté+validé en réel avant pipeline ; Relecteur subagent ~65k tokens) | RATIFIÉ | Livré : init(from:) explicite + test contrat + os.log public + harnais FFI ; flotte Mac/iPad/AppleTV validée, iPhone à redéployer |
| 2026-07-17 | Cycle de vie background iOS (#48, build 98) | 1 (implémentation + 2 tests devicectl réels + oracle 4 agents ~205k tokens) | 3× RATIFIÉ + 1 MAJEUR arbitré (2 vrais findings corrigés, 2 rejetés sur pièces, 1 faux « double-free » réfuté la veille au même endroit) | Livré : grâce 18 s + arrêt propre + hold teardown + BGAppRefresh + MetricKit ; validé terrain (0 échec flotte pendant absence) ; nouveau bug terrain caractérisé → chantier « start borné » (prompt de reprise livré) |
