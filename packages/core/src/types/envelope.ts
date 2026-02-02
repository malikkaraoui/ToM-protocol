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
}
