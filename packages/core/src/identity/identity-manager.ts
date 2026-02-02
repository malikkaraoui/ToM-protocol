import {
  type NodeId,
  type NodeIdentity,
  generateKeypair,
  publicKeyToNodeId,
  signData,
  verifySignature,
} from './keypair.js';
import type { IdentityStorage } from './storage.js';

export class IdentityManager {
  private storage: IdentityStorage;
  private identity: NodeIdentity | null = null;

  constructor(storage: IdentityStorage) {
    this.storage = storage;
  }

  async init(): Promise<NodeIdentity> {
    const existing = await this.storage.load();
    if (existing) {
      this.identity = existing;
      return existing;
    }

    const created = generateKeypair();
    await this.storage.save(created);
    this.identity = created;
    return created;
  }

  getNodeId(): NodeId {
    if (!this.identity) {
      throw new Error('IdentityManager not initialized. Call init() first.');
    }
    return publicKeyToNodeId(this.identity.publicKey);
  }

  sign(data: Uint8Array): Uint8Array {
    if (!this.identity) {
      throw new Error('IdentityManager not initialized. Call init() first.');
    }
    return signData(this.identity.secretKey, data);
  }

  verify(publicKey: Uint8Array, data: Uint8Array, signature: Uint8Array): boolean {
    return verifySignature(publicKey, data, signature);
  }
}
