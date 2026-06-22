# Bug Report — Freebox : ARP WiFi ↔ VM cassé après reboot

**URL de soumission :** https://dev.freebox.fr/bugs/
**Catégorie suggérée :** LAN (ou WiFi)
**Modèle :** Freebox DELTA

---

## Titre

**Isolation ARP entre clients WiFi et VM Freebox après reboot — les clients WiFi ne peuvent plus joindre les VM en IPv4**

## Description

Après un reboot de la Freebox, les clients connectés en WiFi ne peuvent plus communiquer en IPv4 avec les machines virtuelles (VM) hébergées sur la Freebox. Le problème persiste indéfiniment jusqu'à intervention manuelle.

## Environnement

- **Freebox :** Pop V8 (adapter selon votre modèle), firmware à jour
- **Client WiFi :** MacBook Pro, macOS Sequoia 15.3, WiFi 5 GHz, adresse privée **désactivée** (MAC réelle utilisée)
- **VM :** Debian Buster (ARM64), interface `eth0`, IP statique 192.168.0.83/24
- **Réseau :** DHCP Freebox actif, plage 192.168.0.1-254, pas de filtrage MAC

## Symptômes

1. **Avant reboot :** tout fonctionne normalement (ping, SSH, HTTP entre Mac et VM)
2. **Après reboot Freebox :**
   - `ping 192.168.0.83` depuis le Mac WiFi → **100% packet loss**
   - `arp -a` sur le Mac → l'entrée pour 192.168.0.83 reste **(incomplete)**
   - Le Mac ne peut résoudre l'ARP d'**aucun** appareil LAN sauf la gateway (.254)
   - La VM peut ping le Mac (unidirectionnel : VM → Mac OK, Mac → VM KO)

3. **IPv6 fonctionne parfaitement :**
   - `ping6 fe80::248f:5dff:fea5:8ed1%en0` → 0% loss, 10ms
   - SSH via IPv6 link-local → fonctionne

4. **Depuis un autre réseau (WAN) :** SSH via redirection NAT port 2222→22 fonctionne

## Diagnostic réalisé

| Test | Résultat |
|------|----------|
| `ping 192.168.0.83` (Mac WiFi → VM) | KO — 100% loss |
| `ping 192.168.0.70` (VM → Mac WiFi) | OK — 0% loss |
| `ping6 fe80::...` (Mac WiFi → VM) | OK — 0% loss, 10ms |
| `arp -a` sur Mac | 192.168.0.83 → (incomplete) |
| `arp -d 192.168.0.83` + retry | Toujours (incomplete) |
| SSH WAN (82.67.95.8:2222) | OK |
| Mac reboot | Aucun effet |
| Freebox reboot (2e fois) | Aucun effet |
| Toggle WiFi Mac | Aucun effet |
| `sudo route flush` Mac | Aucun effet |
| Désactivation adresse WiFi privée Mac | Aucun effet |
| `tcpdump` côté VM | ARP requests du Mac arrivent, ARP replies envoyées mais n'atteignent pas le Mac |

## Analyse

Le bridge WiFi ↔ VM de la Freebox ne transmet plus les trames ARP reply de la VM vers les clients WiFi. Les ARP requests du Mac arrivent bien à la VM (visible dans tcpdump), la VM répond, mais la réponse ne traverse pas le bridge retour.

**Le problème est au niveau Layer 2 (ARP/bridge) dans la Freebox, pas côté client ni côté VM.**

Arguments :
- IPv6 neighbor discovery fonctionne (utilise ICMPv6, pas ARP)
- La communication est unidirectionnelle (VM→Mac OK, pas l'inverse)
- Aucune manipulation côté Mac ou VM ne résout le problème
- Le problème n'existait pas avant le reboot Freebox

## Impact

- Impossible d'utiliser les VM Freebox en IPv4 depuis le WiFi après un reboot
- Contournement possible via IPv6 link-local ou accès WAN
- Affecte tout développement/test utilisant les VM Freebox en réseau local

## Étapes de reproduction

1. Créer une VM Debian sur la Freebox avec IP statique (ex: 192.168.0.83)
2. Vérifier que ping/SSH fonctionne depuis un client WiFi → OK
3. Rebooter la Freebox
4. Tenter `ping <IP_VM>` depuis le client WiFi → **KO, ARP incomplete**
5. Vérifier que `ping6 <IPv6_link_local_VM>%en0` fonctionne → OK

## Résultat attendu

Après un reboot Freebox, les clients WiFi doivent pouvoir résoudre l'ARP des VM et communiquer en IPv4 normalement.

## Contournement actuel

- Utiliser SSH via IPv6 link-local : `ssh root@fe80::248f:5dff:fea5:8ed1%en0`
- Utiliser SSH via WAN : `ssh -p 2222 root@<IP_PUBLIQUE>`
