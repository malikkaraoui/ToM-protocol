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
