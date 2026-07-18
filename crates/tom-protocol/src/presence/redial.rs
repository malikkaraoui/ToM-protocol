//! M1 — Re-dial ciblé sur « présence attestée + chemin local mort ».
//!
//! Gouverneur PUR (zéro I/O) qui décide QUAND déclencher un re-dial vers un
//! pair pour lequel on a une évidence « vivant » fraîche (quorum de témoins
//! L1-003) alors que notre chemin local est mort (PeerStale ou mark_failed).
//!
//! Conception : `docs/plans/redial-presence-attestee-chemin-mort.md` §4.
//! Non-buts (§3, décisions LOCKED) : pas de dial-storm (throttle strict), pas
//! d'état persisté, accélérateur ciblé — jamais l'unique chemin de récupération
//! (le tick 15 s existant reste le filet de convergence de masse).
//!
//! Garde-fous (review-oracle 18/07) :
//! - horloge qui recule → `saturating_sub`, refus silencieux, jamais de panique ;
//! - `PeerStale` ET `mark_failed` même tick → un seul redial en vol par pair ;
//! - cap global atteint → éviction FIFO, zéro file d'attente ;
//! - reset du throttle sur succès (trafic entrant direct prouvé).

use std::collections::HashMap;

use crate::types::NodeId;

/// Fenêtre de throttle par pair (ms). 60 s = période du cycle rendez-vous
/// existant (`runtime/loop.rs`), rythme auquel l'adressage frais se renouvelle
/// de toute façon — pas une constante neuve (doc §4 throttle).
pub const REDIAL_THROTTLE_MS: u64 = 60_000;

/// Nombre maximum de re-dials « en vol » simultanés (cap global anti-charge,
/// décision #5). Au-delà : éviction FIFO du plus ancien (doc §4).
pub const REDIAL_MAX_IN_FLIGHT: usize = 3;

/// Décideur pur du re-dial ciblé. Ne dial rien lui-même : il autorise ou refuse.
#[derive(Debug, Default)]
pub struct RedialGovernor {
    /// peer → timestamp (ms) du dernier re-dial déclenché. Sert à la fois de
    /// throttle par pair ET de marqueur « en vol » (effacé par `on_success`).
    last_redial: HashMap<NodeId, u64>,
    /// Ordre d'insertion des pairs en vol (FIFO), pour l'éviction sous cap.
    order: Vec<NodeId>,
}

impl RedialGovernor {
    pub fn new() -> Self {
        Self::default()
    }

    /// Nombre de re-dials actuellement « en vol » (déclenchés, pas encore
    /// confirmés par un trafic entrant). Exposé pour métriques/tests.
    pub fn in_flight(&self) -> usize {
        self.order.len()
    }

    /// Tente d'autoriser un re-dial vers `peer` à l'instant `now` (ms).
    ///
    /// Retourne `true` si le re-dial doit être émis. Applique :
    /// - throttle par pair : refus si un re-dial a été déclenché il y a moins de
    ///   `REDIAL_THROTTLE_MS` (horloge qui recule → `saturating_sub` → refus) ;
    /// - cap global : si `REDIAL_MAX_IN_FLIGHT` pairs distincts sont déjà en vol,
    ///   éviction FIFO du plus ancien pour faire de la place (jamais de file).
    ///
    /// Idempotent dans un même tick : un `peer` déjà en vol et non expiré est
    /// refusé, donc `PeerStale` + `mark_failed` au même tick → un seul redial.
    pub fn try_trigger(&mut self, peer: NodeId, now: u64) -> bool {
        if let Some(&last) = self.last_redial.get(&peer) {
            // saturating_sub : si `now < last` (horloge reculée), la différence
            // sature à 0 < throttle → refus. Jamais de panique (orage #28 : Skip).
            if now.saturating_sub(last) < REDIAL_THROTTLE_MS {
                return false;
            }
            // Le throttle du pair a expiré : on le ré-arme (déjà dans l'ordre).
            self.last_redial.insert(peer, now);
            return true;
        }

        // Nouveau pair en vol : appliquer le cap global AVANT insertion.
        if self.order.len() >= REDIAL_MAX_IN_FLIGHT {
            // Éviction FIFO : le plus ancien cède sa place (doc §4, pas de file).
            let evicted = self.order.remove(0);
            self.last_redial.remove(&evicted);
        }
        self.last_redial.insert(peer, now);
        self.order.push(peer);
        true
    }

    /// Réinitialise le throttle d'un pair : appelé quand un trafic entrant
    /// direct prouve que le chemin est ré-établi (doc §4 « reset sur succès »).
    /// Le pair quitte l'état « en vol » → une future transition pourra re-dialer
    /// immédiatement sans attendre la fenêtre de throttle.
    pub fn on_success(&mut self, peer: &NodeId) {
        if self.last_redial.remove(peer).is_some() {
            self.order.retain(|p| p != peer);
        }
    }

    /// Purge les entrées dont le throttle a expiré depuis longtemps (évite une
    /// croissance non bornée si des pairs disparaissent sans succès). Appelée
    /// depuis le tick de nettoyage présence. Conserve les entrées récentes.
    pub fn purge_expired(&mut self, now: u64) {
        let stale: Vec<NodeId> = self
            .last_redial
            .iter()
            .filter(|(_, &last)| now.saturating_sub(last) >= REDIAL_THROTTLE_MS)
            .map(|(p, _)| *p)
            .collect();
        for p in stale {
            self.last_redial.remove(&p);
            self.order.retain(|q| q != &p);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    fn peer(seed: u8) -> NodeId {
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
        let secret = tom_connect::SecretKey::generate(&mut rng);
        secret.public().to_string().parse().unwrap()
    }

    #[test]
    fn first_trigger_allowed_then_throttled() {
        let mut g = RedialGovernor::new();
        let p = peer(1);
        assert!(g.try_trigger(p, 1_000), "premier redial doit passer");
        // Même pair, dans la fenêtre → refusé.
        assert!(!g.try_trigger(p, 1_000 + 30_000), "throttle 60 s doit refuser");
        assert!(!g.try_trigger(p, 1_000 + 59_999), "juste avant l'échéance : refusé");
        // Après la fenêtre → ré-autorisé.
        assert!(g.try_trigger(p, 1_000 + 60_000), "après 60 s : ré-autorisé");
    }

    #[test]
    fn stale_and_failed_same_tick_single_redial() {
        // PeerStale ET mark_failed au même instant → un seul redial (idempotent).
        let mut g = RedialGovernor::new();
        let p = peer(2);
        assert!(g.try_trigger(p, 5_000), "1er déclencheur");
        assert!(!g.try_trigger(p, 5_000), "2e déclencheur même tick → refusé");
        assert_eq!(g.in_flight(), 1);
    }

    #[test]
    fn clock_rewind_refuses_without_panic() {
        let mut g = RedialGovernor::new();
        let p = peer(3);
        assert!(g.try_trigger(p, 100_000));
        // Horloge qui recule : now < last → saturating_sub = 0 < throttle → refus.
        assert!(!g.try_trigger(p, 40_000), "horloge reculée → refus silencieux");
    }

    #[test]
    fn global_cap_fifo_eviction() {
        let mut g = RedialGovernor::new();
        let (a, b, c, d) = (peer(10), peer(11), peer(12), peer(13));
        assert!(g.try_trigger(a, 1000));
        assert!(g.try_trigger(b, 1001));
        assert!(g.try_trigger(c, 1002));
        assert_eq!(g.in_flight(), REDIAL_MAX_IN_FLIGHT);
        // 4e pair : cap atteint → éviction FIFO de `a`, `d` accepté.
        assert!(g.try_trigger(d, 1003), "4e pair accepté après éviction FIFO");
        assert_eq!(g.in_flight(), REDIAL_MAX_IN_FLIGHT, "cap tenu, pas de file");
        // `a` a été évincé → il peut re-déclencher tout de suite (throttle oublié).
        assert!(g.try_trigger(a, 1004), "pair évincé peut re-déclencher");
    }

    #[test]
    fn success_resets_throttle() {
        let mut g = RedialGovernor::new();
        let p = peer(4);
        assert!(g.try_trigger(p, 1000));
        assert!(!g.try_trigger(p, 2000), "throttlé");
        // Trafic entrant direct prouvé → reset.
        g.on_success(&p);
        assert_eq!(g.in_flight(), 0, "succès retire le pair du vol");
        assert!(g.try_trigger(p, 2001), "après succès : re-dial immédiat autorisé");
    }

    #[test]
    fn on_success_unknown_peer_is_noop() {
        let mut g = RedialGovernor::new();
        g.on_success(&peer(99)); // ne doit rien casser
        assert_eq!(g.in_flight(), 0);
    }

    #[test]
    fn purge_expired_frees_slots() {
        let mut g = RedialGovernor::new();
        let (a, b) = (peer(20), peer(21));
        assert!(g.try_trigger(a, 1000));
        assert!(g.try_trigger(b, 1000));
        assert_eq!(g.in_flight(), 2);
        // Bien après la fenêtre → purge retire les deux.
        g.purge_expired(1000 + REDIAL_THROTTLE_MS + 1);
        assert_eq!(g.in_flight(), 0, "purge libère les entrées expirées");
    }
}
