import type { NodeId } from '../identity/index.js';

export interface MessageEnvelope {
  id: string;
  from: NodeId;
  to: NodeId;
  via: NodeId[];
  type: string;
  payload: unknown;
  timestamp: number;
  signature: string;

  // Crypto noop-pipeline (E2E ready - Epic 6)
  // When encrypted, payload becomes opaque and this field holds the ciphertext
  encryptedPayload?: string;
  // Sender's ephemeral public key for key exchange (X25519)
  ephemeralPublicKey?: string;
  // Nonce used for encryption
  nonce?: string;

  // Latency tracking (Golden path metrics)
  // Timestamps at each hop for latency measurement
  hopTimestamps?: number[];
  // Route type for path visualization
  routeType?: 'relay' | 'direct';
}
