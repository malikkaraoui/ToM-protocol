//! Cache d'enveloppes en attente d'ACK, borné en mémoire.
//!
//! Une enveloppe est conservée après émission pour pouvoir être réémise si
//! l'ACK n'arrive pas (R9.2). Le cache retenait l'enveloppe COMPLÈTE — payload
//! inclus — dans une `HashMap` sans aucune borne, ni en nombre ni en octets.
//!
//! Conséquence mesurée le 2026-07-19 : en envoyant 240 Mo vers un pair hors
//! ligne (donc rien n'est jamais acquitté, donc rien ne sort du cache avant
//! expiration du tracker), le relais NAS est monté à 780 Mo de RSS et l'OOM
//! killer l'a tué — y compris après que le backup store ait, lui, été borné.
//!
//! C'est la 4ᵉ occurrence de la même classe de bug dans ce projet : une
//! structure qui retient des payloads avec une borne par-unité (ou aucune)
//! mais sans budget global en octets. D'où le choix d'encapsuler ici plutôt
//! que d'ajouter un compteur à côté d'une `HashMap` manipulée depuis sept
//! endroits : le compteur ne peut pas dériver s'il n'existe qu'un seul chemin
//! d'insertion et de suppression.

use std::collections::HashMap;

use crate::envelope::Envelope;

/// Budget mémoire du cache de réémission.
///
/// Volontairement plus bas que celui du backup store (64 Mio) : la réémission
/// est un confort d'optimisation, alors que le backup est le vrai filet
/// anti-perte (ADR-009). Sous pression, mieux vaut perdre la capacité de
/// retenter un envoi que la copie de secours du message.
const MAX_PENDING_BYTES: usize = 32 * 1024 * 1024;

/// Cache borné d'enveloppes en attente d'ACK.
#[derive(Default)]
pub(crate) struct PendingEnvelopes {
    by_id: HashMap<String, Envelope>,
    /// Somme des `payload.len()` — maintenue incrémentalement, jamais
    /// recalculée par balayage.
    bytes: usize,
}

impl PendingEnvelopes {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Met une enveloppe en cache, en évinçant les plus anciennes si le budget
    /// est dépassé. Retourne `false` si l'enveloppe n'a pas pu être mise en
    /// cache (payload plus gros que le budget entier) : l'envoi lui-même n'est
    /// pas affecté, seule la réémission automatique ne sera pas possible.
    pub(crate) fn insert(&mut self, id: String, envelope: Envelope) -> bool {
        let incoming = envelope.payload.len();
        if incoming > MAX_PENDING_BYTES {
            tracing::warn!(
                message_id = %id,
                payload_bytes = incoming,
                budget = MAX_PENDING_BYTES,
                "envelope trop grosse pour le cache de réémission — envoi non affecté"
            );
            return false;
        }

        // Remplacement d'une entrée existante : défalquer l'ancienne d'abord.
        if let Some(old) = self.by_id.remove(&id) {
            self.bytes = self.bytes.saturating_sub(old.payload.len());
        }

        while self.bytes.saturating_add(incoming) > MAX_PENDING_BYTES {
            let Some(oldest) = self.oldest_id() else {
                break;
            };
            self.remove(&oldest);
            tracing::warn!(
                evicted = %oldest,
                bytes = self.bytes,
                budget = MAX_PENDING_BYTES,
                "cache de réémission saturé — plus ancienne enveloppe évincée"
            );
        }

        self.bytes = self.bytes.saturating_add(incoming);
        self.by_id.insert(id, envelope);
        true
    }

    pub(crate) fn get(&self, id: &str) -> Option<&Envelope> {
        self.by_id.get(id)
    }

    pub(crate) fn remove(&mut self, id: &str) -> Option<Envelope> {
        let envelope = self.by_id.remove(id)?;
        self.bytes = self.bytes.saturating_sub(envelope.payload.len());
        Some(envelope)
    }

    /// Ne garde que les entrées pour lesquelles `keep` renvoie vrai.
    pub(crate) fn retain<F: Fn(&str) -> bool>(&mut self, keep: F) {
        let doomed: Vec<String> = self
            .by_id
            .keys()
            .filter(|id| !keep(id))
            .cloned()
            .collect();
        for id in doomed {
            self.remove(&id);
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.by_id.len()
    }

    pub(crate) fn bytes(&self) -> usize {
        self.bytes
    }

    pub(crate) fn budget() -> usize {
        MAX_PENDING_BYTES
    }

    /// Enveloppe la plus ancienne au sens du timestamp d'émission.
    fn oldest_id(&self) -> Option<String> {
        self.by_id
            .iter()
            .min_by_key(|(_, env)| env.timestamp)
            .map(|(id, _)| id.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::envelope::Envelope;
    use crate::types::{MessageType, NodeId};

    fn node_id(seed: u8) -> NodeId {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
        let secret = tom_connect::SecretKey::generate(&mut rng);
        secret.public().to_string().parse().unwrap()
    }

    fn envelope_of(id: &str, bytes: usize, ts: u64) -> Envelope {
        let mut env = Envelope::new(node_id(1), node_id(2), MessageType::Chat, vec![0u8; bytes]);
        env.id = id.to_string();
        env.timestamp = ts;
        env
    }

    #[test]
    fn budget_is_enforced_in_bytes() {
        // Le scénario du NAS : beaucoup de gros messages vers un pair hors
        // ligne, donc aucun ACK, donc rien ne sort naturellement du cache.
        let mut pending = PendingEnvelopes::new();
        for i in 0..200 {
            pending.insert(format!("m{i}"), envelope_of(&format!("m{i}"), 1024 * 1024, i as u64));
        }
        assert!(
            pending.bytes() <= PendingEnvelopes::budget(),
            "budget dépassé : {} > {}",
            pending.bytes(),
            PendingEnvelopes::budget()
        );
        assert!(pending.len() < 200, "des entrées auraient dû être évincées");
    }

    #[test]
    fn oldest_is_evicted_first() {
        let mut pending = PendingEnvelopes::new();
        let big = MAX_PENDING_BYTES / 2;
        pending.insert("vieux".into(), envelope_of("vieux", big, 1));
        pending.insert("recent".into(), envelope_of("recent", big, 100));
        // Ce troisième dépôt force une éviction.
        pending.insert("nouveau".into(), envelope_of("nouveau", big, 200));

        assert!(!pending.get("vieux").is_some(), "le plus ancien doit partir en premier");
        assert!(pending.get("nouveau").is_some());
    }

    #[test]
    fn counter_returns_to_zero() {
        // Un décrément manqué ferait fuir le budget jusqu'à tout refuser :
        // on exerce les trois voies de sortie (remove, retain, remplacement).
        let mut pending = PendingEnvelopes::new();
        pending.insert("a".into(), envelope_of("a", 4096, 1));
        pending.insert("b".into(), envelope_of("b", 4096, 2));
        pending.insert("c".into(), envelope_of("c", 4096, 3));
        assert_eq!(pending.bytes(), 3 * 4096);

        pending.remove("a");
        pending.retain(|id| id != "b");
        pending.insert("c".into(), envelope_of("c", 8192, 4)); // remplacement

        assert_eq!(pending.len(), 1);
        assert_eq!(pending.bytes(), 8192, "le compteur a dérivé");

        pending.remove("c");
        assert_eq!(pending.bytes(), 0);
    }

    #[test]
    fn oversized_envelope_is_refused_without_flushing_the_cache() {
        let mut pending = PendingEnvelopes::new();
        pending.insert("garde".into(), envelope_of("garde", 4096, 1));

        let ok = pending.insert(
            "enorme".into(),
            envelope_of("enorme", MAX_PENDING_BYTES + 1, 2),
        );

        assert!(!ok);
        assert!(pending.get("enorme").is_none());
        assert!(
            pending.get("garde").is_some(),
            "un refus ne doit pas vider le cache existant"
        );
    }
}
