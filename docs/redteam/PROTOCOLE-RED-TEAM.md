# ToM — Protocole Red Team autonome (plan de vol)

> **Doctrine** : je suis l'attaquant. Le réseau n'est PAS préparé en amont. Je
> l'attaque, j'observe sa réaction, je note scrupuleusement, je boucle. J'invente
> des attaques toujours plus sophistiquées, longues, aléatoires, brutales. Quand
> une attaque **perce**, le code (pas moi) doit apprendre : je diagnostique, je
> patche, je recompile un nouveau build, et je relance. On rejoue toujours les
> anciennes attaques (une évolution du code peut rouvrir un trou). **La boucle ne
> s'arrête jamais.**
>
> État : plan prêt. Exécution sur signal (« je suis à la maison avec les devices »).
> Beaucoup d'attaques ne nécessitent PAS de devices (elles visent la logique du
> protocole sur des nœuds contrôlés) → la boucle peut démarrer sans la flotte, et
> s'étendre au vrai matériel quand il est branché.

---

## 1. La boucle (machine à états)

```
        ┌─────────────────────────────────────────────────────────┐
        │                                                         │
        ▼                                                         │
   [SÉLECTION] ─► [ATTAQUE] ─► [OBSERVATION] ─► [VERDICT] ─► [JOURNAL]
        ▲            │                              │            │
        │            │                       DÉFENDU │            │ PERCÉ
        │            │                              │            ▼
        │            │                              │      [DIAGNOSTIC]
        │            │                              │            │
     [ESCALADE] ◄────┴──────── [RÉGRESSION] ◄───────┴──── [PATCH+REBUILD]
```

1. **SÉLECTION** — piocher la prochaine attaque : corpus connu (régression) ∪ attaque neuve (générateur).
2. **ATTAQUE** — lancer contre le réseau non préparé (nœuds contrôlés et/ou devices réels).
3. **OBSERVATION** — capturer la réaction (métriques, mémoire, CPU, hang, panic, latence, famine du trafic honnête).
4. **VERDICT** — PERCÉ (réseau blessé) ou DÉFENDU.
5. **JOURNAL** — écrire l'entrée (build, horodatage, attaque, params/seed, observation, verdict).
6. Si PERCÉ → **DIAGNOSTIC** (cause racine) → **PATCH** du code → **REBUILD** → l'attaque doit maintenant être DÉFENDUE → ajout au corpus de régression.
7. **RÉGRESSION** — périodiquement, rejouer TOUT le corpus contre le nouveau build.
8. **ESCALADE** — le générateur allonge, combine, randomise, brutalise davantage. Retour en 1.

---

## 2. L'arsenal (taxonomie des attaques)

Chaque attaque a un **id**, des **paramètres** (fuzzables), une **cible** (nœud contrôlé / device réel / relais), et un **critère de perçage**.

### L1 — Présence (Proof of Presence)
| id | Attaque | Perçage si… |
|---|---|---|
| `pres.flood` | rafale de challenges au-delà du budget | latence honnête explose / OOM / budget ne plafonne pas |
| `pres.forge` | attestation à signature forgée (send_raw) | `accepted` monte (drop_bad_signature devrait) |
| `pres.replay` | rejeu d'attestation valide capturée | 2ᵉ acceptée (one-shot devrait droper) |
| `pres.usurp` | relais du chemin répond à la place de la cible | acceptée d'un `from` ≠ cible |
| `pres.reflect` | challenge forgé « from A » vers N nœuds | attestations concentrées sur A |
| `pres.sybil` | essaim de fausses identités qui s'attestent | quorum/gate contourné sans évidence relais |
| `pres.grind` | grinding de seed par choix de sous-ensemble | seed prévisible (ouvert → L1-002) |
| `pres.skew` | dérèglement d'horloge (offset extrême) | fraîcheur cassée / faux rejets massifs |
| `pres.mem` | saturation des collections (challenges/attestations) | mémoire non bornée |
| `pres.malformed` | payloads MessagePack tordus | panic / drop non compté |

### L0 — Transport
| id | Attaque | Perçage si… |
|---|---|---|
| `tr.connstorm` | tempête de connexions QUIC | épuisement fd / hang |
| `tr.slowloris` | connexions ouvertes lentes, jamais finies | famine des connexions honnêtes |
| `tr.oversize` | envelopes > plafond | pas rejeté / amplification |
| `tr.fragbomb` | bombe de réassemblage (chunks) | OOM (RÉGRESSION : déjà durci 347421b) |
| `tr.msgpack` | sérialisation gonflante | inflation mémoire (RÉGRESSION : fix serde_bytes) |

### Disponibilité / Chaos
| id | Attaque | Perçage si… |
|---|---|---|
| `chaos.monkey` | kill/revive/skew aléatoires (Simian Army) | réseau wedge, ne se répare pas |
| `chaos.partition` | split-brain | double-acceptation / pas de guérison |
| `chaos.churn` | départs/retours massifs | découverte ne reprend pas |
| `chaos.starve` | CPU/mémoire sous pression | watchdog / jetsam (device) |

### Découverte
| id | Attaque | Perçage si… |
|---|---|---|
| `disc.squat` | occupation des slots rendez-vous DHT | pairs légitimes évincés (trou connu ADR-010 #2) |
| `disc.fakepeer` | injection de faux pairs | dial vers des fantômes / pollution topologie |

---

## 3. Le générateur (invente des attaques neuves)

Au-delà du corpus, un moteur produit des attaques **jamais vues** :
- **Fuzzing de paramètres** : intensité, timing, taille, ordre, nombre d'identités.
- **Chaînage** : combiner 2-3 attaques simultanées (ex. `pres.flood` + `chaos.monkey` + `pres.skew`).
- **Randomisation** (seedée → reproductible) : ordre et cibles aléatoires.
- **Escalade** : à chaque tour, +durée, +sophistication, +brutalité. Un compteur `sophistication_level` monte ; les attaques de haut niveau sont longues, multi-vecteurs, adaptatives (réagissent à ce que le réseau encaisse).

Chaque attaque générée qui **perce** est cristallisée dans le corpus (avec son seed) → elle devient un test de régression permanent.

---

## 4. L'observation (ce que je capture)

| Signal | Source | Seuil de perçage |
|---|---|---|
| Compteurs présence | `presence_metrics()` (build 20) | `accepted` d'une attestation illégitime > 0 |
| Mémoire (RSS) | `ps` sur le process cible | croissance non bornée |
| CPU | `ps` | spin à 100 % soutenu |
| Hang | watchdog (timeout sur une requête handle) | pas de réponse en N s |
| Panic | capture stderr / exit code | tout panic |
| Latence honnête | trafic honnête en parallèle | explose vs baseline |
| Famine | trafic honnête bloqué pendant l'attaque | acceptations honnêtes → 0 |
| Crash device | app tuée (jetsam/watchdog) | app disparaît de la flotte |

Baseline mesurée AVANT chaque attaque (le réseau au repos) → le verdict est un **delta**.

---

## 5. Le journal (notes scrupuleuses)

Deux fichiers, append-only, jamais réécrits :

- `docs/redteam/journal.jsonl` — machine, une ligne par run :
  ```json
  {"ts":"...","build":"<git sha>","attack":"pres.forge","params":{"seed":42,"rate":500},
   "target":"controlled|device:<name>","observation":{"accepted_illegit":0,"rss_mb":54,"hang":false,"panic":false},
   "verdict":"DEFENDED|BREACH","note":"...","fix_commit":null}
  ```
- `docs/redteam/JOURNAL.md` — humain, narratif : chaque perçage raconté (attaque → symptôme → cause racine → patch → re-test vert), chaque campagne résumée.

**Règle d'or** : rien n'est effacé. Une attaque qui a percé un jour reste dans le corpus POUR TOUJOURS (régression).

---

## 6. Régression (on rejoue le passé)

À chaque nouveau build, avant d'inventer du neuf :
1. Rejouer **tout** le corpus (`docs/redteam/corpus.jsonl` — la liste des attaques + seeds).
2. Priorité aux attaques ayant déjà percé (`was_breach: true`).
3. Toute attaque du corpus qui **reperce** = régression = STOP, on corrige avant d'avancer.

> « Avec l'évolution du code, on n'est pas à l'abri que ça repasse. » → le corpus est le filet.

---

## 7. La boucle d'apprentissage du code

Quand une attaque perce :
1. **Reproduire** de façon déterministe (seed noté).
2. **Diagnostiquer** la cause racine (file:line).
3. **Patcher** — durcir le protocole, jamais masquer le symptôme (§5 anti-hallucination : la correction doit adresser la vraie cause).
4. **Rebuild** (workspace + FFI + XCFramework si la flotte est concernée), bump build number.
5. **Re-test** : l'attaque exacte doit passer DÉFENDU.
6. **Régression** : le corpus complet doit rester vert.
7. **Journal** : entrée de perçage avec `fix_commit`.

Chaque tour de perçage→patch = potentiellement un nouveau build (21, 22, 23…). Le numéro de build trace l'évolution du durcissement.

---

## 8. Intégration de la flotte (quand les devices sont là)

Sans devices (maintenant) : cibles = nœuds contrôlés (tom-stress). Couvre TOUTE la logique protocole.

Avec devices (au signal) : cibles += iPhone×2 / iPad / Mac / Apple TV / NAS. Le harnais attaque le **vrai matériel** via `send_raw` (envelopes forgés) + floods + chaos, et observe via `fleet-probe` + les compteurs remontés par les apps (`presence_metrics()` FFI). Ajoute les perçages spécifiques matériel : jetsam Apple TV, veille iOS, 4G/CGNAT.

---

## 9. Autonomie & cadence

- Pilote : une boucle (`/loop` ou un driver `redteam-run`) — chaque itération = un cycle complet §1.
- Je décide des patchs et des rebuilds en autonomie (gate verte obligatoire avant tout commit).
- Cadence : attaque courte (~30-90 s) → verdict → journal ; campagne longue (multi-vecteurs) périodiquement.
- Le compteur de sophistication monte tant que le réseau tient ; il redescend d'un cran après un perçage (on consolide avant de re-escalader).

---

## 10. Garde-fous (sécurité)

- **Cibles autorisées uniquement** : ma propre flotte / mes devices / le NAS. Jamais le réseau ToM public.
- **Sandbox réseau** : nœuds contrôlés en `n0_discovery(false)`, relais privé ; aucune attaque ne fuit dehors.
- **Hooks de simulation compilés hors prod** : les APIs offensives (skew, injection brute) ne partent pas dans les apps end-user (pas dans le FFI).
- **Réversibilité** : chaque patch est un commit atomique ; rollback possible si une correction casse autre chose.

---

## 11. État du harnais (prêt / à construire)

| Brique | État |
|---|---|
| Compteurs présence par issue (`presence_metrics`) | ✅ build 20 |
| Pilotage (challenge en lot, all-online) | ✅ build 20 |
| Sim horloge (skew) + gate configurable | ✅ build 20 |
| `fleet-probe` (orchestrateur live devices) | ✅ build 20 |
| `chaos-monkey` (kill/revive/skew Simian Army) | ✅ build 21 |
| Attaquant `send_raw` (forge/replay/usurp sur QUIC réel) | 🔨 build 21 (à finir) |
| Journal + corpus (fichiers append-only) | 🔨 squelette créé |
| Driver de boucle (sélection→attaque→verdict→journal) | 🔨 à construire |
| Générateur d'attaques neuves (fuzz + chaînage + escalade) | 🔨 à construire |
| Watchdog hang + capture panic/RSS | 🔨 à construire |

---

## 12. Premiers pas à l'exécution (quand tu fais signe)

1. Démarrer la boucle sur nœuds contrôlés (pas besoin de devices) — remplir le corpus, trouver les premiers perçages, patcher.
2. Brancher la flotte → étendre les attaques au vrai matériel.
3. Laisser tourner ; relever les perçages ; livrer les builds durcis au fil de l'eau.

*Le réseau ne se prépare pas à mes attaques. Il les subit, saigne, puis apprend. Et je recommence, plus fort, pour toujours.*
