use crate::envelope::Envelope;
use crate::tracker::StatusChange;
use crate::types::NodeId;

use super::{DeliveredMessage, ProtocolEvent};

/// Intention produite par la logique pure de RuntimeState.
///
/// Chaque methode handle_* / tick_* retourne Vec<RuntimeEffect>.
/// La boucle principale execute ensuite ces effets via Transport + channels.
#[derive(Debug)]
pub enum RuntimeEffect {
    /// Envoyer une enveloppe au premier hop (relay ou direct).
    SendEnvelope(Envelope),

    /// Envoyer une enveloppe a un noeud precis (hop explicite).
    SendEnvelopeTo {
        target: NodeId,
        envelope: Envelope,
    },

    /// Livrer un message decrypte a l'application (TUI, bot...).
    DeliverMessage(DeliveredMessage),

    /// Notifier un changement de statut (pending -> sent -> relayed -> delivered -> read).
    StatusChange(StatusChange),

    /// Emettre un evenement protocole (peer offline, group created, etc.).
    Emit(ProtocolEvent),

    /// Essayer d'envoyer — si le transport echoue, executer le plan B.
    /// Utilise pour le backup automatique quand un peer est offline.
    SendWithBackupFallback {
        envelope: Envelope,
        on_success: Vec<RuntimeEffect>,
        on_failure: Vec<RuntimeEffect>,
    },
}
