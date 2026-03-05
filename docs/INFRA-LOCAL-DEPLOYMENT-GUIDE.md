# Infrastructure Locale 3-4 Relays — Guide de Déploiement

**Date** : 2026-03-05
**Objectif** : Déployer et valider une infrastructure multi-relay ToM Protocol en local (LAN) avant migration prod.
**Durée estimée** : 6-10h sur 3 jours (3 relays) ou +2h pour 4ème relay (Apple TV)
**Budget** : 0€ (infrastructure maison)

---

## Vue d'ensemble

## Plan strict d'avancement (ordre verrouillé)

Ce plan répond explicitement à la question : **"qu'est-ce qui est déjà opérationnel"** et **"quoi installer sur chaque machine"**.

### Périmètre cible final

Réseau ToM indépendant à 5 nœuds :

1. MacBook Pro (orchestrateur / tests)
2. MacBook Air 2011 (relay)
3. Apple TV HD (relay, via app tvOS)
4. NAS Freebox Delta (relay + discovery)
5. Freebox V6 beaux-parents (relay distant)

---

### Phase 0 — Baseline NAS Freebox Delta (déjà opérationnel : quoi exactement)

**Statut attendu pour considérer le NAS “opérationnel”** :

- [x] `tom-relay` démarre en service (systemd)
- [x] Endpoints relay répondent : `/health`, `/healthz`, `/ready`
- [x] Endpoint metrics répond : `:9090/metrics`
- [x] `tools/relay-discovery` répond : `/health`, `/relays`, `/metrics`, `/status`
- [x] Hooks de smoke observability passent en local

**Important** : cela valide le **socle local**. Ce n'est pas encore **100% du réseau final** tant que :

- [ ] MacBook Air n'est pas intégré en relay stable
- [ ] Apple TV n'est pas intégrée
- [ ] Freebox V6 distante n'est pas intégrée et testée
- [ ] campagne multi-noeuds complète n'est pas validée

**Conclusion concrète** : NAS = **opérationnel socle (≈80%)**, réseau final 5 nœuds = **pas encore 100%**.

---

### Phase 1 — MacBook Air 2011 (installation ToM : fichiers requis)

#### Réponse directe : faut-il un `.py` ou un `.dmg` ?

**Non.** Pour ToM (stack Rust), le chemin recommandé est via **binaires** + **plist launchd**.

#### Artefacts minimum à avoir sur le MacBook Air

1. Binaire relay : `target/release/tom-relay`
2. (Optionnel mais utile test) binaire stress : `target/release/tom-stress`
3. Service launchd : `~/Library/LaunchAgents/com.tom.relay.plist`

Artefacts prêts dans le repo :

- Template plist : `deploy/macos/com.tom.relay.air.plist`
- Script d'installation remote : `scripts/install-mac-air-relay.sh`

Exécution type (depuis le MacBook Pro) :

```bash
chmod +x scripts/install-mac-air-relay.sh
AIR_HOST=<ip-ou-hostname-mac-air> AIR_USER=<user-mac-air> ./scripts/install-mac-air-relay.sh
```

Option : copier aussi `tom-stress` sur le Mac Air :

```bash
AIR_HOST=<ip-ou-hostname-mac-air> AIR_USER=<user-mac-air> AIR_COPY_STRESS=1 ./scripts/install-mac-air-relay.sh
```

#### Critère GO Phase 1

- [ ] `curl http://<IP_MAC_AIR>:3341/health` = 200
- [ ] `curl http://<IP_MAC_AIR>:9091/metrics` répond
- [ ] discovery retourne le relay Mac Air dans `/relays`

---

### Phase 2 — Apple TV HD (pour "rentrer dans la danse")

#### Réponse directe : qu'est-ce qu'il faut ?

Apple TV ne supporte pas un daemon Linux classique. Il faut une **app tvOS wrapper** signée depuis Xcode.

#### Artefacts / prérequis nécessaires

1. `rustup target add aarch64-apple-tvos`
2. Binaire compilé tvOS : `target/aarch64-apple-tvos/release/tom-relay`
3. Projet Xcode tvOS (wrapper) avec le binaire embarqué
4. Signature Apple (certificat + provisioning profile)

Starter prêt dans le repo :

- `apps/relay-tvos/README.md`
- `apps/relay-tvos/TomRelay/` (fichiers Swift + header FFI)
- `scripts/apple-tv-preflight.sh`
- `scripts/build-apple-tv-relay.sh`

Quickstart :

```bash
chmod +x scripts/apple-tv-preflight.sh scripts/build-apple-tv-relay.sh
./scripts/apple-tv-preflight.sh
./scripts/build-apple-tv-relay.sh
```

#### Critère GO Phase 2

- [ ] app lancée sur Apple TV
- [ ] relay actif sur `:3343`
- [ ] `curl http://<IP_APPLE_TV>:3343/health` = 200 depuis MacBook Pro

> Sans wrapper tvOS signé, l'Apple TV ne peut pas être un relay ToM autonome.

---

### Phase 3 — Freebox V6 (beaux-parents)

Intégration après Mac Air + Apple TV pour éviter de déboguer 3 variables à la fois.

#### Prérequis réseau

- [ ] SSH OK sur la V6
- [ ] architecture confirmée (`armv7l` attendu)
- [ ] transfert du binaire `tom-relay` compilé en `armv7-unknown-linux-gnueabihf`

#### Critère GO Phase 3

- [ ] relay V6 actif sur `:3342`
- [ ] health V6 accessible depuis MacBook Pro et NAS
- [ ] relay V6 visible dans discovery `/relays`

---

### Phase 4 — Validation réseau indépendant 5 nœuds

Composition validée :

- MacBook Pro (tests)
- MacBook Air (relay)
- Apple TV (relay)
- NAS Delta (relay + discovery)
- Freebox V6 (relay)

#### Tests de sortie obligatoires (go/no-go)

1. **Discovery** retourne 4 relays actifs (NAS + Air + AppleTV + V6)
2. **Failover** : arrêt relay NAS puis continuité >90%
3. **Résilience discovery down** : fallback HTTP→DNS→hardcoded fonctionne
4. **Campaign** multi-noeuds sans crash

Si ces 4 points sont verts, on considère le **petit réseau indépendant opérationnel**.

---

### Ordre d'exécution non négociable

1. NAS baseline validé
2. MacBook Air intégré
3. Apple TV intégrée
4. Freebox V6 intégrée
5. Validation finale 5 nœuds

Ne pas sauter d'étape : chaque phase réduit le risque de debug croisé.

### Pourquoi local d'abord ?

✅ **Gratuit** (pas de VPS)
✅ **Debug facile** (accès SSH direct, pas de firewall cloud)
✅ **Tests rapides** (latence LAN <10ms)
✅ **Validation complète** (multi-relay, failover, monitoring) avant prod
✅ **Itération rapide** (redémarrer relay = 2 secondes)

### Architecture cible

**Setup minimal (3 relays)** :
```
┌─────────────────────────────────────────────────────┐
│ Discovery Service (NAS ou Mac)                      │
│ http://192.168.0.83:8080/relays                     │
│ ├─ Health polling (10s)                             │
│ └─ Returns 3+ relays actifs                         │
└─────────────────────────────────────────────────────┘
                         ↓
        ┌────────────────┼────────────────┐
        ↓                ↓                ↓
   [Relay 1]        [Relay 2]        [Relay 3]
   NAS Freebox      MacBook Air      Freebox V6
   192.168.0.83     192.168.0.X      192.168.0.Y
   :3340            :3341            :3342
   ├─ /health       ├─ /health       ├─ /health
   └─ :9090/metrics └─ :9091/metrics └─ :9092/metrics
```

**Setup étendu (4 relays, optionnel)** :
```
┌─────────────────────────────────────────────────────┐
│ Discovery Service (NAS ou Mac)                      │
│ http://192.168.0.83:8080/relays                     │
│ ├─ Health polling (10s)                             │
│ └─ Returns 4 relays actifs                          │
└─────────────────────────────────────────────────────┘
                         ↓
    ┌────────────────────┼─────────────────────┐
    ↓         ↓          ↓           ↓
[Relay 1] [Relay 2] [Relay 3]   [Relay 4]
NAS       MacBook   Freebox V6 Apple TV HD
.83:3340  .X:3341   .Y:3342    .Z:3343
```

**Clients** (Mac, NAS, autre) se connectent via discovery, auto-failover si 1-2 relays down.

---

## Prérequis

### Infrastructure disponible

- [x] **NAS Freebox Delta** : 192.168.0.83, Debian ARM64, déjà utilisé pour dev
- [x] **MacBook Air 2011** : macOS, peut faire relay
- [x] **Apple TV HD** : tvOS 16.3, A8 ARM64, toujours connecté, excellent pour relay (voir Phase 3bis)
- [ ] **Freebox V6** (2 étages chez beaux-parents) : ARM Cortex-A9, 256MB RAM, relay 3 alternatif

**Recommandation** :

- **Setup minimal (3 relays)** : NAS + Mac + Freebox V6
- **Setup étendu (4 relays)** : NAS + Mac + Freebox V6 + Apple TV HD

### Software installé

**Sur chaque machine** :
- [x] Rust toolchain (`curl https://sh.rustup.rs | sh`)
- [x] Git (`apt install git` ou `brew install git`)
- [x] SSH access configuré

**Sur Mac (monitoring)** :
- [ ] Prometheus (`brew install prometheus`)
- [ ] Grafana (`brew install grafana`)

### Réseau

- [x] Toutes les machines sur même LAN (192.168.0.x)
- [x] Ports ouverts (pas de firewall entre machines LAN)

---

## Phase 1 : Setup Relay 1 (NAS Freebox)

**Durée** : 30min
**Machine** : NAS Freebox Delta (192.168.0.83)

### Étape 1.1 : Upgrade tom-relay

```bash
# SSH dans NAS
ssh root@192.168.0.83

# Update repo
cd /root/tom-protocol
git pull

# Rebuild relay
cd crates/tom-relay
cargo build --release

# Vérifier binaire
./target/release/tom-relay --version
```

**Validation** :
```bash
./target/release/tom-relay --version
# Attendu : tom-relay 0.1.0 (ou version actuelle)
```

---

### Étape 1.2 : Config systemd service

```bash
# Créer service systemd
cat > /etc/systemd/system/tom-relay.service <<'EOF'
[Unit]
Description=ToM Protocol Relay 1 (NAS)
After=network.target

[Service]
Type=simple
User=root
WorkingDirectory=/root/tom-protocol/crates/tom-relay
ExecStart=/root/tom-protocol/target/release/tom-relay \
    --http-addr 0.0.0.0:3340 \
    --metrics-addr 0.0.0.0:9090
Restart=on-failure
RestartSec=5s
StartLimitInterval=0

[Install]
WantedBy=multi-user.target
EOF

# Reload systemd
systemctl daemon-reload

# Enable auto-start
systemctl enable tom-relay

# Start service
systemctl start tom-relay
```

**Validation** :
```bash
# Check status
systemctl status tom-relay

# Attendu : Active: active (running)
```

---

### Étape 1.3 : Test health endpoint

```bash
# Health check local
curl http://localhost:3340/health

# Health check depuis autre machine LAN
curl http://192.168.0.83:3340/health

# Attendu : {"status":"ok"} ou HTTP 200
```

```bash
# Metrics check
curl http://192.168.0.83:9090/metrics | grep tom_relay

# Attendu : Lignes Prometheus (counters, gauges)
```

**Checkpoint 1** :
- [ ] Relay 1 (NAS) build OK
- [ ] Service systemd actif
- [ ] Health endpoint répond 200 OK
- [ ] Metrics endpoint expose counters

---

## Phase 2 : Setup Relay 2 (MacBook Air)

**Durée** : 45min
**Machine** : MacBook Air 2011 (192.168.0.X)

### Étape 2.1 : Build tom-relay sur Mac

```bash
# Sur Mac
cd ~/tom-protocol
git pull

# Build relay
cd crates/tom-relay
cargo build --release

# Vérifier binaire
./target/release/tom-relay --version
```

---

### Étape 2.2 : Déterminer IP LAN du Mac

```bash
# Trouver IP LAN
ifconfig | grep "inet " | grep -v 127.0.0.1

# Ou
ipconfig getifaddr en0  # WiFi
ipconfig getifaddr en1  # Ethernet

# Noter l'IP : 192.168.0.X (exemple : 192.168.0.150)
```

**Note** : Remplacer `192.168.0.X` par l'IP réelle dans toutes les commandes suivantes.

---

### Étape 2.3 : Option A - Lancer relay en mode dev (simple)

```bash
# Lancer relay en background
nohup ./target/release/tom-relay \
    --http-addr 0.0.0.0:3341 \
    --metrics-addr 0.0.0.0:9091 \
    > /tmp/relay-2.log 2>&1 &

# Noter le PID
echo $! > /tmp/relay-2.pid

# Vérifier
curl http://localhost:3341/health
```

**Pour arrêter** :
```bash
kill $(cat /tmp/relay-2.pid)
```

---

### Étape 2.4 : Option B - Service launchd (auto-start, recommandé)

**Créer plist** : `~/Library/LaunchAgents/com.tom.relay.plist`

```bash
cat > ~/Library/LaunchAgents/com.tom.relay.plist <<'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.tom.relay</string>
    <key>ProgramArguments</key>
    <array>
        <string>/Users/malik/tom-protocol/target/release/tom-relay</string>
        <string>--http-addr</string>
        <string>0.0.0.0:3341</string>
        <string>--metrics-addr</string>
        <string>0.0.0.0:9091</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>/tmp/tom-relay-2.log</string>
    <key>StandardErrorPath</key>
    <string>/tmp/tom-relay-2-error.log</string>
</dict>
</plist>
EOF

# Load service
launchctl load ~/Library/LaunchAgents/com.tom.relay.plist

# Vérifier
launchctl list | grep tom
```

**Pour arrêter** :
```bash
launchctl unload ~/Library/LaunchAgents/com.tom.relay.plist
```

---

### Étape 2.5 : Test health endpoint

```bash
# Local
curl http://localhost:3341/health

# Depuis NAS
ssh root@192.168.0.83 'curl http://192.168.0.X:3341/health'

# Attendu : {"status":"ok"}
```

**Checkpoint 2** :

- [ ] Relay 2 (Mac) build OK
- [ ] Service lancé (dev ou launchd)
- [ ] Health endpoint répond 200 OK
- [ ] Accessible depuis NAS

---

## Phase 3 : Setup Relay 3 (Freebox V6)

**Durée** : 1h
**Machine** : Freebox V6 (192.168.0.Y, chez beaux-parents, 2 étages au-dessus)

**⚠️ Note** : Freebox V6 = matériel ancien (Cortex-A9, 256MB RAM). Cross-compilation recommandée depuis Mac (build natif très lent).

### Étape 3.1 : Activer SSH sur Freebox V6

**Via interface Freebox OS** :
1. Ouvrir http://mafreebox.freebox.fr
2. Paramètres → Mode avancé → Serveur SSH
3. Activer SSH (port 22)
4. Définir mot de passe root

**Validation** :
```bash
# Depuis Mac, tester SSH
ssh root@192.168.0.Y

# Si connexion OK → continuer
```

---

### Étape 3.2 : Déterminer architecture Freebox V6

```bash
# SSH dans Freebox V6
ssh root@192.168.0.Y

# Vérifier archi
uname -m

# Attendu : armv7l (ARM Cortex-A9, 32-bit)
```

**⚠️ Freebox V6 specs** :

- CPU : ARM Cortex-A9 (32-bit)
- RAM : 256MB
- OS : Debian (stripped down)

**Recommandation** : **Ne PAS build nativement** (trop lent, risque d'OOM). Utiliser cross-compilation (Option B).

---

### Étape 3.3 : (OPTIONNEL) Installer Rust sur Freebox V6

**⚠️ SKIP cette étape** si vous utilisez cross-compilation (Option B recommandée).

```bash
# Sur Freebox V6 (seulement si Option A)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Choisir installation par défaut (1)

# Charger environnement
source $HOME/.cargo/env

# Vérifier
cargo --version
```

---

### Étape 3.4 : (OPTIONNEL) Installer build tools

**⚠️ SKIP cette étape** si vous utilisez cross-compilation (Option B recommandée).

```bash
# Sur Freebox V6 (seulement si Option A)
apt update
apt install -y build-essential git
```

---

### Étape 3.5 : Option A - Build sur Freebox V6 (PAS RECOMMANDÉ, 30-60min)

```bash
# Clone repo
git clone https://github.com/your-org/tom-protocol.git
cd tom-protocol/crates/tom-relay

# Build (peut être lent sur ARM)
cargo build --release

# Vérifier
./target/release/tom-relay --version
```

---

### Étape 3.6 : Option B - Cross-compile depuis Mac (FORTEMENT RECOMMANDÉ, 5min)

**⚠️ Important** : Freebox V6 est ARM Cortex-A9 **32-bit** (`armv7l`), pas ARM64. Utiliser target `armv7-unknown-linux-gnueabihf`.

**Sur Mac** :

```bash
# Install cross-compilation tool
cargo install cross

# Build pour ARM 32-bit (Freebox V6)
cd ~/tom-protocol/crates/tom-relay
cross build --release --target armv7-unknown-linux-gnueabihf

# Vérifier binaire
ls -lh target/armv7-unknown-linux-gnueabihf/release/tom-relay

# SCP vers Freebox V6
scp target/armv7-unknown-linux-gnueabihf/release/tom-relay \
    root@192.168.0.Y:/root/tom-relay
```

**Sur Freebox V6** :
```bash
# Rendre exécutable
chmod +x /root/tom-relay

# Vérifier
/root/tom-relay --version
```

---

### Étape 3.7 : Lancer relay 3

```bash
# Sur Freebox V6
nohup /root/tom-relay \
    --http-addr 0.0.0.0:3342 \
    --metrics-addr 0.0.0.0:9092 \
    > /tmp/relay-3.log 2>&1 &

echo $! > /tmp/relay-3.pid

# Vérifier
curl http://localhost:3342/health
```

**⚠️ Note** : Si la Freebox V6 a peu de RAM (256MB), surveiller la consommation mémoire :

```bash
# Vérifier utilisation RAM
free -m

# Surveiller process relay
top -b -n 1 | grep tom-relay
```

---

### Étape 3.8 : Test depuis autres machines

```bash
# Depuis Mac
curl http://192.168.0.Y:3342/health

# Depuis NAS
ssh root@192.168.0.83 'curl http://192.168.0.Y:3342/health'

# Attendu : {"status":"ok"}
```

**Checkpoint 3** :

- [ ] Relay 3 (Freebox V6) build OK via cross-compile (armv7)
- [ ] Binaire transféré vers Freebox V6
- [ ] Service lancé
- [ ] Health endpoint répond 200 OK
- [ ] Accessible depuis Mac et NAS

---

## Phase 3bis : Setup Relay 4 (Apple TV HD) — OPTIONNEL

**Durée** : 2-3h (wrapper tvOS) OU 1h (si jailbreak)
**Machine** : Apple TV HD (tvOS 16.3, A8 ARM64, toujours connecté)

**Modèle** : Apple TV HD (4ème génération)
**Chipset** : A8 (ARM64, 64-bit)
**tvOS** : 16.3

### Pourquoi Apple TV HD ?

✅ **Toujours actif** (24/7, contrairement au MacBook)
✅ **Fiable** (pas de reboot intempestif)
✅ **ARM64** (A8, plus rapide que Freebox V6 Cortex-A9)
✅ **Faible consommation** (~5W, comme Raspberry Pi)
✅ **Redondance** (4 relays = failover plus robuste)
✅ **Déjà disponible** (pas d'achat nécessaire)

### Option A : Wrapper tvOS (recommandé, 2-3h)

**Problème** : Apple TV n'a **pas de SSH natif** (sauf jailbreak).

**Solution** : Créer une **app tvOS** qui lance `tom-relay` en background.

---

#### Étape 3bis.1 : Cross-compile tom-relay pour tvOS (Apple TV HD)

**Note** : Apple TV HD (A8) est ARM64, mais tvOS impose des contraintes spécifiques.

```bash
# Sur Mac
cd ~/tom-protocol/crates/tom-relay

# Installer target tvOS ARM64
rustup target add aarch64-apple-tvos

# Build pour tvOS (Apple TV HD A8)
cargo build --release --target aarch64-apple-tvos

# Vérifier binaire
ls -lh target/aarch64-apple-tvos/release/tom-relay
```

**⚠️ Attention** : Build peut échouer si `aarch64-apple-tvos` n'est pas supporté par toutes les dépendances. Voir Option B (jailbreak) ou Option C (skip) si problème.

---

#### Étape 3bis.2 : Créer wrapper app tvOS

**Xcode project** : `apps/relay-tvos/`

```bash
# Créer nouveau projet Xcode
# File → New → Project → tvOS → App
# Nom : TomRelay
# Language : Swift
```

**Fichier** : `apps/relay-tvos/ContentView.swift`

```swift
import SwiftUI

struct ContentView: View {
    @State private var relayStatus = "Starting..."
    @State private var process: Process?

    var body: some View {
        VStack(spacing: 20) {
            Text("ToM Relay")
                .font(.largeTitle)

            Text(relayStatus)
                .foregroundColor(relayStatus.contains("Running") ? .green : .orange)

            Button("Start Relay") {
                startRelay()
            }

            Button("Stop Relay") {
                stopRelay()
            }
        }
        .onAppear {
            startRelay()
        }
    }

    func startRelay() {
        let relayPath = Bundle.main.path(forResource: "tom-relay", ofType: nil)!

        process = Process()
        process?.executableURL = URL(fileURLWithPath: relayPath)
        process?.arguments = [
            "--http-addr", "0.0.0.0:3343",
            "--metrics-addr", "0.0.0.0:9093"
        ]

        try? process?.run()
        relayStatus = "Running on :3343"
    }

    func stopRelay() {
        process?.terminate()
        relayStatus = "Stopped"
    }
}
```

---

#### Étape 3bis.3 : Embed binaire dans app

1. **Copier binaire** dans Xcode project :

   ```bash
   cp target/aarch64-apple-tvos/release/tom-relay \
      apps/relay-tvos/TomRelay/Resources/tom-relay
   ```

2. **Xcode** : Add Files → tom-relay → Target Membership : TomRelay

3. **Build Settings** :
   - Embedded Content Contains Swift Code : Yes
   - Enable Hardened Runtime : No (pour exec binaire)

---

#### Étape 3bis.4 : Déployer sur Apple TV

1. **Connecter Apple TV** :
   - Settings → Remotes and Devices → Remote App and Devices
   - Pair avec Mac

2. **Xcode** :
   - Product → Destination → Apple TV
   - Product → Run (Cmd+R)

3. **App démarre** sur Apple TV → Relay actif sur port 3343

---

#### Étape 3bis.5 : Trouver IP Apple TV

```bash
# Sur Mac, scanner réseau
arp -a | grep -i apple

# Ou dans Settings Apple TV
# Settings → Network → Show IP : 192.168.0.Z
```

---

#### Étape 3bis.6 : Test health endpoint

```bash
# Depuis Mac
curl http://192.168.0.Z:3343/health

# Attendu : {"status":"ok"}
```

---

### Option B : Jailbreak Apple TV (avancé, 1h)

**Si Apple TV est jailbreaké** → SSH possible → deploy comme sur Freebox Pop.

**Jailbreak tools** :

- checkra1n (A11 et antérieurs)
- unc0ver (certains tvOS)

**Workflow** :

1. Jailbreak Apple TV
2. SSH : `ssh root@192.168.0.Z` (password par défaut : alpine)
3. Installer Rust + build tom-relay
4. Lancer relay comme sur Freebox Pop

**⚠️ Risques** : Brick device, instabilité, pas recommandé pour prod.

---

### Option C : Skip Apple TV (utiliser seulement 3 relays)

**Si wrapper tvOS trop complexe** → utiliser seulement NAS + Mac + Freebox Pop (3 relays suffisent pour validation).

**Apple TV peut être ajouté plus tard** après validation 3 relays.

---

**Checkpoint 3bis** :

- [ ] tom-relay cross-compilé pour tvOS
- [ ] Wrapper app tvOS créée
- [ ] App déployée sur Apple TV via Xcode
- [ ] Relay actif sur port 3343
- [ ] Health endpoint répond 200 OK depuis Mac

---

## Phase 4 : Setup Discovery Service

**Durée** : 30min
**Machine** : NAS Freebox (recommandé) OU Mac

### Option A : Discovery sur NAS (avec relay 1)

**Étape 4.1 : Installer Node.js sur NAS**

```bash
# SSH NAS
ssh root@192.168.0.83

# Install Node.js 20.x
curl -fsSL https://deb.nodesource.com/setup_20.x | bash -
apt install -y nodejs

# Vérifier
node --version
npm --version
```

---

**Étape 4.2 : Setup discovery service**

```bash
# Sur NAS
cd /root/tom-protocol/tools/relay-discovery

# Install dependencies
npm install

# Build
npm run build

# Vérifier build
ls -la dist/
```

---

**Étape 4.3 : Créer config.json**

**IMPORTANT** : Remplacer `192.168.0.X` et `192.168.0.Y` par les IPs réelles.

```bash
# Sur NAS
cd /root/tom-protocol/tools/relay-discovery

cat > config.json <<'EOF'
{
  "relays": [
    {
      "url": "http://192.168.0.83:3340",
      "region": "nas-local",
      "health_endpoint": "http://192.168.0.83:3340/health"
    },
    {
      "url": "http://192.168.0.X:3341",
      "region": "mac-local",
      "health_endpoint": "http://192.168.0.X:3341/health"
    },
    {
      "url": "http://192.168.0.Y:3342",
      "region": "v6-local",
      "health_endpoint": "http://192.168.0.Y:3342/health"
    },
    {
      "url": "http://192.168.0.Z:3343",
      "region": "appletv-local",
      "health_endpoint": "http://192.168.0.Z:3343/health",
      "comment": "Optionnel - décommenter si relay 4 (Apple TV) déployé"
    }
  ],
  "ttl_seconds": 60,
  "health_check_interval_seconds": 10
}
EOF

# Vérifier config
cat config.json
```

---

**Étape 4.4 : Lancer discovery service**

```bash
# Sur NAS
cd /root/tom-protocol/tools/relay-discovery

# Lancer en background
nohup npm start > /tmp/discovery.log 2>&1 &

echo $! > /tmp/discovery.pid

# Vérifier
curl http://localhost:8080/relays | jq
```

---

### Option B : Discovery sur Mac

**Si NAS trop chargé ou problème Node.js** :

```bash
# Sur Mac
cd ~/tom-protocol/tools/relay-discovery

npm install
npm run build

# Créer config.json (même contenu que Option A)
nano config.json

# Lancer
npm start

# Vérifier
curl http://localhost:8080/relays | jq
```

---

### Étape 4.5 : Test discovery endpoints

```bash
# GET /relays
curl http://192.168.0.83:8080/relays | jq

# Attendu :
# {
#   "relays": [
#     {"url": "http://192.168.0.83:3340", "region": "nas-local", "load": 0.0, ...},
#     {"url": "http://192.168.0.X:3341", "region": "mac-local", ...},
#     {"url": "http://192.168.0.Y:3342", "region": "pop-local", ...}
#   ],
#   "ttl_seconds": 60
# }
```

```bash
# GET /health
curl http://192.168.0.83:8080/health

# Attendu : {"status":"ok"}
```

```bash
# GET /status
curl http://192.168.0.83:8080/status | jq

# Attendu : Snapshot avec cache, health checks, etc.
```

**Checkpoint 4** :
- [ ] Discovery service build OK
- [ ] Config.json contient 3 relays avec IPs correctes
- [ ] Service lancé (NAS ou Mac)
- [ ] GET /relays retourne 3 relays
- [ ] GET /health répond 200 OK

---

## Phase 5 : Implémenter Fallback List (CRITIQUE)

**Durée** : 30min
**Machine** : Mac (dev)

### Problème

Si discovery service est down, tous les clients sont bloqués (pas de relay découvert = pas de connexion).

### Solution

Fallback list hardcodée de 3 relays publics dans `tom-transport`.

---

### Étape 5.1 : Ajouter constante fallback

**Fichier** : `crates/tom-transport/src/config.rs`

**Ajouter après les imports** :

```rust
/// Fallback relay list (LAN relays) si discovery échoue
const DEFAULT_RELAY_URLS: &[&str] = &[
    "http://192.168.0.83:3340",   // NAS Freebox Delta
    "http://192.168.0.X:3341",    // MacBook Air (REMPLACER PAR IP RÉELLE)
    "http://192.168.0.Y:3342",    // Freebox V6 (REMPLACER PAR IP RÉELLE)
];
```

**IMPORTANT** : Remplacer `X` et `Y` par les IPs réelles du Mac et Freebox Pop.

---

### Étape 5.2 : Modifier build_endpoint_config()

**Fichier** : `crates/tom-transport/src/config.rs`

**Chercher la méthode `build_endpoint_config` et modifier** :

```rust
async fn build_endpoint_config(&self) -> Result<EndpointConfig> {
    let mut relay_urls = self.relay_urls.clone();

    // Fetch discovery si URL fournie
    if let Some(ref discovery_url) = self.relay_discovery_url {
        match fetch_relay_discovery(discovery_url).await {
            Ok(discovered) => {
                tracing::info!("Discovery fetched {} relays", discovered.len());
                relay_urls.extend(discovered);
            }
            Err(e) => {
                tracing::warn!("Discovery failed: {}, using fallback", e);
                // Fallback : ajouter relays publics si liste vide
                if relay_urls.is_empty() {
                    relay_urls.extend(
                        DEFAULT_RELAY_URLS.iter().map(|s| s.to_string())
                    );
                    tracing::info!("Using {} fallback relays", relay_urls.len());
                }
            }
        }
    }

    // Si toujours vide, utiliser fallback
    if relay_urls.is_empty() {
        relay_urls.extend(
            DEFAULT_RELAY_URLS.iter().map(|s| s.to_string())
        );
        tracing::info!("No relays configured, using {} fallback relays", relay_urls.len());
    }

    // Dédupliquer
    relay_urls.sort();
    relay_urls.dedup();

    // ... reste de la fonction (EndpointConfig, etc.)
}
```

---

### Étape 5.3 : Rebuild tom-transport

```bash
# Sur Mac
cd ~/tom-protocol
cargo build -p tom-transport --release

# Vérifier compilation OK
echo $?  # Doit retourner 0
```

---

### Étape 5.4 : Test fallback

```bash
# Tuer discovery service
pkill -f relay-discovery

# Lancer client (doit utiliser fallback automatiquement)
TOM_RELAY_DISCOVERY_URL=http://192.168.0.83:8080 \
    cargo run -p tom-stress -- ping --count 5

# Logs attendus :
# WARN Discovery failed: connection refused, using fallback
# INFO Using 3 fallback relays
# INFO Connected to relay nas-local (192.168.0.83:3340)
# ✅ Ping 5/5
```

```bash
# Relancer discovery
cd /root/tom-protocol/tools/relay-discovery
npm start &
```

**Checkpoint 5** :
- [ ] Fallback list ajoutée dans config.rs avec IPs correctes
- [ ] build_endpoint_config() modifiée
- [ ] Rebuild tom-transport OK
- [ ] Test fallback fonctionne (ping OK même sans discovery)

---

## Phase 6 : Tests de Validation

**Durée** : 2h
**Machine** : Mac (initiateur) + NAS/Freebox Pop (cibles)

### Test 1 : Multi-relay discovery

**Objectif** : Vérifier que le client découvre et utilise les 3 relays.

```bash
# Sur Mac
TOM_RELAY_DISCOVERY_URL=http://192.168.0.83:8080 \
    cargo run -p tom-stress -- ping --count 20

# Logs attendus :
# INFO Discovery fetched 3 relays
# INFO Connected to relay nas-local (192.168.0.83:3340)
# ✅ Ping 20/20 (100%)
```

**Critère succès** : Ping 20/20, latency <20ms (LAN).

**Validation** :
- [ ] Discovery fetch 3 relays
- [ ] Client connecté à 1 relay
- [ ] Ping 100% success
- [ ] Latency <20ms

---

### Test 2 : Failover automatique

**Objectif** : Vérifier que le client auto-failover si 1 relay meurt.

**Setup** :
- Campaign longue (2min) sur Mac
- Kill relay 1 (NAS) après 30s
- Observer auto-failover vers relay 2 ou 3

```bash
# Terminal 1 (Mac) : Lancer campaign
TOM_RELAY_DISCOVERY_URL=http://192.168.0.83:8080 \
    cargo run -p tom-stress -- campaign \
        --scenarios message \
        --duration 120s \
        --count 200

# Laisser tourner, observer logs
```

```bash
# Terminal 2 (après 30 secondes) : Kill relay NAS
ssh root@192.168.0.83 'systemctl stop tom-relay'

# Observer Terminal 1 :
# [30s]  ERROR Connection to relay nas-local lost
# [32s]  WARN  Trying relay mac-local
# [33s]  INFO  Connected to relay mac-local (192.168.0.X:3341)
# [120s] INFO  Campaign complete: 195/200 (97.5%)
```

```bash
# Terminal 2 : Restart relay NAS
ssh root@192.168.0.83 'systemctl start tom-relay'

# Attendre 10s, vérifier reconnexion
```

**Critère succès** :
- Campaign continue après kill relay (pas de crash)
- Auto-failover <30s
- Success rate >90% (perdre ~10% pendant failover OK)

**Validation** :
- [ ] Campaign continue après kill relay
- [ ] Auto-failover vers relay 2 ou 3
- [ ] Success rate >90%
- [ ] Relay 1 reconnecté après restart

---

### Test 3 : Campaign 1-to-1 cross-LAN

**Objectif** : Tester messaging entre 2 nodes via relays.

**Setup** :
- Machine A (Mac) : Responder
- Machine B (NAS) : Campaign initiator

```bash
# Terminal 1 (Mac) : Responder
TOM_RELAY_DISCOVERY_URL=http://192.168.0.83:8080 \
    cargo run -p tom-stress -- responder

# Copier le NODE_ID affiché
# Exemple : NODE_ID_MAC = "PeerId(abc123...)"
```

```bash
# Terminal 2 (NAS) : Campaign
ssh root@192.168.0.83

cd /root/tom-protocol
TOM_RELAY_DISCOVERY_URL=http://192.168.0.83:8080 \
    ./target/release/tom-stress campaign \
        --responder-addr <NODE_ID_MAC> \
        --scenarios ping,message,burst \
        --duration 60s

# Observer :
# ✅ ping: 50/50 (100%)
# ✅ message: 100/100 (100%)
# ✅ burst: 200/200 (100%)
# Campaign success: 100%
```

**Critère succès** :
- Ping : 100%
- Message : >99%
- Burst : >98%
- Latency <20ms (LAN)

**Validation** :
- [ ] Responder actif sur Mac
- [ ] Campaign lancé depuis NAS
- [ ] Tous scénarios >99% success
- [ ] Latency <20ms

---

### Test 4 : Discovery down (resilience)

**Objectif** : Vérifier fallback list fonctionne si discovery inaccessible.

```bash
# Tuer discovery
pkill -f relay-discovery

# Lancer client avec discovery URL invalide
TOM_RELAY_DISCOVERY_URL=http://192.168.0.83:8080 \
    cargo run -p tom-stress -- ping --count 10

# Logs attendus :
# WARN Discovery failed: connection refused
# INFO Using 3 fallback relays
# INFO Connected to relay nas-local (fallback)
# ✅ Ping 10/10
```

```bash
# Relancer discovery
cd /root/tom-protocol/tools/relay-discovery
npm start &
```

**Critère succès** : Ping fonctionne malgré discovery down (utilise fallback).

**Validation** :
- [ ] Discovery down (pas de process)
- [ ] Client utilise fallback list automatiquement
- [ ] Ping 100% success

---

### Test 5 : Charge LAN (optionnel, 30min)

**Objectif** : Vérifier stabilité sous charge.

**Setup** : 2 campaigns parallel pendant 10min

```bash
# Terminal 1 (Mac) : Campaign 1
TOM_RELAY_DISCOVERY_URL=http://192.168.0.83:8080 \
    cargo run -p tom-stress -- campaign \
        --scenarios burst \
        --duration 600s \
        --count 5000

# Terminal 2 (NAS) : Campaign 2
ssh root@192.168.0.83
TOM_RELAY_DISCOVERY_URL=http://192.168.0.83:8080 \
    ./target/release/tom-stress campaign \
        --scenarios burst \
        --duration 600s \
        --count 5000

# Observer pendant 10min :
# - Pas de crash
# - Latency stable <20ms
# - Success rate >99%
```

**Critère succès** : Les 2 campaigns terminent sans crash, >99% success.

**Validation** :
- [ ] 2 campaigns parallel lancées
- [ ] Durée 10min chacune
- [ ] Aucun crash relay ou client
- [ ] Success rate >99%

---

## Phase 7 : Monitoring Local (Grafana)

**Durée** : 1h
**Machine** : Mac

### Étape 7.1 : Installer Prometheus

```bash
# Sur Mac
brew install prometheus

# Créer config
cat > /opt/homebrew/etc/prometheus.yml <<EOF
global:
  scrape_interval: 10s

scrape_configs:
  - job_name: 'tom-relay'
    static_configs:
      - targets:
        - '192.168.0.83:9090'      # NAS relay-1
        - 'localhost:9091'          # Mac relay-2
        - '192.168.0.Y:9092'       # Freebox V6 relay-3 (REMPLACER Y)
        labels:
          region: 'local'
EOF

# Start Prometheus
brew services start prometheus

# Vérifier
open http://localhost:9090
```

**Dans Prometheus UI** :
- Aller dans Status → Targets
- Vérifier que 3 targets sont UP

---

### Étape 7.2 : Installer Grafana

```bash
# Sur Mac
brew install grafana

# Start Grafana
brew services start grafana

# Ouvrir
open http://localhost:3000

# Login : admin / admin (changer password au premier login)
```

---

### Étape 7.3 : Ajouter Prometheus data source

**Dans Grafana** :
1. Configuration (⚙️) → Data Sources
2. Add data source → Prometheus
3. URL : `http://localhost:9090`
4. Save & Test (doit être vert)

---

### Étape 7.4 : Import dashboard

**Option A : Via UI**
1. Dashboards (☰) → Import
2. Upload file : `~/tom-protocol/deploy/monitoring/grafana-dashboard-option-c.json`
3. Select Prometheus data source
4. Import

**Option B : Via CLI**
```bash
# Copier dashboard JSON vers Grafana provisioning
cp ~/tom-protocol/deploy/monitoring/grafana-dashboard-option-c.json \
   /opt/homebrew/var/lib/grafana/dashboards/

# Restart Grafana
brew services restart grafana
```

---

### Étape 7.5 : Observer dashboard

**Ouvrir Grafana** : http://localhost:3000

**Dashboard panels** :
- **Relays Up** : 3/3 (gauge)
- **Messages relayed/s** : Graphe temps réel
- **Latency p95** : <20ms
- **Drop rate** : <1%
- **Connections actives** : Stable

**Lancer stress test pour générer metrics** :
```bash
TOM_RELAY_DISCOVERY_URL=http://192.168.0.83:8080 \
    cargo run -p tom-stress -- campaign \
        --scenarios burst \
        --duration 60s

# Observer dashboard Grafana en temps réel
```

**Checkpoint 7** :
- [ ] Prometheus installé et scrape 3 targets
- [ ] Grafana installé et data source Prometheus OK
- [ ] Dashboard importé
- [ ] 3 relays visibles (UP)
- [ ] Metrics temps réel fonctionnent

---

## Checklist Validation Finale

### Infrastructure

- [ ] **Relay 1 (NAS Freebox Delta)** : Actif, health OK, metrics OK
- [ ] **Relay 2 (MacBook Air)** : Actif, health OK, metrics OK
- [ ] **Relay 3 (Freebox V6)** : Actif, health OK, metrics OK
- [ ] **Relay 4 (Apple TV HD - optionnel)** : Actif, health OK, metrics OK
- [ ] **Discovery service** : Actif, retourne 3-4 relays
- [ ] **Fallback list** : Implémentée avec IPs correctes

### Tests

- [ ] **Test 1** : Multi-relay discovery (20/20 ping)
- [ ] **Test 2** : Failover (>90% success après kill relay)
- [ ] **Test 3** : Campaign 1-to-1 (>99% success)
- [ ] **Test 4** : Discovery resilience (ping OK sans discovery)
- [ ] **Test 5** : Charge LAN (optionnel, >99% success)

### Monitoring

- [ ] **Prometheus** : Scrape 3 targets UP
- [ ] **Grafana** : Dashboard affiche 3 relays
- [ ] **Metrics temps réel** : Visible pendant stress test

### Documentation

- [ ] **IPs documentées** : Noter les IPs réelles dans ce fichier
- [ ] **Commandes start/stop** : Testées et documentées

---

## IPs Réelles (à compléter)

| Machine | IP LAN | Relay Port | Metrics Port |
|---------|--------|------------|--------------|
| NAS Freebox Delta | 192.168.0.83 | 3340 | 9090 |
| MacBook Air 2011 | 192.168.0.__ | 3341 | 9091 |
| Freebox V6 (beaux-parents) | 192.168.0.__ | 3342 | 9092 |
| Apple TV HD (optionnel) | 192.168.0.__ | 3343 | 9093 |
| Discovery | 192.168.0.83 (ou Mac) | 8080 | - |

**Action** : Compléter les `__` avec les IPs réelles.

---

## Troubleshooting

## Note terrain — Freebox des beaux-parents (accès distant)

Infos capturées (app Freebox, 2026-03-05) :

- **Nom box** : `Freebox Eric`
- **IP accès distant** : `88.190.129.5`
- **Port accès distant (HTTP)** : `40655`
- **HTTPS distant** : activé
- **Port accès distant sécurisé (HTTPS)** : `39202`
- **Nom de domaine Freebox** : `b981gn6o.fbxos...` (tronqué dans la capture)

### URL d'administration distante (Freebox OS)

- HTTP : `http://88.190.129.5:40655`
- HTTPS : `https://88.190.129.5:39202`

> ⚠️ Cette configuration correspond à l'**administration Freebox OS**. Ce n'est **pas** une preuve que les ports relay ToM (3342/9092) sont exposés depuis Internet.

### Impact pour ce guide

- Pour **Phase 3 (Relay 3 Freebox V6)** : l'accès distant Freebox facilite l'admin, mais il faut toujours vérifier les **redirections NAT** dédiées si on veut un test WAN réel.
- Pour un test WAN minimal relay 3, vérifier au besoin :
    - `UDP/TCP 3342` → machine relay 3
    - `TCP 9092` (optionnel, metrics)

### TODO opérateur

- Compléter ici le domaine Freebox exact (non tronqué).
- Capturer une preuve de redirection NAT (capture Freebox OS ou export config).

### Problème : Relay ne démarre pas

**Symptômes** :
```bash
systemctl status tom-relay
# Active: failed
```

**Solutions** :
```bash
# Voir logs détaillés
journalctl -u tom-relay -n 50

# Vérifier port disponible
netstat -tuln | grep 3340

# Si port occupé, tuer process
lsof -ti:3340 | xargs kill -9
```

---

### Problème : Discovery ne trouve pas les relays

**Symptômes** :
```bash
curl http://192.168.0.83:8080/relays
# {"relays": [], ...}
```

**Solutions** :
```bash
# Vérifier config.json
cat /root/tom-protocol/tools/relay-discovery/config.json

# Vérifier que relays répondent
curl http://192.168.0.83:3340/health
curl http://192.168.0.X:3341/health
curl http://192.168.0.Y:3342/health

# Voir logs discovery
tail -f /tmp/discovery.log
```

---

### Problème : Client ne se connecte à aucun relay

**Symptômes** :
```bash
cargo run -p tom-stress -- ping
# Error: No relays available
```

**Solutions** :
```bash
# Vérifier discovery
curl http://192.168.0.83:8080/relays

# Vérifier fallback list dans code
grep DEFAULT_RELAY_URLS crates/tom-transport/src/config.rs

# Tester avec relay URL direct
TOM_RELAY_URL=http://192.168.0.83:3340 \
    cargo run -p tom-stress -- ping
```

---

### Problème : Grafana ne voit pas les metrics

**Symptômes** : Dashboard vide ou "No data".

**Solutions** :
```bash
# Vérifier Prometheus scrape targets
open http://localhost:9090/targets

# Si target DOWN, vérifier firewall
curl http://192.168.0.83:9090/metrics

# Si metrics vides, relancer relay
ssh root@192.168.0.83 'systemctl restart tom-relay'
```

---

## Prochaines Étapes (après validation locale)

### Option 1 : Rester 100% local (gratuit)

**Avantages** :
- Gratuit
- Contrôle total
- Debug facile

**Inconvénients** :
- Pas accessible publiquement
- Pas de test cross-région réel (WAN)

**Usage** : Dev/test uniquement.

---

### Option 2 : Hybrid (1 relay externe)

**Setup** :
- 2 relays locaux (NAS + Mac)
- 1 relay externe (VPS Hetzner EU, 4€/mois)

**Test** : Campaign Mac (LAN) ↔ VPS (WAN)

**Validation** : Si latency <100ms et success >95% → OK pour scale.

**Coût** : 4€/mois.

---

### Option 3 : Full prod (3 VPS)

**Setup** :
- 3 VPS TLS (EU, US, ASIA)
- Discovery sur Vercel (gratuit)
- Monitoring Grafana Cloud

**Coût** : ~12€/mois pour infrastructure globale.

**Usage** : Production publique.

---

## Commandes Utiles

### Start/Stop Relays

```bash
# Relay 1 (NAS)
ssh root@192.168.0.83 'systemctl start tom-relay'
ssh root@192.168.0.83 'systemctl stop tom-relay'
ssh root@192.168.0.83 'systemctl status tom-relay'

# Relay 2 (Mac)
launchctl load ~/Library/LaunchAgents/com.tom.relay.plist
launchctl unload ~/Library/LaunchAgents/com.tom.relay.plist
# ou
kill $(cat /tmp/relay-2.pid)

# Relay 3 (Freebox V6)
ssh root@192.168.0.Y 'kill $(cat /tmp/relay-3.pid)'
```

---

### Start/Stop Discovery

```bash
# Sur NAS
ssh root@192.168.0.83 'kill $(cat /tmp/discovery.pid)'
ssh root@192.168.0.83 'cd /root/tom-protocol/tools/relay-discovery && npm start &'

# Sur Mac
pkill -f relay-discovery
cd ~/tom-protocol/tools/relay-discovery && npm start
```

---

### Monitoring

```bash
# Prometheus
brew services start prometheus
brew services stop prometheus
open http://localhost:9090

# Grafana
brew services start grafana
brew services stop grafana
open http://localhost:3000
```

---

### Quick Tests

```bash
# Ping via discovery
TOM_RELAY_DISCOVERY_URL=http://192.168.0.83:8080 \
    cargo run -p tom-stress -- ping --count 10

# Ping via relay direct
TOM_RELAY_URL=http://192.168.0.83:3340 \
    cargo run -p tom-stress -- ping --count 10

# Check all relays health
for port in 3340 3341 3342; do
    curl -s http://192.168.0.83:$port/health || echo "Port $port DOWN"
done
```

---

## Résumé Exécutif

**Durée totale** : 6-10h sur 3 jours (3 relays) ou 8-12h (4 relays avec Apple TV HD)

**Jour 1** (2-3h) :
- Setup relay 1 (NAS) : 30min
- Setup relay 2 (Mac) : 45min
- Setup relay 3 (Freebox V6, cross-compile) : 30min-1h
- Setup relay 4 (Apple TV HD, optionnel) : +2h

**Jour 2** (2h) :
- Setup discovery service : 30min
- Implémenter fallback list : 30min
- Tests validation : 1h

**Jour 3** (2h) :
- Tests complets : 1h
- Monitoring Grafana : 1h

**Résultat** : Infrastructure 3 relays locale validée, prête pour migration prod.

---

## Contact & Support

**Questions** : Partager ce fichier avec GPT 5.3 Codex pour assistance.

**Logs** :

- Relay 1 : `journalctl -u tom-relay -f` (NAS Freebox Delta)
- Relay 2 : `tail -f /tmp/relay-2.log` (MacBook Air)
- Relay 3 : `tail -f /tmp/relay-3.log` (Freebox V6)
- Relay 4 : Voir logs tvOS app (Apple TV HD, si déployé)
- Discovery : `tail -f /tmp/discovery.log`

**Debugging** : RUST_LOG=debug pour logs verbeux.

---

**Fin du guide. Bon déploiement ! 🚀**
