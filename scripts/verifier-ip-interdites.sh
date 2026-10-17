#!/usr/bin/env bash
# verifier-ip-interdites.sh — garde anti-retour des IP publiques de l'infra perso.
#
# Le dépôt est public : les IP du domicile/de la flotte ne doivent jamais y
# revenir. Elles ne sont PAS écrites ici : scripts/ip-interdites.sha256 contient
# le SHA-256 (texte exact, sans retour à la ligne) de chaque élément interdit :
#   - une IPv4 : « a.b.c.d » ;
#   - un préfixe IPv6 /64 : les 4 premiers groupes en minuscules, sans zéros de
#     tête, séparés par « : » (ex. « 2001:db8:0:1 » pour 2001:0DB8:0000:0001::/64).
# Le script hache chaque candidat trouvé dans les fichiers texte suivis par git et
# compare. Il n'affiche jamais la valeur trouvée, seulement fichier:ligne.
#
# Ce n'est PAS un secret : un SHA-256 d'IPv4 (ou d'un /64 dans un /32 connu) se
# retrouve par force brute. La garde évite le texte clair et le retour par
# inadvertance ; les valeurs interdites restent dans l'historique git public.
#
# Exemptions temporaires : scripts/ip-interdites.exemptions (un chemin par ligne).
# Limites : fichiers binaires (PDF compressés, images), fichiers non suivis et
# écritures non standard (entier, hexadécimal, séparateurs exotiques) non vus.
#
# Ajouter une valeur : printf '%s' '<valeur>' | shasum -a 256 | cut -d' ' -f1 >> scripts/ip-interdites.sha256
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
# -z : noms séparés par NUL (certains contiennent « : » ou des espaces).
# git grep sort en 1 quand rien ne correspond : ce n'est pas une erreur.
{ git grep -z -l -I -i -E \
    '[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}|[0-9a-f]{1,4}:[0-9a-f]{1,4}:[0-9a-f]{1,4}:[0-9a-f]{1,4}' \
    -- . || [[ $? -eq 1 ]]; } \
    | python3 -I -c '
import hashlib, re, sys
hashs_path, exempt_path = sys.argv[1], sys.argv[2]
interdits = {l.split()[0].lower() for l in open(hashs_path) if l.strip() and not l.startswith("#")}
exempts = {l.strip() for l in open(exempt_path) if l.strip() and not l.startswith("#")}

def h(texte):
    return hashlib.sha256(texte.encode()).hexdigest()

# Lookahead : un candidat par position de départ (chevauchements), et toutes les
# longueurs du dernier élément, pour ne pas rater une valeur collée à d autres
# caractères avant ou après (ex. « 1<ip>9 »).
v4 = re.compile(r"(?=(\d{1,3}\.\d{1,3}\.\d{1,3}\.)(\d{1,3}))")
v6 = re.compile(r"(?=((?:[0-9a-f]{1,4}:){3})([0-9a-f]{1,4}))", re.I)

def norm6(groupes):
    return ":".join(g.lstrip("0").lower() or "0" for g in groupes)

def interdite(ligne):
    for m in v4.finditer(ligne):
        for k in range(1, len(m.group(2)) + 1):
            if h(m.group(1) + m.group(2)[:k]) in interdits:
                return True
    for m in v6.finditer(ligne):
        tete = m.group(1).rstrip(":").split(":")
        for k in range(1, len(m.group(2)) + 1):
            if h(norm6(tete + [m.group(2)[:k]])) in interdits:
                return True
    return False

trouve = 0
for chemin in sys.stdin.buffer.read().split(b"\0"):
    if not chemin:
        continue
    nom = chemin.decode("utf-8", "replace")
    if nom in exempts:
        continue
    with open(chemin, "r", encoding="utf-8", errors="replace") as f:
        for no, ligne in enumerate(f, 1):
            if interdite(ligne):
                print(f"{nom}:{no}: <IP_INTERDITE>")
                trouve += 1
if trouve:
    print(f"verifier-ip-interdites: {trouve} ligne(s) avec une IP interdite", file=sys.stderr)
    sys.exit(1)
print("verifier-ip-interdites: OK (aucune IP interdite)")
' "$hashs" "$exemptions"
