import { describe, expect, it } from 'vitest';
import { IdentityManager } from './identity-manager.js';
import { MemoryStorage } from './storage.js';

describe('IdentityManager', () => {
  it('generates new identity when no stored identity exists', async () => {
    const storage = new MemoryStorage();
    const manager = new IdentityManager(storage);
    const identity = await manager.init();
    expect(identity.publicKey).toBeInstanceOf(Uint8Array);
    expect(identity.publicKey.length).toBe(32);
    expect(identity.secretKey.length).toBe(64);
  });

  it('loads existing identity from storage without generating new one', async () => {
    const storage = new MemoryStorage();
    const manager1 = new IdentityManager(storage);
    const identity1 = await manager1.init();

    const manager2 = new IdentityManager(storage);
    const identity2 = await manager2.init();

    expect(identity2.publicKey).toEqual(identity1.publicKey);
    expect(identity2.secretKey).toEqual(identity1.secretKey);
  });

  it('getNodeId returns consistent hex string', async () => {
    const storage = new MemoryStorage();
    const manager = new IdentityManager(storage);
    await manager.init();
    const id1 = manager.getNodeId();
    const id2 = manager.getNodeId();
    expect(id1).toBe(id2);
    expect(id1).toMatch(/^[0-9a-f]{64}$/);
  });

  it('throws if getNodeId called before init', () => {
    const storage = new MemoryStorage();
    const manager = new IdentityManager(storage);
    expect(() => manager.getNodeId()).toThrow('not initialized');
  });

  it('sign and verify round-trip works through manager', async () => {
    const storage = new MemoryStorage();
    const manager = new IdentityManager(storage);
    const identity = await manager.init();
    const data = new TextEncoder().encode('test message');
    const signature = manager.sign(data);
    expect(manager.verify(identity.publicKey, data, signature)).toBe(true);
  });
});
