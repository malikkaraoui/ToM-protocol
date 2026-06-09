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

### 1. Contrat Rust ↔ Swift

- **Oui, les noms de champs correspondent exactement** entre `NodeStatusFFI` et `TomNodeStatus.CodingKeys` :
  - `node_id` ↔ `nodeId`
  - `status` ↔ `status`
  - `peers_count` ↔ `peersCount`
  - `groups_count` ↔ `groupsCount`
  - `local_role` ↔ `localRole`
  - `path_kind` ↔ `pathKind`
  - `path_rtt_ms` ↔ `pathRttMs`
- **Pas de champ manquant** sur ce contrat précis.
- **Compatibilité de types** :
  - `node_id`, `status`, `local_role`, `path_kind` → `String` JSON, OK.
  - `path_rtt_ms: u64` → `UInt64?` Swift, **OK exact**.
  - `peers_count` et `groups_count` sont `u64` côté Rust mais `Int` côté Swift.
    - **Compatible en pratique** pour des cardinalités réalistes.
    - **Pas mathématiquement total** : si Rust sérialise une valeur `> Int.max`, le `JSONDecoder` Swift échouera. Sur Apple TV arm64, `Int` est signé 64 bits.
    - Donc le contrat est robuste **pour l’usage réel**, mais pas “preuve formelle sur tout `u64`”.

### 2. Suffisance des tests

Les 2 tests ajoutés améliorent nettement la situation, mais **ils ne suffisent pas à eux seuls pour verrouiller entièrement le contrat**.

Ce qu’ils couvrent bien :
- les **bonnes clés JSON** côté Rust,
- la **ré-échappement correcte** d’une chaîne contenant `"`.

Ce qui manque encore :

1. **Un test de fixture sans dépendre de l’ordre des champs**
   - `node_status_json_keys_match_swift_decoder` verrouille aussi l’**ordre exact** produit par serde.
   - Cet ordre n’est pas vraiment le contrat interop ; les **noms des clés** le sont.
   - Le test actuel peut casser pour une raison bénigne, et inversement ne dit rien sur le décodage Swift réel.

2. **Un test de bornes pour `peers_count/groups_count`**
   - Si Swift garde `Int`, il faut au minimum verrouiller que les valeurs émises restent dans la plage décodable côté Apple.
   - Je recommanderais un test Rust avec `i64::MAX as u64` comme limite haute “interop Swift sûre”, ou alors un changement Swift vers `UInt64` si vous voulez supprimer cette zone grise.

3. **Un vrai smoke test côté Apple**
   - Le contrat critique est Rust → JSON → `JSONDecoder` Swift.
   - Aujourd’hui, le repo verrouille surtout Rust → JSON → Rust.
   - Il manque idéalement un test Swift qui décode un payload fixe construit pour ce contrat.

4. **Un test d’échappement plus large**
   - Vous couvrez le guillemet double, qui était le bug réel.
   - Il manque encore au moins un cas avec backslash / newline / Unicode pour éviter un faux sentiment de sécurité “on a testé un caractère, donc tout va bien”.

### 3. Autres JSON manuels dans `crates/tom-protocol-ffi/src/lib.rs`

- **Dans ce fichier précis, je ne vois pas d’autre payload JSON FFI encore construit à la main via `format!()`**.
- Les autres sorties JSON repérées passent par `serde_json::to_string(...)` :
  - `tom_node_receive_messages()`
  - `tom_node_connected_peers()`
  - `tom_node_discovered_peers()`
- Les autres `format!()` présents servent à fabriquer des **messages de log / strings métier**, pas du JSON FFI.

### Verdict court

- Le fix `NodeStatusFFI` est **le bon correctif**.
- Le contrat de clés Swift est **correct**.
- Le seul point de type à surveiller est **`u64` Rust → `Int` Swift** pour `peers_count/groups_count`.
- Le plus gros manque aujourd’hui est **l’absence d’un test de décodage Swift réel**.

**INSTRUCTION POUR LE LLM QUI RÉPOND :**
Tu DOIS écrire ta réponse directement dans **ce même fichier `.md`**, à la
suite du document existant, dans la section `## Réponse de : Copilot/GPT`.
Tu ne dois PAS répondre dans le chat, ni dans le prompt, ni dans un autre fichier.
Ce document est le vase communicant entre GPT et Claude. Utilise Edit/Write.
Quand tu as fini, dis : "J'ai répondu dans `docs/handoffs/2026-06-09-ffi-nodestatusffi-contrat-swift.md`."

---

## Intégration

### Actions retenues

**Aucune régression détectée.** Le contrat de clés est correct, le fix est sain.

Points à adresser dans une prochaine session :

1. **Type `u64` → Swift `Int`** (peers_count/groups_count) : pas un bug aujourd'hui
   (arm64 = Int 64-bit signé, valeurs réalistes < i64::MAX), mais zona grise
   formelle. → changer les champs Swift en `UInt64` ou ajouter un test de borne
   Rust avec `i64::MAX as u64` pour verrouiller explicitement.

2. **Tests de contrat à enrichir** (`types.rs`) :
   - Ajouter test escaping backslash + newline + Unicode (au-delà du seul `"`)
   - Ajouter test de décodage côté Swift (XCTest fixture sur payload fixe)
   - Le test JSON actuel verrouille l'ordre serde — extraire une assertion
     séparée sur les noms de clés uniquement (plus stable)

3. **Autres `format!` dans lib.rs** : aucun autre JSON manuel trouvé ✅

### Statut

`intégré` — correctif immédiat non requis, tickets ouverts ci-dessus pour la suite.
