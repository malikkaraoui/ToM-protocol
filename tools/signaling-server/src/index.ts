// TEMPORARY: Bootstrap signaling server (ADR-002) — marked for elimination
// This server exists only for the PoC phase. It will be replaced by
// distributed DHT-based peer discovery as the network matures.

export const SIGNALING_SERVER_VERSION = '0.0.1';

export interface Participant {
  nodeId: string;
  username: string;
}

export interface SignalingMessage {
  type: 'register' | 'signal' | 'participants' | 'error';
  from?: string;
  to?: string;
  nodeId?: string;
  username?: string;
  participants?: Participant[];
  payload?: unknown;
  error?: string;
}

export { createSignalingServer } from './server.js';
