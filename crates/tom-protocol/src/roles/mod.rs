/// Dynamic role management — contribution scoring and role promotion/demotion.
///
/// Nodes earn contribution scores by relaying messages for the network.
/// High scorers get promoted to Relay role; low scorers get demoted back to Peer.
/// Scores decay progressively (5%/hour) — no permanent bans (design decision #4).
pub mod antispam;
pub mod manager;
pub mod metrics;
pub mod scoring;

pub use antispam::{AntiSpam, AntiSpamConfig};
pub use manager::{RoleAction, RoleManager};
pub use metrics::RoleMetrics;
pub use scoring::ContributionMetrics;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::relay::{PeerInfo, PeerRole, PeerStatus, Topology};
    use crate::types::NodeId;

    fn node_id(seed: u8) -> NodeId {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
        let secret = tom_connect::SecretKey::generate(&mut rng);
        secret.public().to_string().parse().unwrap()
    }

    fn make_topology(nodes: &[(NodeId, PeerRole)]) -> Topology {
        let mut topo = Topology::new();
        for (id, role) in nodes {
            topo.upsert(PeerInfo {
                node_id: *id,
                role: *role,
                status: PeerStatus::Online,
                last_seen: 1000,
            });
        }
        topo
    }

    // ── ContributionMetrics integration ───────────────────────────────

    #[test]
    fn score_zero_at_creation() {
        let m = ContributionMetrics::new(0);
        assert_eq!(m.score(0), 0.0, "fresh node with no activity has score 0");
    }

    #[test]
    fn single_relay_produces_positive_score() {
        let mut m = ContributionMetrics::new(1000);
        m.record_relay(2000);
        let s = m.score(2000);
        assert!(s > 0.0, "one relay should produce positive score, got {s}");
    }

    #[test]
    fn score_increases_monotonically_with_relays() {
        let mut m = ContributionMetrics::new(0);
        let mut prev = 0.0f64;
        for i in 1..=10u64 {
            m.record_relay(i * 1000);
            let s = m.score(i * 1000);
            assert!(s > prev, "relay {i}: score should grow ({prev} → {s})");
            prev = s;
        }
    }

    #[test]
    fn score_decays_5pct_per_hour() {
        let mut m = ContributionMetrics::new(0);
        m.record_relay(1000);
        let at_0h = m.score(1000);
        let at_1h = m.score(1000 + 3_600_000);
        // Expected decay: e^(-0.05) ≈ 0.9512
        let ratio = at_1h / at_0h;
        assert!(
            (ratio - 0.9512).abs() < 0.002,
            "1h decay ratio should be ~0.9512, got {ratio:.4}"
        );
    }

    #[test]
    fn score_never_goes_negative() {
        let mut m = ContributionMetrics::new(0);
        m.record_relay(1000);
        // After 1000 hours (extreme idle)
        let s = m.score(1000 + 1000 * 3_600_000);
        assert!(s >= 0.0, "score must never go negative, got {s}");
    }

    #[test]
    fn all_failures_leaves_zero_success_rate_contribution() {
        let mut m = ContributionMetrics::new(0);
        for i in 0..5u64 {
            m.record_relay_failure(1000 + i * 500);
        }
        // success_rate = 0, so success_rate * weight = 0
        // Score comes only from uptime component
        let s = m.score(3000);
        // 5 failures → uptime accumulates but success_rate=0, relay_count=0
        assert!(s >= 0.0, "score non-negative even with all failures");
    }

    #[test]
    fn bandwidth_ratio_pure_giver() {
        let mut m = ContributionMetrics::new(0);
        m.record_relay(1000);
        m.bytes_relayed = 10 * 1_048_576; // 10 MB relayed
        m.bytes_received = 0;             // nothing received → pure giver (ratio clamped to 1.0)
        let s = m.score(1000);
        // bandwidth_ratio component: 1.0 * 1.5 = 1.5 extra
        assert!(s > 6.0, "pure-giver should have ratio bonus, got {s}");
    }

    #[test]
    fn bytes_relayed_scale_score() {
        let mut low = ContributionMetrics::new(0);
        low.record_relay(1000);
        low.bytes_relayed = 1 * 1_048_576; // 1 MB

        let mut high = ContributionMetrics::new(0);
        high.record_relay(1000);
        high.bytes_relayed = 1000 * 1_048_576; // 1000 MB

        assert!(
            high.score(1000) > low.score(1000),
            "more bandwidth → higher score"
        );
    }

    // ── RoleManager integration ────────────────────────────────────────

    #[test]
    fn unknown_node_score_is_zero() {
        let local = node_id(1);
        let unknown = node_id(99);
        let mgr = RoleManager::new(local);
        assert_eq!(mgr.score(&unknown, 1000), 0.0);
    }

    #[test]
    fn restore_scores_loads_metrics() {
        let local = node_id(1);
        let peer = node_id(2);
        let mut mgr = RoleManager::new(local);

        // Build up score then save
        for i in 0..20u64 {
            mgr.record_relay(peer, 1000 + i * 1000);
        }
        let snapshot = mgr.scores().clone();
        let score_before = mgr.score(&peer, 20_000);

        // New manager with restored scores
        let mut mgr2 = RoleManager::new(local);
        mgr2.restore_scores(snapshot);

        assert!(
            (mgr2.score(&peer, 20_000) - score_before).abs() < f64::EPSILON,
            "restored scores should be identical"
        );
    }

    #[test]
    fn relay_already_promoted_no_double_action() {
        let local = node_id(1);
        let peer = node_id(2);
        let mut mgr = RoleManager::new(local);
        let mut topo = make_topology(&[(peer, PeerRole::Relay)]);

        // Score well above threshold
        for i in 0..25u64 {
            mgr.record_relay(peer, 1000 + i * 1000);
        }

        let actions = mgr.evaluate(&mut topo, 25_000);
        // Already a Relay, not below demotion — no action
        assert!(
            !actions.iter().any(|a| matches!(a, RoleAction::Promoted { node_id, .. } if *node_id == peer)),
            "already-Relay should not get Promoted action again"
        );
    }

    #[test]
    fn peer_with_score_between_thresholds_no_action() {
        let local = node_id(1);
        let peer = node_id(2);
        let mut mgr = RoleManager::new(local);
        let mut topo = make_topology(&[(peer, PeerRole::Peer)]);

        // 2 relays → score ~7-8 (between DEMOTION=2 and PROMOTION=10)
        mgr.record_relay(peer, 1000);
        mgr.record_relay(peer, 2000);

        let score = mgr.score(&peer, 2000);
        assert!(score > 2.0 && score < 10.0, "score should be between thresholds: {score}");

        let actions = mgr.evaluate(&mut topo, 2000);
        assert!(actions.is_empty(), "mid-score peer should not change role: {actions:?}");
    }

    #[test]
    fn record_bytes_relayed_tracked() {
        let local = node_id(1);
        let peer = node_id(2);
        let mut mgr = RoleManager::new(local);

        mgr.record_bytes_relayed(peer, 5 * 1_048_576, 1000);

        let s = mgr.scores().get(&peer).expect("entry created");
        assert_eq!(s.bytes_relayed, 5 * 1_048_576);
    }

    #[test]
    fn record_bytes_received_tracked() {
        let local = node_id(1);
        let peer = node_id(2);
        let mut mgr = RoleManager::new(local);

        mgr.record_bytes_received(peer, 2 * 1_048_576, 1000);

        let s = mgr.scores().get(&peer).expect("entry created");
        assert_eq!(s.bytes_received, 2 * 1_048_576);
    }

    #[test]
    fn multiple_relays_fail_then_recover() {
        let local = node_id(1);
        let peer = node_id(2);
        let mut mgr = RoleManager::new(local);
        let mut topo = make_topology(&[(peer, PeerRole::Relay)]);

        // One relay → heavy decay → should demote
        mgr.record_relay(peer, 1000);
        let actions_demote = mgr.evaluate(&mut topo, 1000 + 100 * 3_600_000);
        assert!(
            actions_demote.iter().any(|a| matches!(a, RoleAction::Demoted { .. })),
            "extreme decay should demote"
        );

        // Resume heavy activity → should re-promote
        let base = 1000 + 100 * 3_600_000;
        for i in 0..25u64 {
            mgr.record_relay(peer, base + i * 1000);
        }
        let actions_promote = mgr.evaluate(&mut topo, base + 25_000);
        assert!(
            actions_promote.iter().any(|a| matches!(a, RoleAction::Promoted { .. })),
            "re-activity should re-promote after recovery"
        );
    }

    #[test]
    fn get_all_scores_returns_topology_peers() {
        let local = node_id(1);
        let peer_a = node_id(2);
        let peer_b = node_id(3);
        let mut mgr = RoleManager::new(local);
        let topo = make_topology(&[(peer_a, PeerRole::Peer), (peer_b, PeerRole::Relay)]);

        mgr.record_relay(peer_a, 1000);

        let all = mgr.get_all_scores(&topo, 1000);
        assert_eq!(all.len(), 2, "should list all topology peers");
        let peer_a_entry = all.iter().find(|(id, _, _)| *id == peer_a);
        assert!(peer_a_entry.is_some());
        assert!(peer_a_entry.unwrap().1 > 0.0, "peer_a should have nonzero score");
    }
}
