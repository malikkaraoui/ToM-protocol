/**
 * Tests for Secure Random Utilities
 *
 * Validates cryptographically secure random generation functions.
 */

import { describe, expect, it } from 'vitest';
import { secureId, secureRandomBytes, secureRandomHex, secureRandomUUID } from './secure-random.js';

describe('secureRandomBytes', () => {
  it('should generate bytes of specified length', () => {
    const bytes = secureRandomBytes(16);
    expect(bytes).toBeInstanceOf(Uint8Array);
    expect(bytes.length).toBe(16);
  });

  it('should generate different bytes on each call', () => {
    const bytes1 = secureRandomBytes(32);
    const bytes2 = secureRandomBytes(32);

    // Convert to strings for comparison
    const str1 = Array.from(bytes1).join(',');
    const str2 = Array.from(bytes2).join(',');

    expect(str1).not.toBe(str2);
  });

  it('should handle zero length', () => {
    const bytes = secureRandomBytes(0);
    expect(bytes.length).toBe(0);
  });

  it('should handle large lengths', () => {
    const bytes = secureRandomBytes(1024);
    expect(bytes.length).toBe(1024);
  });
});

describe('secureRandomHex', () => {
  it('should generate hex string of specified length', () => {
    const hex = secureRandomHex(16);
    expect(hex.length).toBe(16);
    expect(hex).toMatch(/^[0-9a-f]+$/);
  });

  it('should generate different hex strings on each call', () => {
    const hex1 = secureRandomHex(32);
    const hex2 = secureRandomHex(32);
    expect(hex1).not.toBe(hex2);
  });

  it('should handle odd lengths', () => {
    const hex = secureRandomHex(7);
    expect(hex.length).toBe(7);
    expect(hex).toMatch(/^[0-9a-f]+$/);
  });

  it('should produce valid hex characters only', () => {
    // Generate many to ensure all possible values are valid hex
    for (let i = 0; i < 100; i++) {
      const hex = secureRandomHex(64);
      expect(hex).toMatch(/^[0-9a-f]+$/);
    }
  });
});

describe('secureRandomUUID', () => {
  it('should generate valid UUID v4 format', () => {
    const uuid = secureRandomUUID();
    // UUID v4 format: xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx
    expect(uuid).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/);
  });

  it('should generate different UUIDs on each call', () => {
    const uuid1 = secureRandomUUID();
    const uuid2 = secureRandomUUID();
    expect(uuid1).not.toBe(uuid2);
  });

  it('should have version 4 indicator', () => {
    const uuid = secureRandomUUID();
    // Version is in position 14 (after second hyphen)
    expect(uuid.charAt(14)).toBe('4');
  });

  it('should have correct variant bits', () => {
    const uuid = secureRandomUUID();
    // Variant is in position 19 (after third hyphen)
    const variant = uuid.charAt(19);
    expect(['8', '9', 'a', 'b']).toContain(variant);
  });

  it('should generate valid UUIDs consistently', () => {
    // Generate many UUIDs to ensure format is always correct
    for (let i = 0; i < 100; i++) {
      const uuid = secureRandomUUID();
      expect(uuid).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/);
    }
  });
});

describe('secureId', () => {
  it('should generate ID with correct prefix', () => {
    const id = secureId('subnet');
    expect(id.startsWith('subnet-')).toBe(true);
  });

  it('should include timestamp component', () => {
    const before = Date.now();
    const id = secureId('test');
    const after = Date.now();

    // Extract timestamp from ID (second component)
    const parts = id.split('-');
    const timestamp = Number.parseInt(parts[1], 10);

    expect(timestamp).toBeGreaterThanOrEqual(before);
    expect(timestamp).toBeLessThanOrEqual(after);
  });

  it('should include random hex suffix', () => {
    const id = secureId('grp', 8);
    const parts = id.split('-');

    // Last part should be 8 hex characters
    const randomPart = parts[2];
    expect(randomPart.length).toBe(8);
    expect(randomPart).toMatch(/^[0-9a-f]+$/);
  });

  it('should use default random length of 8', () => {
    const id = secureId('msg');
    const parts = id.split('-');
    expect(parts[2].length).toBe(8);
  });

  it('should generate different IDs even with same prefix', () => {
    const id1 = secureId('peer');
    const id2 = secureId('peer');
    expect(id1).not.toBe(id2);
  });

  it('should respect custom random length', () => {
    const id = secureId('custom', 16);
    const parts = id.split('-');
    expect(parts[2].length).toBe(16);
  });

  it('should handle various prefix formats', () => {
    const prefixes = ['msg', 'subnet', 'gossip', 'grp', 'inv', 'ack'];
    for (const prefix of prefixes) {
      const id = secureId(prefix);
      expect(id.startsWith(`${prefix}-`)).toBe(true);
      expect(id.split('-').length).toBe(3);
    }
  });
});

describe('entropy quality', () => {
  it('should produce evenly distributed bytes', () => {
    // Generate large sample
    const samples = 10000;
    const counts = new Array(256).fill(0);

    for (let i = 0; i < samples; i++) {
      const bytes = secureRandomBytes(1);
      counts[bytes[0]]++;
    }

    // Calculate chi-squared statistic
    const expected = samples / 256;
    let chiSquared = 0;
    for (const count of counts) {
      chiSquared += (count - expected) ** 2 / expected;
    }

    // Chi-squared critical value for 255 degrees of freedom at p=0.01 is ~310
    // We use a more lenient threshold for test stability
    expect(chiSquared).toBeLessThan(400);
  });

  it('should not produce repeating patterns', () => {
    const bytes = secureRandomBytes(1000);

    // Check for simple repeating patterns (e.g., every 4th byte same)
    let patternFound = true;
    for (let period = 1; period <= 10; period++) {
      patternFound = true;
      for (let i = period; i < bytes.length; i++) {
        if (bytes[i] !== bytes[i % period]) {
          patternFound = false;
          break;
        }
      }
      if (patternFound) break;
    }

    expect(patternFound).toBe(false);
  });
});
