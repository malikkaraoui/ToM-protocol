/**
 * Group Security Tests (Story 4.6 - Security Hardening)
 */

import { beforeEach, describe, expect, it, vi } from 'vitest';
import { generateKeypair, publicKeyToNodeId } from '../identity/index';
import {
  NonceTracker,
  createSignableData,
  generateNonce,
  hexToUint8Array,
  signGroupMessage,
  uint8ArrayToHex,
  verifyGroupMessageSignature,
} from './group-security';
import type { GroupMessagePayload } from './group-types';

describe('Group Security', () => {
  describe('generateNonce', () => {
    it('should generate unique nonces', () => {
      const nonce1 = generateNonce();
      const nonce2 = generateNonce();

      expect(nonce1).not.toBe(nonce2);
      expect(nonce1.length).toBeGreaterThan(0);
    });

    it('should generate hex-like strings', () => {
      const nonce = generateNonce();
      // Should be alphanumeric (hex or UUID format)
      expect(nonce).toMatch(/^[a-f0-9-]+$/);
    });
  });

  describe('NonceTracker', () => {
    let tracker: NonceTracker;

    beforeEach(() => {
      tracker = new NonceTracker({ maxAgeMs: 1000, maxSize: 100 });
    });

    it('should accept new nonces', () => {
      expect(tracker.checkAndRecord('nonce-1')).toBe(true);
      expect(tracker.checkAndRecord('nonce-2')).toBe(true);
    });

    it('should reject duplicate nonces', () => {
      expect(tracker.checkAndRecord('nonce-1')).toBe(true);
      expect(tracker.checkAndRecord('nonce-1')).toBe(false); // Replay!
    });

    it('should track size', () => {
      tracker.checkAndRecord('nonce-1');
      tracker.checkAndRecord('nonce-2');
      expect(tracker.size).toBe(2);
    });

    it('should cleanup expired nonces', () => {
      vi.useFakeTimers();

      tracker.checkAndRecord('nonce-1');
      expect(tracker.size).toBe(1);

      // Advance past maxAge
      vi.advanceTimersByTime(2000);
      tracker.cleanup();

      expect(tracker.size).toBe(0);
      // Should now accept the same nonce again
      expect(tracker.checkAndRecord('nonce-1')).toBe(true);

      vi.useRealTimers();
    });

    it('should respect maxSize limit', () => {
      const smallTracker = new NonceTracker({ maxSize: 5, maxAgeMs: 60000 });

      for (let i = 0; i < 10; i++) {
        smallTracker.checkAndRecord(`nonce-${i}`);
      }

      // Should have cleaned up old entries
      expect(smallTracker.size).toBeLessThanOrEqual(10);
    });

    it('should clear all nonces', () => {
      tracker.checkAndRecord('nonce-1');
      tracker.checkAndRecord('nonce-2');
      tracker.clear();
      expect(tracker.size).toBe(0);
    });

    it('should start and stop cleanup', () => {
      vi.useFakeTimers();

      tracker.startCleanup(100);
      vi.advanceTimersByTime(200);
      tracker.stopCleanup();

      vi.useRealTimers();
    });
  });

  describe('Message Signing', () => {
    let keypair: { publicKey: Uint8Array; secretKey: Uint8Array };
    let nodeId: string;
    let baseMessage: GroupMessagePayload;

    beforeEach(() => {
      keypair = generateKeypair();
      nodeId = publicKeyToNodeId(keypair.publicKey);
      baseMessage = {
        type: 'group-message',
        groupId: 'grp-123',
        messageId: 'msg-1',
        senderId: nodeId,
        senderUsername: 'Alice',
        text: 'Hello, World!',
        sentAt: Date.now(),
      };
    });

    it('should sign a message', () => {
      const signed = signGroupMessage(baseMessage, keypair.secretKey);

      expect(signed.signature).toBeDefined();
      expect(signed.nonce).toBeDefined();
      expect(signed.text).toBe(baseMessage.text);
    });

    it('should verify a valid signature', () => {
      const signed = signGroupMessage(baseMessage, keypair.secretKey);
      const isValid = verifyGroupMessageSignature(signed, keypair.publicKey);

      expect(isValid).toBe(true);
    });

    it('should verify with hex nodeId', () => {
      const signed = signGroupMessage(baseMessage, keypair.secretKey);
      const isValid = verifyGroupMessageSignature(signed, nodeId);

      expect(isValid).toBe(true);
    });

    it('should reject tampered message', () => {
      const signed = signGroupMessage(baseMessage, keypair.secretKey);

      // Tamper with the text
      const tampered = { ...signed, text: 'Tampered!' };
      const isValid = verifyGroupMessageSignature(tampered, keypair.publicKey);

      expect(isValid).toBe(false);
    });

    it('should reject wrong public key', () => {
      const signed = signGroupMessage(baseMessage, keypair.secretKey);

      // Use different keypair to verify
      const otherKeypair = generateKeypair();
      const isValid = verifyGroupMessageSignature(signed, otherKeypair.publicKey);

      expect(isValid).toBe(false);
    });

    it('should reject message without signature', () => {
      const isValid = verifyGroupMessageSignature(baseMessage, keypair.publicKey);
      expect(isValid).toBe(false);
    });

    it('should preserve existing nonce', () => {
      const messageWithNonce = { ...baseMessage, nonce: 'existing-nonce' };
      const signed = signGroupMessage(messageWithNonce, keypair.secretKey);

      expect(signed.nonce).toBe('existing-nonce');
    });
  });

  describe('createSignableData', () => {
    it('should create deterministic data', () => {
      const message: GroupMessagePayload = {
        type: 'group-message',
        groupId: 'grp-123',
        messageId: 'msg-1',
        senderId: 'node-1',
        senderUsername: 'Alice',
        text: 'Hello',
        sentAt: 1234567890,
        nonce: 'test-nonce',
      };

      const data1 = createSignableData(message);
      const data2 = createSignableData(message);

      expect(Array.from(data1)).toEqual(Array.from(data2));
    });

    it('should produce different data for different messages', () => {
      const message1: GroupMessagePayload = {
        type: 'group-message',
        groupId: 'grp-123',
        messageId: 'msg-1',
        senderId: 'node-1',
        senderUsername: 'Alice',
        text: 'Hello',
        sentAt: 1234567890,
      };

      const message2: GroupMessagePayload = {
        ...message1,
        text: 'Different',
      };

      const data1 = createSignableData(message1);
      const data2 = createSignableData(message2);

      expect(Array.from(data1)).not.toEqual(Array.from(data2));
    });
  });

  describe('Hex Utilities', () => {
    it('should convert Uint8Array to hex', () => {
      const arr = new Uint8Array([0x00, 0x0f, 0xff, 0x10]);
      const hex = uint8ArrayToHex(arr);
      expect(hex).toBe('000fff10');
    });

    it('should convert hex to Uint8Array', () => {
      const arr = hexToUint8Array('000fff10');
      expect(Array.from(arr)).toEqual([0x00, 0x0f, 0xff, 0x10]);
    });

    it('should roundtrip correctly', () => {
      const original = new Uint8Array([1, 2, 3, 255, 0, 128]);
      const hex = uint8ArrayToHex(original);
      const result = hexToUint8Array(hex);
      expect(Array.from(result)).toEqual(Array.from(original));
    });
  });
});
