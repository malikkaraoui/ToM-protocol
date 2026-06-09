# Handoff — FFI : contrat JSON NodeStatusFFI ↔ décodeur Swift

> Date : 2026-06-09
> Type : review
> Priorité : haute (bug silencieux possible côté tvOS)
> reviewedRange: a84b469..6b912b2

---

## De : Claude (Sonnet 4.6)

### Contexte

Commit `6b912b2` — `fix(ffi): serde via NodeStatusFFI`

La fonction C `tom_node_status()` dans `crates/tom-protocol-ffi/src/lib.rs`
renvoyait un JSON construit à la main via `format!()`. Un champ contenant un
guillemet (ex : `local_role = "we\"ird"`) produisait du JSON malformé qui
faisait échouer silencieusement le décodeur Swift `TomNodeStatus` côté
tvOS/iOS — l'UI freezait sans aucun log d'erreur.

Le fix remplace le `format!` par une struct `NodeStatusFFI` dérivant
`Serialize`/`Deserialize` (serde), et y ajoute deux tests de contrat qui
verrouillent les noms de clés exactes attendus par le décodeur Swift.

### Fichiers à lire

- `crates/tom-protocol-ffi/src/types.rs` — struct `NodeStatusFFI` + tests (lignes 126–244)
- `crates/tom-protocol-ffi/src/lib.rs` — fonction `tom_node_status()` (chercher `NodeStatusFFI`)
- `apps/tom-node-tvos/TomNode/Models/TomModels.swift` — struct Swift `TomNodeStatus` (décodeur)

### Question précise

1. **Contrat de nommage** : les champs `node_id`, `status`, `peers_count`,
   `groups_count`, `local_role`, `path_kind`, `path_rtt_ms` dans `NodeStatusFFI`
   correspondent-ils exactement aux clés `CodingKeys` du `TomNodeStatus` Swift ?
   Y a-t-il un champ manquant ou un type incompatible (ex : Swift attend `Int`
   mais Rust sérialise `u64` → JSON number → ça passe, mais vérifier) ?

2. **Tests de contrat** : les deux tests dans `types.rs` sont-ils suffisants pour
   garantir la stabilité du contrat ? Que manque-t-il (ex : test round-trip
   `nil`/valeurs limites, test de déserialisation côté Swift) ?

3. **Risque résiduel** : y a-t-il d'autres `format!` ou constructions JSON
   manuelles dans `lib.rs` qui mériteraient le même traitement ?

### Contraintes / hors scope

- Ne pas proposer de réécrire l'architecture FFI (unifistruct vs fonctions C)
- Se concentrer sur la robustesse du contrat JSON existant
- Le décodeur Swift est `Codable` standard, pas de customisation

---

## Réponse de : Copilot/GPT

**INSTRUCTION POUR LE LLM QUI RÉPOND :**
Tu DOIS écrire ta réponse directement dans **ce même fichier `.md`**, à la
suite du document existant, dans la section `## Réponse de : Copilot/GPT`.
Tu ne dois PAS répondre dans le chat, ni dans le prompt, ni dans un autre fichier.
Ce document est le vase communicant entre GPT et Claude. Utilise Edit/Write.
Quand tu as fini, dis : "J'ai répondu dans `docs/handoffs/2026-06-09-ffi-nodestatusffi-contrat-swift.md`."

---

## Intégration
<!-- Claude remplit après lecture de la réponse -->
