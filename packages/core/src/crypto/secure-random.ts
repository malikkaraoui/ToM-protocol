/**
 * Secure Random Utilities (Security Fix - CVSS 7.5)
 *
 * Provides cryptographically secure random generation for IDs and nonces.
 * Replaces weak Math.random() usage with crypto APIs.
 *
 * Works in both browser (crypto.getRandomValues) and Node.js (crypto.randomBytes).
 *
 * @see https://github.com/malikkaraoui/ToM-protocol/issues/28
 */

/**
 * Generate cryptographically secure random bytes
 *
 * @param length - Number of random bytes to generate
 * @returns Uint8Array of random bytes
 * @throws Error if no secure random source is available
 */
export function secureRandomBytes(length: number): Uint8Array {
  // Browser environment
  if (typeof crypto !== 'undefined' && typeof crypto.getRandomValues === 'function') {
    const bytes = new Uint8Array(length);
    crypto.getRandomValues(bytes);
    return bytes;
  }

  // Node.js environment
  if (typeof globalThis !== 'undefined') {
    try {
      // Dynamic import to avoid bundling issues
      // eslint-disable-next-line @typescript-eslint/no-require-imports
      const nodeCrypto = require('node:crypto') as typeof import('node:crypto');
      return new Uint8Array(nodeCrypto.randomBytes(length));
    } catch {
      // Not in Node.js or crypto not available
    }
  }

  throw new Error('No cryptographically secure random source available');
}

/**
 * Generate a cryptographically secure random hex string
 *
 * @param length - Number of hex characters (will use length/2 bytes)
 * @returns Random hex string of specified length
 */
export function secureRandomHex(length: number): string {
  const bytes = secureRandomBytes(Math.ceil(length / 2));
  const hex = Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('');
  return hex.slice(0, length);
}

/**
 * Generate a cryptographically secure UUID v4
 *
 * Uses crypto.randomUUID() if available, otherwise generates manually
 * using secure random bytes.
 *
 * @returns UUID string in format xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx
 */
export function secureRandomUUID(): string {
  // Use native if available
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
    return crypto.randomUUID();
  }

  // Generate UUID v4 manually using secure random bytes
  const bytes = secureRandomBytes(16);

  // Set version (4) and variant (10xx) bits per RFC 4122
  bytes[6] = (bytes[6] & 0x0f) | 0x40; // Version 4
  bytes[8] = (bytes[8] & 0x3f) | 0x80; // Variant 10xx

  // Format as UUID string
  const hex = Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('');

  return [hex.slice(0, 8), hex.slice(8, 12), hex.slice(12, 16), hex.slice(16, 20), hex.slice(20, 32)].join('-');
}

/**
 * Generate a secure ID with prefix and timestamp
 *
 * Format: {prefix}-{timestamp}-{randomHex}
 *
 * @param prefix - ID prefix (e.g., 'subnet', 'gossip', 'grp')
 * @param randomLength - Length of random hex suffix (default: 8)
 * @returns Secure ID string
 */
export function secureId(prefix: string, randomLength = 8): string {
  return `${prefix}-${Date.now()}-${secureRandomHex(randomLength)}`;
}
