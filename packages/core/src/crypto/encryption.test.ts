import { describe, expect, it } from 'vitest';
import {
  decryptPayload,
  encryptPayload,
  encryptionKeyToHex,
  generateEncryptionKeypair,
  hexToEncryptionKey,
  isEncryptedPayload,
} from './encryption.js';

describe('Encryption Module', () => {
  describe('generateEncryptionKeypair', () => {
    it('should generate a valid keypair', () => {
      const keypair = generateEncryptionKeypair();
      expect(keypair.publicKey).toBeInstanceOf(Uint8Array);
      expect(keypair.secretKey).toBeInstanceOf(Uint8Array);
      expect(keypair.publicKey.length).toBe(32); // Curve25519 public key
      expect(keypair.secretKey.length).toBe(32); // Curve25519 secret key
    });

    it('should generate unique keypairs', () => {
      const keypair1 = generateEncryptionKeypair();
      const keypair2 = generateEncryptionKeypair();
      expect(keypair1.publicKey).not.toEqual(keypair2.publicKey);
      expect(keypair1.secretKey).not.toEqual(keypair2.secretKey);
    });
  });

  describe('key encoding', () => {
    it('should round-trip public key through hex encoding', () => {
      const keypair = generateEncryptionKeypair();
      const hex = encryptionKeyToHex(keypair.publicKey);
      const decoded = hexToEncryptionKey(hex);
      expect(decoded).toEqual(keypair.publicKey);
    });

    it('should produce consistent hex encoding', () => {
      const keypair = generateEncryptionKeypair();
      const hex1 = encryptionKeyToHex(keypair.publicKey);
      const hex2 = encryptionKeyToHex(keypair.publicKey);
      expect(hex1).toBe(hex2);
      expect(hex1.length).toBe(64); // 32 bytes = 64 hex chars
    });
  });

  describe('encryptPayload / decryptPayload', () => {
    it('should encrypt and decrypt a simple payload', () => {
      const sender = generateEncryptionKeypair();
      const recipient = generateEncryptionKeypair();

      const payload = { message: 'Hello, World!', timestamp: Date.now() };
      const encrypted = encryptPayload(payload, recipient.publicKey);
      const decrypted = decryptPayload(encrypted, recipient.secretKey);

      expect(decrypted).toEqual(payload);
    });

    it('should encrypt and decrypt complex payloads', () => {
      const recipient = generateEncryptionKeypair();

      const payload = {
        type: 'chat',
        text: 'Test message with unicode: ',
        nested: { a: 1, b: [2, 3, 4] },
        nullValue: null,
        boolValue: true,
      };

      const encrypted = encryptPayload(payload, recipient.publicKey);
      const decrypted = decryptPayload(encrypted, recipient.secretKey);

      expect(decrypted).toEqual(payload);
    });

    it('should fail to decrypt with wrong key', () => {
      const recipient = generateEncryptionKeypair();
      const wrongRecipient = generateEncryptionKeypair();

      const payload = { secret: 'data' };
      const encrypted = encryptPayload(payload, recipient.publicKey);
      const decrypted = decryptPayload(encrypted, wrongRecipient.secretKey);

      expect(decrypted).toBeNull();
    });

    it('should produce different ciphertext for same payload (ephemeral keys)', () => {
      const recipient = generateEncryptionKeypair();
      const payload = { message: 'same content' };

      const encrypted1 = encryptPayload(payload, recipient.publicKey);
      const encrypted2 = encryptPayload(payload, recipient.publicKey);

      // Different ephemeral keys = different ciphertext
      expect(encrypted1.ciphertext).not.toBe(encrypted2.ciphertext);
      expect(encrypted1.ephemeralPublicKey).not.toBe(encrypted2.ephemeralPublicKey);

      // But both decrypt to same content
      const decrypted1 = decryptPayload(encrypted1, recipient.secretKey);
      const decrypted2 = decryptPayload(encrypted2, recipient.secretKey);
      expect(decrypted1).toEqual(payload);
      expect(decrypted2).toEqual(payload);
    });

    it('should detect tampered ciphertext', () => {
      const recipient = generateEncryptionKeypair();
      const payload = { message: 'original' };
      const encrypted = encryptPayload(payload, recipient.publicKey);

      // Tamper with ciphertext
      const tampered = {
        ...encrypted,
        ciphertext: `${encrypted.ciphertext.slice(0, -2)}ff`,
      };

      const decrypted = decryptPayload(tampered, recipient.secretKey);
      expect(decrypted).toBeNull();
    });
  });

  describe('isEncryptedPayload', () => {
    it('should return true for valid encrypted payloads', () => {
      const recipient = generateEncryptionKeypair();
      const encrypted = encryptPayload({ test: true }, recipient.publicKey);
      expect(isEncryptedPayload(encrypted)).toBe(true);
    });

    it('should return false for invalid payloads', () => {
      expect(isEncryptedPayload(null)).toBe(false);
      expect(isEncryptedPayload(undefined)).toBe(false);
      expect(isEncryptedPayload({})).toBe(false);
      expect(isEncryptedPayload({ ciphertext: 'abc' })).toBe(false);
      expect(isEncryptedPayload({ ciphertext: 'abc', nonce: 'def' })).toBe(false);
      expect(isEncryptedPayload('string')).toBe(false);
      expect(isEncryptedPayload(123)).toBe(false);
    });

    it('should return true only when all required fields are strings', () => {
      expect(
        isEncryptedPayload({
          ciphertext: 'abc',
          nonce: 'def',
          ephemeralPublicKey: 'ghi',
        }),
      ).toBe(true);

      expect(
        isEncryptedPayload({
          ciphertext: 123,
          nonce: 'def',
          ephemeralPublicKey: 'ghi',
        }),
      ).toBe(false);
    });
  });
});
