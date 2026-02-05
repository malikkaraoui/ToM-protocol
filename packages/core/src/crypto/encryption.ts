/**
 * End-to-End Encryption (Story 6.1)
 *
 * Provides E2E encryption using TweetNaCl.js:
 * - nacl.box (x25519-xsalsa20-poly1305) for asymmetric encryption
 * - Each node has a separate Curve25519 keypair for encryption
 * - Messages are encrypted so only the recipient can read them
 * - Relays only see routing metadata, never content
 *
 * @see architecture.md for security requirements
 */

import nacl from 'tweetnacl';
import { hexToUint8Array, uint8ArrayToHex } from '../groups/group-security.js';

// ============================================
// Types
// ============================================

export interface EncryptionKeypair {
  publicKey: Uint8Array;
  secretKey: Uint8Array;
}

export interface EncryptedPayload {
  /** Encrypted ciphertext (hex encoded) */
  ciphertext: string;
  /** Random nonce used for encryption (hex encoded) */
  nonce: string;
  /** Sender's ephemeral public key for this message (hex encoded) */
  ephemeralPublicKey: string;
}

// ============================================
// Key Generation
// ============================================

/**
 * Generate a new Curve25519 keypair for encryption
 */
export function generateEncryptionKeypair(): EncryptionKeypair {
  const keypair = nacl.box.keyPair();
  return {
    publicKey: keypair.publicKey,
    secretKey: keypair.secretKey,
  };
}

/**
 * Convert encryption public key to hex string for transmission
 */
export function encryptionKeyToHex(publicKey: Uint8Array): string {
  return uint8ArrayToHex(publicKey);
}

/**
 * Convert hex string back to encryption public key
 */
export function hexToEncryptionKey(hex: string): Uint8Array {
  return hexToUint8Array(hex);
}

// ============================================
// Encryption / Decryption
// ============================================

/**
 * Encrypt a message payload for a specific recipient
 *
 * Uses ephemeral key exchange: generates a new keypair for each message,
 * performs X25519 key exchange with recipient's public key, then encrypts
 * with XSalsa20-Poly1305.
 *
 * @param payload - The payload to encrypt (will be JSON serialized)
 * @param recipientPublicKey - Recipient's Curve25519 public key
 * @returns Encrypted payload with ciphertext, nonce, and ephemeral public key
 */
export function encryptPayload(payload: unknown, recipientPublicKey: Uint8Array): EncryptedPayload {
  // Generate ephemeral keypair for this message (forward secrecy)
  const ephemeralKeypair = nacl.box.keyPair();

  // Serialize payload to bytes
  const plaintext = new TextEncoder().encode(JSON.stringify(payload));

  // Generate random nonce
  const nonce = nacl.randomBytes(nacl.box.nonceLength);

  // Encrypt using ephemeral secret key and recipient's public key
  const ciphertext = nacl.box(plaintext, nonce, recipientPublicKey, ephemeralKeypair.secretKey);

  return {
    ciphertext: uint8ArrayToHex(ciphertext),
    nonce: uint8ArrayToHex(nonce),
    ephemeralPublicKey: uint8ArrayToHex(ephemeralKeypair.publicKey),
  };
}

/**
 * Decrypt an encrypted payload
 *
 * @param encrypted - The encrypted payload
 * @param recipientSecretKey - Recipient's Curve25519 secret key
 * @returns Decrypted payload (parsed from JSON), or null if decryption fails
 */
export function decryptPayload<T = unknown>(encrypted: EncryptedPayload, recipientSecretKey: Uint8Array): T | null {
  try {
    const ciphertext = hexToUint8Array(encrypted.ciphertext);
    const nonce = hexToUint8Array(encrypted.nonce);
    const senderPublicKey = hexToUint8Array(encrypted.ephemeralPublicKey);

    // Decrypt using sender's ephemeral public key and our secret key
    const plaintext = nacl.box.open(ciphertext, nonce, senderPublicKey, recipientSecretKey);

    if (!plaintext) {
      return null; // Decryption failed (wrong key or tampered)
    }

    // Parse JSON payload
    const decoded = new TextDecoder().decode(plaintext);
    return JSON.parse(decoded) as T;
  } catch {
    return null; // Invalid format or parsing error
  }
}

// ============================================
// Type Guards
// ============================================

/**
 * Check if a payload is encrypted
 */
export function isEncryptedPayload(payload: unknown): payload is EncryptedPayload {
  if (typeof payload !== 'object' || payload === null) return false;
  const p = payload as Record<string, unknown>;
  return typeof p.ciphertext === 'string' && typeof p.nonce === 'string' && typeof p.ephemeralPublicKey === 'string';
}

// ============================================
// Key Storage Helpers
// ============================================

const ENCRYPTION_KEY_STORAGE_KEY = 'tom-encryption-keypair';

/**
 * Store encryption keypair (browser localStorage)
 */
export function storeEncryptionKeypair(keypair: EncryptionKeypair): void {
  if (typeof localStorage === 'undefined') return;

  const data = {
    publicKey: uint8ArrayToHex(keypair.publicKey),
    secretKey: uint8ArrayToHex(keypair.secretKey),
  };
  localStorage.setItem(ENCRYPTION_KEY_STORAGE_KEY, JSON.stringify(data));
}

/**
 * Load encryption keypair from storage
 */
export function loadEncryptionKeypair(): EncryptionKeypair | null {
  if (typeof localStorage === 'undefined') return null;

  const stored = localStorage.getItem(ENCRYPTION_KEY_STORAGE_KEY);
  if (!stored) return null;

  try {
    const data = JSON.parse(stored);
    return {
      publicKey: hexToUint8Array(data.publicKey),
      secretKey: hexToUint8Array(data.secretKey),
    };
  } catch {
    return null;
  }
}

/**
 * Get or create encryption keypair (with persistence)
 */
export function getOrCreateEncryptionKeypair(): EncryptionKeypair {
  const existing = loadEncryptionKeypair();
  if (existing) return existing;

  const keypair = generateEncryptionKeypair();
  storeEncryptionKeypair(keypair);
  return keypair;
}
