# L1-002 — L'entropie non-biaisable, par la murmuration

> **Design-first — pas de code protocolaire avant validation de Malik.**
> Cadrage (29/07) : **recherche-first** — on affronte le mur de l'imbiaisabilité AVANT
> de construire le tirage tournant. Chantier = ce document **+ deux simulations** qui
> prouvent/réfutent (chiffres réels, §3 et §4), pas des affirmations.
>
> Ancrage : `CARTE-rendezvous-tournant.md` §1 (la murmuration), règle #1 « l'aléa qui
> fait tourner ». **Révisé 07/08** après 3 relectures adversariales Fable 5 (doc,
> simulation-instrument, rotation) — voir « Journal des relectures » en fin.

---

## 0. Où on en est (vérifié dans le code)

| Brique | Rôle règle #1 | État réel | Fichier |
|---|---|---|---|
| **L1-001** | produit le **seed** (l'aléa) depuis les signatures d'attestation, ordre-indépendant | ✅ **FAIT** | `presence/aggregator.rs`, `presence_seed()` |
| **L1-003** | la **vue vivante** quorum-signée (l'« état vivant ») | ✅ **FAIT** (build 39) | `presence/quorum.rs`, `relay_view.rs` |
| **L1-002** | consomme seed + vue → **tire le rôle qui tourne, sans biais, round après round** | ❌ **CE CHANTIER** | — |

---

## 1. Le problème, sans jargon

Le tirage doit désigner l'hôte du rendez-vous (le « secrétaire » tournant) de façon que
**personne ne puisse se faire tirer plus souvent que sa part**. Trois obstacles :

1. **Le trou de L1-001, dit par le code** (`aggregator.rs:9-12`) : *celui qui agrège
   choisit QUELLES attestations inclure → grinding par sous-ensemble OUVERT*. Plus
   généralement : **quiconque peut recalculer la graine en local peut RE-TIRER**.
2. **La contrainte dure** : ToM est **P2P asynchrone, SANS horloge**. Une VDF (aléa
   par délai *mesuré*) suppose un temps absolu invérifiable en async — rejetée (minuteur
   mécanique) et classée « recherche, littérature mince » (`TOM-PLAN-GLOBAL.md` M1.2).
3. **La ROTATION dans le temps** (révélé par l'audit) : le rôle change à chaque vague.
   Un tirage sain isolé ne garantit RIEN sur la séquence : boucle de rétroaction,
   runs consécutifs (censure durable), monopole temporel. **C'est le cœur du sujet** (§4).

---

## 2. La piste de la nature (notre analyse murmuration)

Rappel carte §1 (STARFLAG) : chaque étourneau suit ~7 voisins fixes, l'info se propage en
**vagues scale-free**, **aucune vue globale**.

**La bonne question** : *comment une nuée vire sans que personne ne puisse prédire NI
diriger le virage ?* La direction **émerge** de milliers d'ajustements locaux ; l'influence
d'un oiseau — ou d'un petit groupe complice — est **diluée** sur toute la structure
corrélée. Deux mécanismes remplacent l'horloge :

- **(a) La vague comme ordre sans horloge.** Le « temps » de la nuée se compte en **hops
  de propagation témoignés**, pas en secondes : on ne peut pas *fast-forward* une vague,
  il faut réellement **être vu** par ses ~7 voisins.
  > ⚠️ **Honnêteté (audit)** : c'est une **analogie de conception, pas une VDF
  > cryptographique**. Un hop réseau a un délai variable et bricolable ; la « profondeur
  > de vague » n'est pas un délai infalsifiable. Ce qu'elle apporte n'est pas un *délai*
  > mais un *ordre témoigné* (H1 ci-dessous). Ne pas la vendre comme un substitut de VDF.
- **(b) Le témoignage comme engagement.** Être vu par ses voisins **avant** que la graine
  soit révélable = **être engagé par le fait d'être observé**. On ne peut plus « recalculer
  puis choisir ».

Et **la masse dilue** : plus la nuée est grande, plus l'influence d'un individu tend vers
zéro — **mais ce pilier a une limite précise, mesurée en §3 (EXP-3/4)**.

---

## 3. Simulation A — le TIRAGE (un round) : `grinding_murmuration.py`

Stdlib pure, **sha256 réel**, déterministe. Sélection = **tirage égal** :
gagnant `= argmin_id sha256(graine‖id)`. Métrique : **avantage = P(grinder tiré) − f**.

> ⚠️ **Ce que cette simulation prouve — et NE prouve PAS (correction d'audit).**
> Elle **isole la couche tirage/grinding d'UN round** : « étant donné K essais effectifs,
> quel avantage ». **EXP-0b fige `honest_blob` avant les re-tirages du grinder — c'est
> précisément l'hypothèse H1 (engagement-avant-révélation).** Donc EXP-0b **ne valide pas
> H1, il la SUPPOSE**, puis en déduit que re-tirer = K tirages Bernoulli(f). Le résultat
> est **conditionnel** : *SI H1 tient, alors avantage = 1−(1−f)^K*. La validité de H1 en
> async est l'objet de la couche « vague réelle » (§8), pas un acquis.

**Validé (sha256 réel, seeds 42 & 7) :** sélection uniforme (EXP-0a : écart ≤ bande de bruit
~3σ) → baseline équitable = f=G/N. EXP-0b : empirique ≈ `1−(1−f)^K` **sous H1**.

**EXP-1 — Régime naïf (re-tirage libre), f=0,10 :** avantage **identique** de N=50 à 50 000 ;
**0,90 à T=10⁶**. → **la masse seule ne protège PAS du re-tirage**. Le trou reste ouvert.

**EXP-2 — Régime témoigné (K=b+1 essais), f=0,10 :** b=0 → **0** (équité) ; b=1 → 0,090 ;
b=2 → 0,171 ; b=5 → 0,369. → le témoignage plafonne les essais **si** b reste petit.

**EXP-3 — adversaire de TAILLE FIXE G (b=1) :** G=5 → avantage 0,047 (N=100) … **5·10⁻⁵**
(N=10⁵). → contre un adversaire absolu, la masse **écrase** l'avantage.

**EXP-4 — adversaire à FRACTION constante f (b=1) :** f=0,10 → 0,090 **plat en N**.

> ⚠️ **Limite du pilier (correction d'audit).** « Plus de monde = plus fort » ne vaut
> QUE contre un adversaire de **taille absolue fixe** (EXP-3). Contre un adversaire
> **proportionnel** (botnet à f=10 % constant — **le cas réaliste**), la masse **n'aide
> pas** (EXP-4, plat). La digue est alors le **témoignage (H1) + le fade (H3) + le quorum
> (H4)**, pas le nombre. La masse est un amplificateur, jamais une garantie à elle seule.

---

## 4. Simulation B — la ROTATION (T rounds) : `rotation_murmuration.py`

**La simulation A masquait le vrai risque.** Ici on modélise la dynamique temporelle. Le
**feedback** est modélisé honnêtement — *tenir le rôle → pouvoir de re-tirer la graine du
round suivant* (force **ρ ∈ [0,1]**) ; ρ=0 = aucun pouvoir (équité), ρ=1 = contrôle quasi
total. Métrique : **Φ_T = fraction du temps où l'adversaire tient le rôle** (équité ⟺ Φ_T≈f).

> Correctif de modèle vs 1er proto : un « feedback = hash(base‖gagnant) » laisse la graine
> uniforme → **aucun** auto-renforcement. Le seul levier qui crée une vraie boucle, c'est
> d'accorder au tenant des **essais** sur la graine suivante. C'est ce qui est simulé.

Résultats **f=0,10, N=50, T=400, robustes sur seeds 42 & 7** :

**Balayage ρ (sans cooldown) — où bascule-t-on de l'équité à la capture ?**

| ρ (pouvoir conféré par le rôle) | Φ_T | R_max (run le + long) | verdict |
|---|---|---|---|
| 0 | **0,10 = équité parfaite** | 2 | le tournant sain |
| 0,2 | 0,17 | 6 | biais |
| 0,4 | 0,29 | 13 | biais |
| 0,6 | 0,43 | 26 | **CAPTURE** |
| 0,8 | 0,58 | 44 | capture |
| 1,0 | 0,70 | 66 | capture quasi totale |

→ **La rotation crée une capture expansive réelle** dès que le rôle donne prise sur la
graine suivante (ρ ≳ 0,6). Le tirage-unique (sim A) ne pouvait pas le voir.

**Balayage cooldown (tour de garde, règle #2) contre ρ=0,9 FORT :**

| C (repos après un gain) | Φ_T | R_max | note |
|---|---|---|---|
| 0 | 0,65 | 61 | capture, censure durable |
| 2 | 0,40 | 22 | encore capturé |
| 4 | 0,25 | 8 | runs cassés |
| 5 | 0,24 | **5** | runs cassés, **butée viabilité** |
| 10 | 0,21 | 5 | Φ_T ne descend plus à f |

**Enseignement (net) — deux menaces distinctes, deux parades distinctes :**

1. **Capture de fréquence** (Φ_T ≫ f) → se tue **à la SOURCE** en gardant **ρ bas** :
   c'est un **quorum q-of-n** qui co-signe l'ancrage de la graine, de sorte que **le tenant
   seul ne contrôle pas la graine suivante**. Réutilise L1-003. **C'est la parade n°1.**
2. **Runs / censure durable** (R_max grand) → le **cooldown** : il casse les runs
   (R_max 61→5) **mais ne restaure PAS l'équité** (Φ_T reste ~0,24 = 2,4× f) et **bute sur
   la viabilité** : il faut C≥4-5, or petit réseau exige **C < 1/(2f) = 5** → pile à la
   limite pour f=0,10, **impossible pour f≥0,2**.

> **ρ est le paramètre de sécurité n°1 du rendez-vous tournant.** Ni la masse ni le
> cooldown seuls ne matent le feedback ; seul le **quorum d'ancrage** le fait, à la source.

---

## 5. Ce que L1-002 doit RÉELLEMENT garantir (hypothèses porteuses)

- **H1 — Engagement avant révélation.** La contribution d'un nœud à la graine de la vague
  *w* est **témoignée par ≥q de ses ~7 voisins AVANT** que la graine de *w* soit calculable
  (une contribution/nœud/vague, comme le challenge one-shot L1-001). **NON prouvée** (sim A
  la suppose ; à tester en §8). Réutilise L1-001/L1-003.
- **H2 — Imprévisibilité à l'engagement.** La graine dépend d'assez de contributions
  honnêtes que le grinder ne peut pas prédire quand il s'engage (⊇ quorum L1-003).
- **H3 — Abstention détectée + faded.** Retirer une contribution témoignée est
  **publiquement constatable** → coûte (fade), ce qui **borne b**. **NON modélisée** :
  dans la sim, b est un *paramètre imposé*, pas une conséquence du fade. Le lien fade→b
  reste à mesurer (§8). Sans H3, l'abstention rouvre le grinding.
- **H4 — Le rôle ne confère aucune prise sur la graine suivante (ρ→0).** *(Nouvelle,
  imposée par la sim B.)* Garantie par la **co-signature quorum q-of-n** de l'ancrage :
  le tenant du round *w* ne peut pas re-tirer la graine de *w+1*. **C'est la parade
  centrale contre la capture expansive.** À spécifier et prouver.

---

## 6. Les 3 candidats du plan global, remis à leur place (instruments)

- **Beacon passif** (hash des N derniers messages) = la turbulence de la nuée — support de
  la graine émergente (**H2**). Retenu comme *source*.
- **Signature-seuil t-of-n** du churn présent = **le quorum qui co-signe la graine** —
  matérialise **H1 ET H4** (engagement témoigné + pas de contrôle par le tenant seul).
  🎯 **piste d'implémentation centrale** (renforcée par la sim B).
- **VDF** = second rideau, seulement si H1-par-témoignage échoue ; à n'ouvrir que si
  nécessaire. Ne pas s'appuyer sur « profondeur-de-vague = VDF » (analogie, pas preuve).

---

## 7. Critère de garde L1.2bis (avant d'autoriser M1.3 / le rôle tournant)

Sur la **simulation étendue (§8)** ET un **banc réseau réel** :

- **(a) H1 tenu** : un changement de contribution après témoignage est détectable ; un nœud
  ne fait pas accepter > b contributions/vague.
- **(b) H4 tenu** : ρ effectif mesuré ≈ 0 (le tenant ne biaise pas la graine suivante) →
  **Φ_T ≈ f** sur T grand, et **R_max borné** (runs cassés par cooldown viable).
- **(c) H3 tenu** : l'abstention est détectée et *faded*, et le **b effectif** émergent est
  petit (≤1) — mesuré, pas supposé.
- **(d) seuil chiffré** : `avantage = 1−(1−f)^(b+1)−f` et `Φ_T−f` sous une cible **à fixer
  conjointement avec M1.4** (anti-Sybil) en fonction du f max toléré. **Aucun chiffre
  (ex-« 0,02 ») n'est justifiable AVANT M1.4** — le retirer d'ici là.
- **(e) revue cryptographe externe** (le plan le note : « le seul point où l'auto-évaluation
  ne suffit pas »).

---

## 8. Tensions ouvertes & limites reconnues (issues des audits)

1. **✅ Le point fixe sur flotte stable — TRANCHÉ (décision Malik, 07/08) : on ASSUME
   « le rôle tourne au rythme de l'activité ».** Fait vérifié qui fonde la décision : le
   seed n'est PAS ancré sur l'annuaire de présence, mais sur une **fenêtre glissante de
   30 s d'attestations éphémères** (`state.rs:2584` → `presence.aggregate_seed()` ;
   `PRESENCE_TTL_MS=30_000`, `mod.rs:47` ; *« ephemeral: 30s lifetime »*, `attestation.rs:6`).
   Une attestation = **preuve d'activité récente constatée** → le contenu de la fenêtre
   change dès qu'il y a de la vie → le seed tourne **même à membres stables**. Doctrine
   actée : (a) le rôle tourne dès qu'il y a du pouls (présence qui bouge OU trafic/relais) ;
   (b) il **se pose sans conséquence** quand la flotte est **figée EN MEMBRES *et* MUETTE**
   (0 attestation/30 s) — car alors **rien à héberger** ; la première vie le fait re-tourner ;
   (c) **zéro horloge** introduite (fidèle règle #1 + murmuration : une nuée posée ne vire
   pas, ce n'est pas un bug) ; (d) ce n'est **pas une capture** — sous H4 un secrétaire figé
   a été tiré équitablement **une fois** (il reste, il n'a pas triché ; censure passive
   faible car nœuds en vue directe sur flotte stable). **Écarté** : le compteur de vague
   (horloge logique) et le cooldown anti-figement (horloge déguisée + butée de viabilité
   §4). Le cooldown reste réservé à sa fonction §4 (casser les runs sous feedback).
2. **H1 supposée, non prouvée** (pétition de principe EXP-0b) → couche « vague réelle ».
3. **fade→b non modélisé** (b est une fixture) → couche fade-dynamique.
4. **Pilier « masse=force » limité aux adversaires absolus** (EXP-4) — assumé explicitement.
5. **« Profondeur-de-vague » = analogie**, pas VDF cryptographique — ne pas s'y adosser.

## 9. Prochaines étapes (design-first — pas de code protocolaire avant go)

1. **Couche fade-dynamique** : modéliser fade + f dynamique → mesurer le **b effectif** sur
   100+ vagues (ferme #3 et le seuil).
2. **Couche vague réelle** : propagation ~7 voisins + témoignage q-of-k + révélation
   retardée → **tester H1/H4** et le rushing/dernier-révélateur (ferme #2).
3. **Point fixe tranché** (décision 07/08, §8.1) → spécifier le protocole de vague.
4. **Revue cryptographe externe** du principe.
5. **Seulement après garde L1.2bis verte** : implémenter, puis M1.3.

---

## Annexe — reproduire

```bash
cd docs/plans/L1-002-sim
python3 grinding_murmuration.py --seed 42 --rounds 600   # sim A (tirage)  -> results-seed42.txt
python3 rotation_murmuration.py --seed 42 --rounds 400   # sim B (rotation) -> results-rotation-seed42.txt
```
Validé seeds **42 & 7** (verdicts qualitatifs concordants).

## Journal des relectures Fable 5 (07/08)

- **Doc** : EXP-0b = pétition de principe (fige H1) ; seuil 0,02 arbitraire ; pilier limité
  aux adversaires absolus ; **point fixe sur flotte stable**. → intégrés §3, §4, §7, §8.
- **Simulation-instrument** : anti-fabrication OK (chiffres doc == script) ; feedback
  inter-rounds non testé. → sim B ajoutée (§4).
- **Rotation** : condition de contraction (quorum + bruit honnête + fade) ; `E[R_max] ≈
  ln(T)/ln(1/f)` ; viabilité cooldown `C < 1/(2f)`. → §4 + H4.

## Sources

- `docs/plans/CARTE-rendezvous-tournant.md` §1 (murmuration : STARFLAG / PMC / PLOS).
- `docs/plans/TOM-PLAN-GLOBAL.md` M1.2–M1.4 · code : `presence/aggregator.rs`,
  `presence/quorum.rs`, `runtime/state.rs::presence_seed`.
