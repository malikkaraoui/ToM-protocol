import type { PeerInfo } from '../discovery/network-topology.js';
import type { NodeId } from '../identity/index.js';
import type { MessageEnvelope } from './envelope.js';

export interface TomEventMap {
  'message:received': MessageEnvelope;
  'message:sent': MessageEnvelope;
  'peer:connected': NodeId;
  'peer:disconnected': NodeId;
  'peer:discovered': PeerInfo;
  'peer:departed': NodeId;
  'peer:stale': NodeId;
  'identity:ready': NodeId;
}
