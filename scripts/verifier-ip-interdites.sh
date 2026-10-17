#!/usr/bin/env bash
# verifier-ip-interdites.sh — garde anti-retour des IP publiques de l'infra perso.
#
# Le dépôt est public : les IP du domicile/de la flotte ne doivent jamais y
# revenir. Elles ne sont PAS écrites ici : scripts/ip-interdites.sha256 contient
# le SHA-256 de chaque IPv4 interdite (texte exact, sans retour à la ligne).
# Le script hache chaque IPv4 trouvée dans les fichiers texte suivis par git et
# compare. Il n'affiche jamais l'IP trouvée, seulement fichier:ligne.
#
# Exemptions temporaires : scripts/ip-interdites.exemptions (un chemin par ligne).
# Limite : les fichiers binaires (PDF compressés, images) ne sont pas inspectés.
#
# Ajouter une IP : printf '%s' '<ip>' | shasum -a 256 | cut -d' ' -f1 >> scripts/ip-interdites.sha256
#
# Usage : bash scripts/verifier-ip-interdites.sh
# Sortie : 0 = rien trouvé · 1 = IP interdite trouvée · 2 = erreur d'usage
set -euo pipefail

racine="$(git rev-parse --show-toplevel)"
hashs="$racine/scripts/ip-interdites.sha256"
exemptions="$racine/scripts/ip-interdites.exemptions"

if [[ ! -s "$hashs" ]]; then
    echo "verifier-ip-interdites: liste de hashs absente ou vide ($hashs)" >&2
    exit 2
fi
[[ -f "$exemptions" ]] || exemptions=/dev/null

cd "$racine"
# -z : noms de fichiers séparés par NUL (certains contiennent « : » ou des espaces).
git grep -z -l -I -E '[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}' -- . \
    | python3 -I -c '
import hashlib, re, sys
hashs_path, exempt_path = sys.argv[1], sys.argv[2]
interdits = {l.split()[0].lower() for l in open(hashs_path) if l.strip() and not l.startswith("#")}
exempts = {l.strip() for l in open(exempt_path) if l.strip() and not l.startswith("#")}
# Lookahead : candidats qui se chevauchent, pour ne pas rater une IP collée à
# d autres chiffres (ex. « x182.67… » contient aussi « 82.67… »).
cand = re.compile(r"(?=(\d{1,3}(?:\.\d{1,3}){3}))")
trouve = 0
for chemin in sys.stdin.buffer.read().split(b"\0"):
    if not chemin:
        continue
    nom = chemin.decode("utf-8", "replace")
    if nom in exempts:
        continue
    with open(chemin, "r", encoding="utf-8", errors="replace") as f:
        for no, ligne in enumerate(f, 1):
            for m in cand.finditer(ligne):
                if hashlib.sha256(m.group(1).encode()).hexdigest() in interdits:
                    print(f"{nom}:{no}: <IP_INTERDITE>")
                    trouve += 1
                    break
if trouve:
    print(f"verifier-ip-interdites: {trouve} ligne(s) avec une IP interdite", file=sys.stderr)
    sys.exit(1)
print("verifier-ip-interdites: OK (aucune IP interdite)")
' "$hashs" "$exemptions"
