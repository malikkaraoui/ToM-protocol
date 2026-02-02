import type { TransportLayer } from '../transport/transport-layer.js';
import type { MessageEnvelope } from '../types/envelope.js';

export interface RouterEvents {
  onMessageDelivered: (envelope: MessageEnvelope) => void;
  onMessageForwarded: (envelope: MessageEnvelope, nextHop: string) => void;
  onMessageRejected: (envelope: MessageEnvelope, reason: string) => void;
}

export interface SignatureVerifier {
  verify(publicKey: Uint8Array, data: Uint8Array, signature: Uint8Array): boolean;
}

export class Router {
  private localNodeId: string;
  private transport: TransportLayer;
  private events: RouterEvents;
  private verifier: SignatureVerifier | null;

  constructor(localNodeId: string, transport: TransportLayer, events: RouterEvents, verifier?: SignatureVerifier) {
    this.localNodeId = localNodeId;
    this.transport = transport;
    this.events = events;
    this.verifier = verifier ?? null;
  }

  handleIncoming(envelope: MessageEnvelope): void {
    // If this message is addressed to us, deliver it
    if (envelope.to === this.localNodeId) {
      this.events.onMessageDelivered(envelope);
      return;
    }

    // Otherwise, forward to the intended recipient
    const nextHop = envelope.to;
    const peer = this.transport.getPeer(nextHop);

    if (!peer) {
      this.events.onMessageRejected(envelope, 'PEER_UNREACHABLE');
      return;
    }

    peer.send(envelope);
    this.events.onMessageForwarded(envelope, nextHop);
  }

  createEnvelope(
    to: string,
    type: string,
    payload: unknown,
    via: string[],
    sign?: (data: Uint8Array) => Uint8Array,
  ): MessageEnvelope {
    const envelope: MessageEnvelope = {
      id: crypto.randomUUID(),
      from: this.localNodeId,
      to,
      via,
      type,
      payload,
      timestamp: Date.now(),
      signature: '',
    };

    if (sign) {
      const data = new TextEncoder().encode(JSON.stringify({ ...envelope, signature: '' }));
      const sig = sign(data);
      envelope.signature = Array.from(sig)
        .map((b) => b.toString(16).padStart(2, '0'))
        .join('');
    }

    return envelope;
  }

  sendViaRelay(envelope: MessageEnvelope, relayId: string): void {
    const relayPeer = this.transport.getPeer(relayId);
    if (!relayPeer) {
      this.events.onMessageRejected(envelope, 'RELAY_UNREACHABLE');
      return;
    }

    // Add relay to via path if not already present
    if (!envelope.via.includes(relayId)) {
      envelope.via.push(relayId);
    }

    relayPeer.send(envelope);
  }
}
