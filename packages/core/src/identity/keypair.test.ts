import { describe, expect, it } from 'vitest';
import { generateKeypair, publicKeyToNodeId, signData, verifySignature } from './keypair.js';

describe('keypair', () => {
  it('generates valid Ed25519 keypair', () => {
    const identity = generateKeypair();
    expect(identity.publicKey).toBeInstanceOf(Uint8Array);
    expect(identity.secretKey).toBeInstanceOf(Uint8Array);
    expect(identity.publicKey.length).toBe(32);
    expect(identity.secretKey.length).toBe(64);
  });

  it('generates unique keypairs', () => {
    const a = generateKeypair();
    const b = generateKeypair();
    expect(a.publicKey).not.toEqual(b.publicKey);
  });

  it('signs data and verifies signature successfully', () => {
    const identity = generateKeypair();
    const data = new TextEncoder().encode('hello tom');
    const signature = signData(identity.secretKey, data);
    expect(signature).toBeInstanceOf(Uint8Array);
    expect(signature.length).toBe(64);
    expect(verifySignature(identity.publicKey, data, signature)).toBe(true);
  });

  it('verification fails with wrong public key', () => {
    const identity = generateKeypair();
    const other = generateKeypair();
    const data = new TextEncoder().encode('hello tom');
    const signature = signData(identity.secretKey, data);
    expect(verifySignature(other.publicKey, data, signature)).toBe(false);
  });

  it('verification fails with tampered data', () => {
    const identity = generateKeypair();
    const data = new TextEncoder().encode('hello tom');
    const signature = signData(identity.secretKey, data);
    const tampered = new TextEncoder().encode('hello tampered');
    expect(verifySignature(identity.publicKey, tampered, signature)).toBe(false);
  });

  it('converts public key to hex node id', () => {
    const identity = generateKeypair();
    const nodeId = publicKeyToNodeId(identity.publicKey);
    expect(nodeId).toHaveLength(64);
    expect(nodeId).toMatch(/^[0-9a-f]{64}$/);
  });
});
