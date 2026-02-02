export const TOM_PROTOCOL_VERSION = '0.0.1';

export {
  IdentityManager,
  generateKeypair,
  signData,
  verifySignature,
  publicKeyToNodeId,
  MemoryStorage,
  LocalStorageAdapter,
  FileStorageAdapter,
} from './identity/index.js';
export type { NodeIdentity, NodeId, IdentityStorage } from './identity/index.js';

export type { MessageEnvelope } from './types/index.js';
export type { TomEventMap } from './types/index.js';

export { TomError } from './errors/index.js';
export type { TomErrorCode } from './errors/index.js';

export { TransportLayer } from './transport/index.js';
export type { PeerConnection, TransportEvents, SignalingClient } from './transport/index.js';
