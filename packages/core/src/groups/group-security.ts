/**
 * Group Security (Story 4.6 - Security Hardening)
 *
 * Provides cryptographic security for group messages:
 * - Ed25519 signatures for message authentication
 * - Nonce-based anti-replay protection
 * - Signature verification utilities
 *
 * @see architecture.md for security requirements
 */

import { signData, verifySignature } from '../identity/index.js';
import type { NodeId } from '../identity/index.js';
import type { GroupMessagePayload } from './group-types.js';

// ============================================
// Nonce Generation & Tracking
// ============================================

/**
 * Generate a random nonce for anti-replay protection
 */
export function generateNonce(): string {
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
    return crypto.randomUUID();
  }
  // Fallback for environments without crypto.randomUUID
  const hex = () => Math.floor(Math.random() * 16).toString(16);
  return Array.from({ length: 32 }, hex).join('');
}

/**
 * Nonce tracker for anti-replay protection
 * Tracks seen nonces with TTL to prevent memory exhaustion
 */
export class NonceTracker {
  private seenNonces = new Map<string, number>(); // nonce -> timestamp
  private maxAge: number;
  private maxSize: number;
  private cleanupInterval: ReturnType<typeof setInterval> | null = null;

  constructor(options: { maxAgeMs?: number; maxSize?: number } = {}) {
    this.maxAge = options.maxAgeMs ?? 5 * 60 * 1000; // 5 minutes default
    this.maxSize = options.maxSize ?? 10000; // Max 10k nonces
  }

  /**
   * Check if a nonce has been seen before
   * Returns true if nonce is new (valid), false if replay detected
   */
  checkAndRecord(nonce: string): boolean {
    if (this.seenNonces.has(nonce)) {
      return false; // Replay detected
    }

    // Cleanup if at capacity
    if (this.seenNonces.size >= this.maxSize) {
      this.cleanup();
    }

    this.seenNonces.set(nonce, Date.now());
    return true;
  }

  /**
   * Remove expired nonces
   */
  cleanup(): void {
    const now = Date.now();
    for (const [nonce, timestamp] of this.seenNonces) {
      if (now - timestamp > this.maxAge) {
        this.seenNonces.delete(nonce);
      }
    }
  }

  /**
   * Start periodic cleanup
   */
  startCleanup(intervalMs = 60_000): void {
    if (this.cleanupInterval) return;
    this.cleanupInterval = setInterval(() => this.cleanup(), intervalMs);
  }

  /**
   * Stop periodic cleanup
   */
  stopCleanup(): void {
    if (this.cleanupInterval) {
      clearInterval(this.cleanupInterval);
      this.cleanupInterval = null;
    }
  }

  /**
   * Get current size
   */
  get size(): number {
    return this.seenNonces.size;
  }

  /**
   * Clear all nonces
   */
  clear(): void {
    this.seenNonces.clear();
  }
}

// ============================================
// Message Signing
// ============================================

/**
 * Create the canonical data to sign for a group message
 * Includes all fields that should be authenticated
 */
export function createSignableData(message: GroupMessagePayload): Uint8Array {
  // Create deterministic string of critical fields
  const canonical = JSON.stringify({
    type: message.type,
    groupId: message.groupId,
    messageId: message.messageId,
    senderId: message.senderId,
    text: message.text,
    sentAt: message.sentAt,
    nonce: message.nonce,
  });
  return new TextEncoder().encode(canonical);
}

/**
 * Sign a group message
 * @param message - The message to sign
 * @param secretKey - Sender's Ed25519 secret key
 * @returns The message with signature added
 */
export function signGroupMessage(message: GroupMessagePayload, secretKey: Uint8Array): GroupMessagePayload {
  // Add nonce if not present
  const messageWithNonce = {
    ...message,
    nonce: message.nonce ?? generateNonce(),
  };

  const data = createSignableData(messageWithNonce);
  const signature = signData(secretKey, data);

  return {
    ...messageWithNonce,
    signature: uint8ArrayToHex(signature),
  };
}

/**
 * Verify a group message signature
 * @param message - The message to verify
 * @param senderPublicKey - Sender's Ed25519 public key (or hex-encoded nodeId)
 * @returns true if signature is valid
 */
export function verifyGroupMessageSignature(
  message: GroupMessagePayload,
  senderPublicKey: Uint8Array | string,
): boolean {
  if (!message.signature) {
    return false; // No signature present
  }

  try {
    const publicKey = typeof senderPublicKey === 'string' ? hexToUint8Array(senderPublicKey) : senderPublicKey;

    const data = createSignableData(message);
    const signature = hexToUint8Array(message.signature);

    return verifySignature(publicKey, data, signature);
  } catch {
    return false; // Invalid format
  }
}

// ============================================
// Hex Encoding Utilities
// ============================================

export function uint8ArrayToHex(arr: Uint8Array): string {
  return Array.from(arr)
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('');
}

export function hexToUint8Array(hex: string): Uint8Array {
  const matches = hex.match(/.{1,2}/g);
  if (!matches) {
    throw new Error('Invalid hex string');
  }
  return new Uint8Array(matches.map((byte) => Number.parseInt(byte, 16)));
}

// ============================================
// Security Configuration
// ============================================

/** Default security settings */
export const GROUP_SECURITY_DEFAULTS = {
  /** Require signatures on all messages */
  requireSignatures: false, // Off by default for backwards compatibility
  /** Require nonces for anti-replay */
  requireNonces: false, // Off by default for backwards compatibility
  /** Max age for nonce tracking (ms) */
  nonceMaxAgeMs: 5 * 60 * 1000, // 5 minutes
  /** Max nonces to track per hub */
  nonceMaxSize: 10000,
};

export interface GroupSecurityConfig {
  requireSignatures?: boolean;
  requireNonces?: boolean;
  nonceMaxAgeMs?: number;
  nonceMaxSize?: number;
}
