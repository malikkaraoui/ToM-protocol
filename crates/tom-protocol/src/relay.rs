/// Relay selection for ToM protocol.
///
/// Chooses the best relay node based on network topology: role,
/// online status, and last-seen timestamp.
use serde::{Deserialize, Serialize};
use std::{cmp::Reverse, collections::HashMap};
use crate::types::NodeId;

/// Maximum relay depth for path selection.
pub const MAX_RELAY_DEPTH: usize = 4;

/// Maximum tracked peers in topology (memory exhaustion protection, R11.2).
pub const MAX_PEERS: usize = 10_000;

/// TTL d'oubli d'une identité de topologie (anti-ravivage M2/M3, doc validée
/// Malik 18/07 : **24 h strict** — « rien ne survit 24 h », cohérent avec la
/// décision #2, messages traités comme métadonnées). Un pair non-`Online` dont
/// `now - last_seen > TOPOLOGY_TTL_MS` est écarté à la sauvegarde ET au chargement
/// de la base (M2), et évincé de la mémoire au tick 60 s (M3). Un appareil absent
/// > 24 h revient par re-découverte (rendez-vous ADR-010 ~60 s, mDNS en LAN).
pub const TOPOLOGY_TTL_MS: u64 = 24 * 60 * 60 * 1_000;

/// Staleness threshold for Online peers (presence/L1-003 hardening, Faille 1).
///
/// A peer promoted to `Online` via quorum L1-003 (or any other means) remains
/// `Online` only while `now - last_seen < PEER_ONLINE_STALE_MS`. If a peer goes
/// silent (no activity, no fresh attestation) for this duration, it is degraded
/// back to `Known` (address known via discovery, but liveness no longer proven).
/// This prevents Sybil attacks where a peer is promoted via temporary quorum,
/// then goes silent — the victim would incorrectly believe the peer is still
/// alive indefinitely without this mechanism.
///
/// Chosen as 2× PRESENCE_TTL_MS (60s): allows up to 2 tick cycles (30s each)
/// before degradation, providing margin against legitimate jitter while degrasing
/// false positives within a reasonable window. Faster than the ~105s sag from
/// the original heartbeat-only liveness loop (runtime/loop.rs).
pub const PEER_ONLINE_STALE_MS: u64 = 60_000;

// ── Peer topology info ─────────────────────────────────────────────────

/// Role a node plays in the network (assigned dynamically).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PeerRole {
    /// Regular participant — sends/receives messages.
    Peer,
    /// Relay-capable — can forward messages for others.
    Relay,
}

/// Current status of a known peer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PeerStatus {
    Online,
    Offline,
    /// Recently seen but may be transitioning.
    Stale,
    /// Discovered (address known via DHT/gossip/neighbor-up) but no real
    /// work has been observed yet — addressable, not proven live (ADR-011
    /// PoP, two-tier Known/Online). Appended last to keep the discriminants
    /// of the pre-existing variants stable for persisted state.
    Known,
}

/// Information about a known peer in the network topology.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub node_id: NodeId,
    pub role: PeerRole,
    pub status: PeerStatus,
    /// Unix ms timestamp of last observed activity.
    pub last_seen: u64,
}

// ── Network topology ───────────────────────────────────────────────────

/// Snapshot of known network topology — peers and their roles/status.
///
/// Updated by the discovery layer (gossip). The RelaySelector reads
/// this to make routing decisions.
#[derive(Debug, Default)]
pub struct Topology {
    peers: HashMap<NodeId, PeerInfo>,
}

impl Topology {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add or update a peer. Returns false if at capacity and no room could be
    /// made for a new peer.
    ///
    /// When full, evict the stalest NON-online peer (oldest `last_seen`) to make
    /// room, rather than rejecting the newcomer outright (red-team FINDING #10):
    /// a swarm of signed announces from fresh keypairs could otherwise fill the
    /// table with soon-dead entries and permanently starve discovery of real,
    /// live peers. Genuinely Online peers are never evicted — if every slot is
    /// Online, the table is legitimately full and the new peer is rejected.
    pub fn upsert(&mut self, info: PeerInfo) -> bool {
        if self.peers.len() >= MAX_PEERS && !self.peers.contains_key(&info.node_id) {
            let victim = self
                .peers
                .values()
                .filter(|p| p.status != PeerStatus::Online)
                .min_by_key(|p| p.last_seen)
                .map(|p| p.node_id);
            match victim {
                Some(id) => {
                    self.peers.remove(&id);
                }
                None => return false,
            }
        }
        self.peers.insert(info.node_id, info);
        true
    }

    /// Remove a peer.
    pub fn remove(&mut self, node_id: &NodeId) {
        self.peers.remove(node_id);
    }

    /// Anti-ravivage M3 — éviction proactive douce : retire les pairs NON-`Online`
    /// dont `now - last_seen > TOPOLOGY_TTL_MS` (identités jamais revues depuis
    /// 24 h). On ne touche JAMAIS un pair `Online` (vivant, prouvé) — décision #4
    /// (fade, pas de ban) : ce n'est pas une exclusion, juste de l'oubli d'adresses
    /// mortes ; un pair re-vu est re-découvert par les canaux normaux. Retourne le
    /// nombre de pairs évincés (observabilité). `MAX_PEERS` reste le garde-fou dur.
    pub fn evict_stale(&mut self, now: u64) -> usize {
        let before = self.peers.len();
        self.peers.retain(|_, p| {
            p.status == PeerStatus::Online || now.saturating_sub(p.last_seen) <= TOPOLOGY_TTL_MS
        });
        before - self.peers.len()
    }

    /// Get info for a specific peer.
    pub fn get(&self, node_id: &NodeId) -> Option<&PeerInfo> {
        self.peers.get(node_id)
    }

    /// Get mutable info for a specific peer (used by RoleManager to update roles).
    pub fn get_mut(&mut self, node_id: &NodeId) -> Option<&mut PeerInfo> {
        self.peers.get_mut(node_id)
    }

    /// All known peers.
    pub fn peers(&self) -> impl Iterator<Item = &PeerInfo> {
        self.peers.values()
    }

    /// Access the raw peer map (for persistence).
    pub fn peers_map(&self) -> &HashMap<NodeId, PeerInfo> {
        &self.peers
    }

    /// Number of known peers.
    pub fn len(&self) -> usize {
        self.peers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.peers.is_empty()
    }

    /// Number of online peers.
    pub fn online_count(&self) -> usize {
        self.peers.values().filter(|p| p.status == PeerStatus::Online).count()
    }

    /// Number of peers with Relay role.
    pub fn relay_count(&self) -> usize {
        self.peers.values().filter(|p| p.role == PeerRole::Relay).count()
    }

    /// Local node's role (defaults to Peer if not in topology).
    pub fn local_role(&self, node_id: &NodeId) -> PeerRole {
        self.peers.get(node_id).map_or(PeerRole::Peer, |p| p.role)
    }

    /// All online relay-capable peers, sorted by most recently seen.
    pub fn online_relays(&self) -> Vec<&PeerInfo> {
        let mut relays: Vec<&PeerInfo> = self
            .peers
            .values()
            .filter(|p| p.role == PeerRole::Relay && p.status == PeerStatus::Online)
            .collect();
        relays.sort_by_key(|a| Reverse(a.last_seen));
        relays
    }

    /// Relay-capable peers usable as a first-hop candidate: proven-live
    /// (Online) OR merely discovered (Known, ADR-011 two-tier). Used only
    /// for relay-*path* selection (cold-start needs an address to dial
    /// before any real work can happen) — never for liveness reporting
    /// (`online_count`) or hub election (`online_relays`), which stay
    /// strictly Online.
    pub fn reachable_relays(&self) -> Vec<&PeerInfo> {
        let mut relays: Vec<&PeerInfo> = self
            .peers
            .values()
            .filter(|p| {
                p.role == PeerRole::Relay
                    && matches!(p.status, PeerStatus::Online | PeerStatus::Known)
            })
            .collect();
        relays.sort_by_key(|a| Reverse(a.last_seen));
        relays
    }
}

// ── Relay selection ────────────────────────────────────────────────────

/// Why a particular relay was selected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectionReason {
    /// Most recently seen online relay.
    MostRecent,
    /// Only available relay.
    OnlyOption,
    /// Alternate after primary failed.
    Alternate,
    /// No relay available.
    NoRelayAvailable,
}

/// Result of relay selection.
#[derive(Debug)]
pub struct RelaySelection {
    pub relay_id: Option<NodeId>,
    pub reason: SelectionReason,
}

/// Selects the best relay for message routing.
///
/// Pure logic — reads topology, returns a selection. No I/O.
pub struct RelaySelector {
    self_id: NodeId,
}

impl RelaySelector {
    pub fn new(self_id: NodeId) -> Self {
        Self { self_id }
    }

    /// Select the best relay to reach `target`.
    ///
    /// Filters: must be a relay, must be online, must not be self or target.
    /// Prefers the most recently seen relay.
    pub fn select_best(
        &self,
        target: NodeId,
        topology: &Topology,
    ) -> RelaySelection {
        self.select_best_excluding(target, topology, &[])
    }

    /// Select the best relay, excluding specific nodes (e.g., failed relays).
    pub fn select_best_excluding(
        &self,
        target: NodeId,
        topology: &Topology,
        exclude: &[NodeId],
    ) -> RelaySelection {
        let candidates: Vec<&PeerInfo> = topology
            .reachable_relays()
            .into_iter()
            .filter(|p| {
                p.node_id != self.self_id
                    && p.node_id != target
                    && !exclude.contains(&p.node_id)
            })
            .collect();

        match candidates.len() {
            0 => RelaySelection {
                relay_id: None,
                reason: SelectionReason::NoRelayAvailable,
            },
            1 => RelaySelection {
                relay_id: Some(candidates[0].node_id),
                reason: SelectionReason::OnlyOption,
            },
            _ => RelaySelection {
                relay_id: Some(candidates[0].node_id), // Already sorted by last_seen desc
                reason: SelectionReason::MostRecent,
            },
        }
    }

    /// Select an alternate relay after a failure.
    pub fn select_alternate(
        &self,
        target: NodeId,
        topology: &Topology,
        failed: &[NodeId],
    ) -> RelaySelection {
        let selection = self.select_best_excluding(target, topology, failed);
        if selection.relay_id.is_some() {
            RelaySelection {
                relay_id: selection.relay_id,
                reason: SelectionReason::Alternate,
            }
        } else {
            selection
        }
    }

    /// Build a multi-hop relay path to reach `target` (BFS through relay nodes).
    ///
    /// Returns a `via` chain of relay NodeIds, capped at `MAX_RELAY_DEPTH`.
    /// For the PoC, returns a single-hop path (one relay).
    pub fn select_path(
        &self,
        target: NodeId,
        topology: &Topology,
    ) -> Vec<NodeId> {
        // PoC: single-hop relay selection
        match self.select_best(target, topology).relay_id {
            Some(relay) => vec![relay],
            None => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node_id(seed: u8) -> NodeId {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
        let secret = tom_connect::SecretKey::generate(&mut rng);
        secret.public().to_string().parse().unwrap()
    }

    fn make_relay(seed: u8, last_seen: u64) -> PeerInfo {
        PeerInfo {
            node_id: node_id(seed),
            role: PeerRole::Relay,
            status: PeerStatus::Online,
            last_seen,
        }
    }

    fn make_peer(seed: u8) -> PeerInfo {
        PeerInfo {
            node_id: node_id(seed),
            role: PeerRole::Peer,
            status: PeerStatus::Online,
            last_seen: 1708000000000,
        }
    }

    // ── Topology tests ─────────────────────────────────────────────────

    #[test]
    fn topology_upsert_and_get() {
        let mut topo = Topology::new();
        let info = make_relay(1, 1000);
        let id = info.node_id;

        topo.upsert(info);
        assert_eq!(topo.len(), 1);
        assert!(topo.get(&id).is_some());
    }

    #[test]
    fn topology_remove() {
        let mut topo = Topology::new();
        let info = make_relay(1, 1000);
        let id = info.node_id;

        topo.upsert(info);
        topo.remove(&id);
        assert!(topo.is_empty());
    }

    /// Anti-ravivage M3 : `evict_stale` retire les fantômes non-Online > 24 h,
    /// garde les Online (même très vieux) et les non-Online récents.
    #[test]
    fn topology_evict_stale_removes_only_old_non_online() {
        let mut topo = Topology::new();
        let now = 1_000_000_000_000u64;

        // Fantôme : Offline, vu il y a 25 h → évincé.
        topo.upsert(PeerInfo {
            node_id: node_id(1),
            role: PeerRole::Peer,
            status: PeerStatus::Offline,
            last_seen: now - 25 * 60 * 60 * 1000,
        });
        // Online très vieux → GARDÉ (vivant, jamais évincé, décision #4).
        topo.upsert(PeerInfo {
            node_id: node_id(2),
            role: PeerRole::Relay,
            status: PeerStatus::Online,
            last_seen: now - 48 * 60 * 60 * 1000,
        });
        // Offline récent (1 h) → GARDÉ (sous le TTL).
        topo.upsert(PeerInfo {
            node_id: node_id(3),
            role: PeerRole::Peer,
            status: PeerStatus::Offline,
            last_seen: now - 60 * 60 * 1000,
        });

        let evicted = topo.evict_stale(now);
        assert_eq!(evicted, 1, "seul le fantôme > 24 h non-Online est évincé");
        assert!(topo.get(&node_id(1)).is_none(), "fantôme évincé");
        assert!(topo.get(&node_id(2)).is_some(), "Online gardé même très vieux");
        assert!(topo.get(&node_id(3)).is_some(), "Offline récent gardé");
    }

    /// Anti-ravivage : horloge qui recule → saturating_sub, aucun évincement à tort.
    #[test]
    fn topology_evict_stale_clock_rewind_safe() {
        let mut topo = Topology::new();
        topo.upsert(PeerInfo {
            node_id: node_id(1),
            role: PeerRole::Peer,
            status: PeerStatus::Offline,
            last_seen: 5_000_000,
        });
        // now < last_seen : saturating_sub = 0 → jamais > TTL → gardé.
        assert_eq!(topo.evict_stale(1_000_000), 0);
        assert!(topo.get(&node_id(1)).is_some());
    }

    // Distinct NodeId from a u64 seed (u8 helper only spans 256 ids; the
    // capacity tests below need MAX_PEERS distinct identities).
    fn node_id_u64(seed: u64) -> NodeId {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        tom_connect::SecretKey::generate(&mut rng)
            .public()
            .to_string()
            .parse()
            .unwrap()
    }

    fn peer_at(seed: u64, status: PeerStatus, last_seen: u64) -> PeerInfo {
        PeerInfo {
            node_id: node_id_u64(seed),
            role: PeerRole::Peer,
            status,
            last_seen,
        }
    }

    #[test]
    fn topology_full_evicts_stalest_offline_admits_newcomer() {
        // FINDING #10: a full table must still admit a new (live) peer by
        // evicting the stalest NON-online entry, so an announce swarm can't
        // permanently starve discovery of real peers.
        let mut topo = Topology::new();
        for i in 0..(MAX_PEERS - 1) as u64 {
            topo.upsert(peer_at(i, PeerStatus::Online, 5000));
        }
        // The one evictable slot: an old Offline peer.
        let victim = node_id_u64(999_999);
        topo.upsert(PeerInfo {
            node_id: victim,
            role: PeerRole::Peer,
            status: PeerStatus::Offline,
            last_seen: 1,
        });
        assert_eq!(topo.len(), MAX_PEERS);

        let newcomer = node_id_u64(1_000_000);
        assert!(topo.upsert(PeerInfo {
            node_id: newcomer,
            role: PeerRole::Peer,
            status: PeerStatus::Online,
            last_seen: 9999,
        }));
        assert!(topo.get(&newcomer).is_some(), "live newcomer must be admitted");
        assert!(topo.get(&victim).is_none(), "stalest offline peer must be evicted");
        assert_eq!(topo.len(), MAX_PEERS);
    }

    #[test]
    fn topology_full_of_online_rejects_new() {
        // If every slot is a genuinely Online peer, the table is legitimately
        // full: no live peer may be evicted, so the newcomer is rejected.
        let mut topo = Topology::new();
        for i in 0..MAX_PEERS as u64 {
            topo.upsert(peer_at(i, PeerStatus::Online, 5000));
        }
        let newcomer = node_id_u64(2_000_000);
        assert!(!topo.upsert(peer_at(2_000_000, PeerStatus::Online, 9999)));
        assert!(topo.get(&newcomer).is_none());
        assert_eq!(topo.len(), MAX_PEERS);
    }

    #[test]
    fn online_relays_sorted_by_last_seen() {
        let mut topo = Topology::new();
        topo.upsert(make_relay(1, 1000)); // oldest
        topo.upsert(make_relay(2, 3000)); // newest
        topo.upsert(make_relay(3, 2000)); // middle

        let relays = topo.online_relays();
        assert_eq!(relays.len(), 3);
        assert_eq!(relays[0].last_seen, 3000);
        assert_eq!(relays[1].last_seen, 2000);
        assert_eq!(relays[2].last_seen, 1000);
    }

    #[test]
    fn online_relays_excludes_peers_and_offline() {
        let mut topo = Topology::new();
        topo.upsert(make_relay(1, 1000)); // online relay
        topo.upsert(make_peer(2));         // peer, not relay
        topo.upsert(PeerInfo {
            node_id: node_id(3),
            role: PeerRole::Relay,
            status: PeerStatus::Offline,
            last_seen: 5000,
        }); // offline relay

        let relays = topo.online_relays();
        assert_eq!(relays.len(), 1);
        assert_eq!(relays[0].node_id, node_id(1));
    }

    // ── Selection tests ────────────────────────────────────────────────

    #[test]
    fn select_best_picks_most_recent() {
        let me = node_id(100);
        let target = node_id(200);
        let selector = RelaySelector::new(me);

        let mut topo = Topology::new();
        topo.upsert(make_relay(1, 1000));
        topo.upsert(make_relay(2, 3000)); // most recent
        topo.upsert(make_relay(3, 2000));

        let result = selector.select_best(target, &topo);
        assert_eq!(result.relay_id, Some(node_id(2)));
        assert_eq!(result.reason, SelectionReason::MostRecent);
    }

    #[test]
    fn select_best_excludes_self_and_target() {
        let me = node_id(1);
        let target = node_id(2);
        let selector = RelaySelector::new(me);

        let mut topo = Topology::new();
        topo.upsert(make_relay(1, 5000)); // self — excluded
        topo.upsert(make_relay(2, 4000)); // target — excluded
        topo.upsert(make_relay(3, 3000)); // valid

        let result = selector.select_best(target, &topo);
        assert_eq!(result.relay_id, Some(node_id(3)));
    }

    #[test]
    fn select_best_no_relays_available() {
        let me = node_id(100);
        let target = node_id(200);
        let selector = RelaySelector::new(me);
        let topo = Topology::new();

        let result = selector.select_best(target, &topo);
        assert_eq!(result.relay_id, None);
        assert_eq!(result.reason, SelectionReason::NoRelayAvailable);
    }

    #[test]
    fn select_best_only_option() {
        let me = node_id(100);
        let target = node_id(200);
        let selector = RelaySelector::new(me);

        let mut topo = Topology::new();
        topo.upsert(make_relay(1, 1000));

        let result = selector.select_best(target, &topo);
        assert_eq!(result.relay_id, Some(node_id(1)));
        assert_eq!(result.reason, SelectionReason::OnlyOption);
    }

    #[test]
    fn select_best_excluding_failed() {
        let me = node_id(100);
        let target = node_id(200);
        let selector = RelaySelector::new(me);

        let mut topo = Topology::new();
        topo.upsert(make_relay(1, 3000)); // best but failed
        topo.upsert(make_relay(2, 2000)); // fallback

        let failed = vec![node_id(1)];
        let result = selector.select_best_excluding(target, &topo, &failed);
        assert_eq!(result.relay_id, Some(node_id(2)));
    }

    #[test]
    fn select_alternate() {
        let me = node_id(100);
        let target = node_id(200);
        let selector = RelaySelector::new(me);

        let mut topo = Topology::new();
        topo.upsert(make_relay(1, 3000));
        topo.upsert(make_relay(2, 2000));

        let result = selector.select_alternate(target, &topo, &[node_id(1)]);
        assert_eq!(result.relay_id, Some(node_id(2)));
        assert_eq!(result.reason, SelectionReason::Alternate);
    }

    #[test]
    fn select_path_single_hop() {
        let me = node_id(100);
        let target = node_id(200);
        let selector = RelaySelector::new(me);

        let mut topo = Topology::new();
        topo.upsert(make_relay(1, 3000));

        let path = selector.select_path(target, &topo);
        assert_eq!(path, vec![node_id(1)]);
    }

    #[test]
    fn select_path_no_relay() {
        let me = node_id(100);
        let target = node_id(200);
        let selector = RelaySelector::new(me);
        let topo = Topology::new();

        let path = selector.select_path(target, &topo);
        assert!(path.is_empty());
    }

    #[test]
    fn topology_max_peers_cap() {
        use rand::SeedableRng;
        let mut topo = Topology::new();
        // Fill to MAX_PEERS
        for i in 0..MAX_PEERS {
            let mut rng = rand::rngs::StdRng::seed_from_u64(i as u64 + 1000);
            let secret = tom_connect::SecretKey::generate(&mut rng);
            let id: NodeId = secret.public().to_string().parse().unwrap();
            assert!(topo.upsert(PeerInfo {
                node_id: id,
                role: PeerRole::Peer,
                status: PeerStatus::Online,
                last_seen: 1000,
            }));
        }
        assert_eq!(topo.len(), MAX_PEERS);

        // New peer should be rejected
        let overflow_id = node_id(255);
        assert!(
            !topo.upsert(PeerInfo {
                node_id: overflow_id,
                role: PeerRole::Peer,
                status: PeerStatus::Online,
                last_seen: 2000,
            }),
            "should reject new peer at capacity"
        );
        assert_eq!(topo.len(), MAX_PEERS);

        // Updating existing peer should still work
        let existing_id = topo.peers().next().unwrap().node_id;
        assert!(topo.upsert(PeerInfo {
            node_id: existing_id,
            role: PeerRole::Relay,
            status: PeerStatus::Online,
            last_seen: 5000,
        }));
        assert_eq!(topo.get(&existing_id).unwrap().role, PeerRole::Relay);
    }

    #[test]
    fn topology_upsert_updates_existing() {
        let mut topo = Topology::new();
        let id = node_id(1);

        topo.upsert(PeerInfo {
            node_id: id,
            role: PeerRole::Relay,
            status: PeerStatus::Online,
            last_seen: 1000,
        });

        topo.upsert(PeerInfo {
            node_id: id,
            role: PeerRole::Relay,
            status: PeerStatus::Offline,
            last_seen: 2000,
        });

        assert_eq!(topo.len(), 1);
        assert_eq!(topo.get(&id).unwrap().status, PeerStatus::Offline);
        assert_eq!(topo.get(&id).unwrap().last_seen, 2000);
    }
}
