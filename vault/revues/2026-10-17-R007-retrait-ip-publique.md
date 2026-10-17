---
date: 2026-10-17
revue: R007
mandat: M0009
branche: fix/retrait-ip-publique
verdict: RESERVE
tip: e7f6ce8e6fa67a8045f9df5478d9c7ee88de2186
niveau: N2
---

# R007 — Doublage N2 retrait IP publique (tour 4)

## Historique

- **Tour 1 : RESERVE.** IPv6 non retirée, garde en sortie 1 sans IP. Corrigé par `ec8b180` (retrait du préfixe) et `79e139b` (préfixe /64 dans la garde, code de sortie sans IP).
- **Tour 2 : RESERVE.** IPv4 à zéros de tête non détectée. Corrigé par `a7912d1` (forme canonique avant hachage).
- **Tour 3 : RESERVE.** Pré-sélection `git grep` dépendante de la locale (ligne Latin-1 ignorée sous UTF-8). Corrigé par `e7f6ce8` : plus de pré-sélection, lecture de tous les fichiers de `git ls-files -z` en octets, binaires ignorés (NUL dans les 8000 premiers octets), décodage UTF-8 avec remplacement.
- **Tour 4 (ce rapport) : RESERVE.** Le correctif `e7f6ce8` tient sur le contenu des fichiers réguliers. La garde ne lit pas trois autres objets suivis (nom de fichier, cible de lien symbolique, blob d'index d'un fichier supprimé du disque) : une IP y passe. Deux points mineurs de mandat s'y ajoutent (placeholders non échappés dans quatre documents).

## Méthode et garde-fous

- Worktree lecture seule, HEAD = `e7f6ce8`. Aucun fichier suivi modifié, aucun commit, aucun `cargo`, aucun réseau, aucun SSH. `git status --short` vide à la fin.
- Valeurs de référence chargées depuis la base `9278dca` dans des variables shell et un script Python, jamais affichées. Les sorties contenant une adresse ont été masquées (`<IP>`).
- Matrice exécutée dans un dépôt jetable neuf : `scratchpad/r007-matrice-depot/`. Stubs réseau (`ssh`, `sshpass`, `autossh`, `scp`, `rsync`, `curl`, `nc`) dans `scratchpad/r007-stubs-depot/`. Aucun `rm -rf`.
- Garde-fou de lecture : sorties de la garde = `fichier:ligne` uniquement, pas de valeur.

## 1. Correctif `e7f6ce8` : lecture du script et matrice

### 1.1 Lecture du script (100 lignes)

- Pas de pré-sélection : chaque fichier de `git ls-files -z` est lu en octets. Correct.
- Binaire : test NUL sur les 8000 premiers octets. Correct, mais cela exclut aussi l'UTF-16 (voir remarque R3).
- Décodage UTF-8 avec `replace` : une ligne Latin-1 n'est plus perdue. Correct.
- Hachage : forme canonique (zéros de tête retirés, minuscules) avant SHA-256. Correct pour IPv4 et IPv6.
- Lookahead sur la position de départ et sur les préfixes du dernier octet/groupe : voulu, pour « IP suivie d'un chiffre » (`d93f57b`). Conséquence : un voisin dont le dernier octet commence par celui d'une IP interdite est signalé (voir remarque R4).
- Exemptions : comparaison exacte du chemin ; les lignes `#` sont ignorées. Correct.
- Lien symbolique : `os.path.islink` → fichier ignoré. **Défaut, voir réserve R1b.**
- Fichier supprimé du disque : `os.path.isfile` → ignoré. **Défaut, voir réserve R1c.**
- Nom de fichier : jamais testé. **Défaut, voir réserve R1a.**
- Fichier illisible : `open()` lève une exception non interceptée (trace Python, sortie 1). Pas de faux OK, voir remarque R5.

### 1.2 Relance du tip (`LC_ALL=C` et `LC_ALL=fr_FR.UTF-8`)

- `bash scripts/verifier-ip-interdites.sh` dans le worktree : sortie 0 sous les deux locales, message `OK (aucune IP interdite)`.
- Les 3 empreintes de `scripts/ip-interdites.sha256` (2 IPv4 + 1 préfixe /64) sont bien celles des valeurs de référence (contrôle par `shasum -a 256` puis `grep -c`, résultat 1 pour chacune).
- Le fichier d'exemptions ne contient que `vault/30-discoveries.md` (la ligne de commentaire est ignorée).

### 1.3 Matrice (dépôt jetable, 35 cas, deux locales)

Légende : `rc` = code de sortie, `tb` = trace Python. « att » = attendu.

| Cas | Contenu testé | att | C | fr | Verdict |
|---|---|---|---|---|---|
| 01 | IPv4 brute | 1 | 1 | 1 | OK |
| 02 | IPv4 collée à un chiffre avant | 1 | 1 | 1 | OK |
| 03 | IPv4 collée à un chiffre après | 1 | 1 | 1 | OK |
| 04 | IPv4 collée à des lettres | 1 | 1 | 1 | OK |
| 05 | IPv4 suivie d'un point final | 1 | 1 | 1 | OK |
| 06 | IPv4 zéros de tête sur tous les octets | 1 | 1 | 1 | OK |
| 07 | IPv4 zéros de tête sur le dernier octet | 1 | 1 | 1 | OK |
| 08 | IPv4 mappée `::ffff:` | 1 | 1 | 1 | OK |
| 09 | IPv6 forme de base | 1 | 1 | 1 | OK |
| 10 | IPv6 majuscules, groupes zéro-complétés | 1 | 1 | 1 | OK |
| 11 | IPv6 collée (lettres avant et après) | 1 | 1 | 1 | OK |
| 12 | IPv6 `::/64` | 1 | 1 | 1 | OK |
| 13 | IPv6 zéros de tête, `0:0:1` | 1 | 1 | 1 | OK |
| 14 | Latin-1 avec IPv4 | 1 | 1 | 1 | OK |
| 15 | Latin-1 sans IP (contrôle) | 0 | 0 | 0 | OK |
| 16 | CRLF | 1 | 1 | 1 | OK |
| 17 | UTF-16 avec IPv4 | 0 | 0 | 0 | OK (limite de conception, voir R3) |
| 18 | Binaire NUL avec IPv4 | 0 | 0 | 0 | OK (voulu) |
| 19 | Sans IP | 0 | 0 | 0 | OK |
| 20 | Nom avec espace et `:` | 1 | 1 | 1 | OK |
| 21 | Lignes neutres (`1.2.3.4`, `8.8.8.8`, littéral IPv6 générique du FAI, `2001:db8:0:1`, `0.0.0.0`, `010.001.0.1`) | 0 | 0 | 0 | OK |
| 22 | IPv4 suivie d'un chiffre | 1 | 1 | 1 | OK (voulu) |
| 23 | **IPv4 dans le nom de fichier** | 1 | **0** | **0** | **ÉCART (R1a)** |
| 24 | **Lien symbolique dont la cible contient une IPv4** | 1 | **0** | **0** | **ÉCART (R1b)** |
| 25 | Lien symbolique vers cible neutre | 0 | 0 | 0 | OK |
| 26 | **Fichier suivi (index contient l'IP), supprimé du disque** | 0 | 0 | 0 | **Passe à tort (R1c)** |
| 27 | Ligne d'1 Mo neutre | 0 | 0 | 0 | OK (0,9 s) |
| 28 | Ligne d'1 Mo avec IP en fin | 1 | 1 | 1 | OK (0,7 s) |
| 29 | Fichier suivi, illisible (chmod 0) | - | rc 1 + trace | rc 1 + trace | Sortie 1 non explicite (R5) |
| 30 | Deux fichiers, aucun IP | 0 | 0 | 0 | OK |
| 31 | IP dans `vault/30-discoveries.md` | 0 | 0 | 0 | OK (exempté) |
| 32 | Voisin dont le dernier octet commence par celui d'une IP interdite | 1 | 1 | 1 | Faux positif voulu (R4) |
| 33 | IPv4 de la seconde adresse flotte | 1 | 1 | 1 | OK |
| 34 | Seconde adresse flotte zéro-complétée | 1 | 1 | 1 | OK |
| 35 | IPv6 avec groupes zéro ajoutés | 1 | 1 | 1 | OK |

Bilan : 32 cas conformes, 2 écarts sur le périmètre demandé (23, 24), 1 écart sur un objet suivi (26), 1 trace (29). Aucun plantage Python sur les entrées hostiles. Aucune sortie ne contient de valeur.

Cas de plantage : aucun, hors 29. Les lignes d'1 Mo passent en moins d'une seconde.

Limites acceptées : binaires (18), UTF-16 traité comme binaire (17, voir R3).

## 2. Non-régression du tip (`e7f6ce8`)

Les comptages sont faits avec `git grep --cached` (index) et des motifs construits à partir des valeurs de référence, avec zéros de tête tolérés (`0*`) et casse ignorée. Le fichier exempté est exclu.

| Contrôle | Résultat |
|---|---|
| IPv4 domicile, zéro-complétée ou non, dans le contenu suivi (hors `vault/30-discoveries.md`) | 0 |
| Seconde IPv4 flotte, idem | 0 |
| Préfixe IPv6 /64, idem | 0 |
| Même motif dans les noms de fichiers suivis | 0 |
| Même motif dans les cibles de liens symboliques | 0 |
| Contrôle de sensibilité : motif IPv4 sur le fichier exempté | 5 lignes trouvées (le motif fonctionne) |
| Contrôle de sensibilité : motif IPv6 sur le fichier exempté | 2 lignes trouvées |
| IPv4 publiques restantes (tous types, hors `vault/30`) | 65 occurrences ; aucune ne se rattache aux plages des deux adresses publiques (0) |
| Littéraux IPv6 du préfixe FAI générique restants | 3 occurrences, 2 formes, dans les 2 fichiers prévus par le mandat |
| IPv4 privées | Une seule ligne dans `docs/archive/RECAP-DEV-2026-03-05.md` (plage 192.168, privée, hors périmètre) |
| PDF (`docs/article ToM protocol.pdf`, `docs/plans/2026-04-01-plan-maitre-reseau-vivant-v2.pdf`) : texte extrait par `pdftotext` | 0 occurrence dans les deux |
| 4 JSONL `experiments/iroh-poc/results/*.jsonl` modifiés | Valides : 24, 783, 139 et 2505 lignes JSON |
| `CLAUDE.md` | 31 373 caractères (≤ 40 000) |
| `git log --format=%B 9278dca..HEAD` (7 commits) | 0 IPv4, 0 littéral IPv6 du préfixe FAI, 0 `Co-Authored-By` |
| Auteur et committeur des 7 commits | `…@gmail.com` (conforme) |
| `bash -n` des 4 scripts touchés (`car-test.sh`, `deploy-nas-node.sh`, `nas-node-ctl.sh`, `verifier-ip-interdites.sh`) | OK |
| Variables absentes, scripts lancés avec stubs réseau (`env -i`, PATH piégé) | `car-test.sh`, `deploy-nas-node.sh`, `nas-node-ctl.sh` : sortie 1 et aucun appel réseau, sans variable. Avec `TOM_NAS_SSH_PASS` seul ou `TOM_HOME_HOST` seul : sortie 1 et aucun appel. |
| Mot de passe SSH (9 caractères, lu dans `9278dca`) | 0 occurrence dans l'index du tip. **1 occurrence dans l'historique `9278dca`** (voir R6). |
| Placeholders du guide `docs/INFRA-LOCAL-DEPLOYMENT-GUIDE.md` (hors blocs de code et hors `code`) | 0 non échappé |

## 3. Remarques (verdict inchangé)

- **R3, UTF-16.** Un fichier UTF-16 contenant une IP passe (cas 17). Le filtre NUL traite ce fichier comme binaire, comme le fait `git` lui-même (`git diff` affiche « Bin »). Acceptable si on l'écrit dans la liste des limites ; à acter explicitement.
- **R4, faux positifs voulus.** Cas 32 : un voisin dont le dernier octet commence par celui d'une IP interdite est signalé. C'est le compromis de `d93f57b`. Les IPs neutres du cas 21 passent.
- **R5, fichier illisible.** Cas 29 : exception Python, sortie 1, pas de message `fichier:ligne`. C'est fail-closed, donc acceptable, mais le message est opaque. Un `try/except OSError` qui signale le fichier serait plus clair. Rien ne continue silencieusement.
- **R7, garde non câblée.** `verifier-ip-interdites.sh` n'est appelé nulle part : ni `scripts/pre-push-gate.sh`, ni les 29 fichiers de `.github/`. C'est une garde sans déclencheur. À câbler dans la gate de push ou en CI, ou à documenter comme manuelle.
- **R8, numéros de ligne.** Lecture du code (non testée) : `str.splitlines()` coupe aussi sur `\x0c`, `\x1c`-`\x1e`, `\x85`, ` `, ` `. Les numéros rapportés peuvent différer de `git grep -n`. Le détecteur, lui, ne manque pas une IP pour autant.
- **R9, deploy-nas-node.sh.** Sans binaire compilé (`target/…/tom-chat` absent), sortie 1 sans message sur stderr. Le contrôle de variable est correct ; le message d'erreur manque.
- **R10, `sshpass -p "$NAS_SSH_PASS"`** (dans `car-test.sh`). Le mot de passe passe en argument de ligne de commande, donc visible par `ps` sur la machine locale pendant l'exécution. Schéma préexistant, pas introduit par cette branche.
- **R11, commit touchant `vault/reprise/`.** La branche modifie `vault/reprise/2026-10-17-ETAT-DES-LIEUX.md` (2 lignes, sans IP). À vérifier que cela figure dans le mandat M0009, car l'interdiction de toucher `vault/reprise/` vaut pour les relecteurs, pas nécessairement pour l'auteur.

## 4. Réserves (défauts prouvés hors limites acceptées)

### R1, la garde lit le disque, pas l'objet suivi

Cause : le script itère sur `git ls-files` puis lit chaque chemin sur le disque, avec des exclusions (`islink`, `isfile`) et un filtre de contenu. Trois objets suivis échappent. Les limites acceptées (binaires, fichiers non suivis, écritures exotiques) ne couvrent aucun des trois.

- **R1a, nom de fichier.** Un fichier `note-<IPv4 interdite>.txt` au contenu neutre passe (cas 23). Le nom est public dans l'arbre.
- **R1b, lien symbolique.** Un lien dont la cible contient l'IPv4 passe (cas 24). La cible est un blob texte de l'index, lu par `git`, donc publiée.
- **R1c, fichier suivi puis supprimé du disque.** Le cas 26 montre que `git add f`, `rm f` puis `git commit` committe le blob contenant l'IP alors que la garde passe. `git ls-files -s` le confirme (statut `AD`).

Correctif suggéré (un seul mécanisme, à la source) : itérer sur `git ls-files -s -z` ; tester le nom (octets) ; lire le blob par `git cat-file --batch` pour tous les modes sauf `160000` (sous-modules) ; ne jamais lire le disque. Cela supprime les trois exclusions et couvre liens et blobs d'index. Ajouter un test de non-régression sur les cas 23, 24 et 26 de la matrice.

Note de méthode (R0 du tour 4) : c'est le quatrième tour sur la même classe de défaut (couverture de la garde). Les correctifs précédents ont élargi la lecture du contenu ; le mécanisme (lecture disque et filtre de contenu) est en cause, il faut le changer plutôt qu'ajouter une exception.

### R2, placeholders non échappés dans la prose

Le mandat exige des placeholders échappés en HTML. Quatre documents touchés par la branche contiennent encore `<IP_DOMICILE>` en prose (hors blocs de code et hors backticks). GitHub les rend invisibles ou les interprète comme balises :

- `docs/archive/RECAP-DEV-2026-03-05.md` (2 lignes)
- `docs/archive/freebox-bug-report-arp-wifi-vm.md` (1 ligne)
- `docs/plans/2026-03-30-apple-tv-3rd-node.md` (1 ligne)
- `docs/plans/capstone-run-checklist.md` (1 ligne)

Pas de fuite (ce sont des placeholders, pas des valeurs). Correctif : `&lt;IP_DOMICILE&gt;`, ou un backtick. Le guide `docs/INFRA-LOCAL-DEPLOYMENT-GUIDE.md` est conforme.

### R6, mot de passe SSH dans l'historique public (hors périmètre de la branche)

Le mot de passe SSH (9 caractères) figure dans le commit de base `9278dca`. Il n'est pas dans le tip, mais l'historique est public. Il faut le faire tourner (rotation). Ce n'est pas corrigeable par cette branche, puisque l'historique n'est pas réécrit. À signaler au fondateur.

## 5. Verdict

**RESERVE.**

Le correctif `e7f6ce8` tient sur le contenu des fichiers réguliers : matrice conforme sous `LC_ALL=C` et `fr_FR.UTF-8`, aucun plantage sur les entrées hostiles, tip propre sur toutes les formes d'IP testées.

Réserves à lever avant GO :

1. **R1 (a, b, c).** Nom de fichier, cible de lien symbolique et blob d'index non lus par la garde. Corriger à la source (index + `cat-file`) et ajouter les cas 23, 24 et 26 comme test.
2. **R2.** Quatre documents avec placeholders non échappés en prose.

Remarques non bloquantes : R3 (acter UTF-16), R4, R5 (message d'erreur), R7 (câbler la garde), R8, R9, R10, R11, et R6 (rotation du mot de passe, à signaler au fondateur).
