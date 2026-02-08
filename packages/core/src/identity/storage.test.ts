/**
 * Tests for Identity Storage
 *
 * Validates storage adapters for persisting node identity.
 */

import { beforeEach, describe, expect, it, vi } from 'vitest';
import { FileStorageAdapter, LocalStorageAdapter, MemoryStorage } from './storage.js';

describe('MemoryStorage', () => {
  let storage: MemoryStorage;

  beforeEach(() => {
    storage = new MemoryStorage();
  });

  it('should return null when no identity stored', async () => {
    const identity = await storage.load();
    expect(identity).toBeNull();
  });

  it('should save and load identity correctly', async () => {
    const mockIdentity = {
      publicKey: new Uint8Array([1, 2, 3, 4, 5, 6, 7, 8]),
      secretKey: new Uint8Array([10, 20, 30, 40, 50, 60, 70, 80]),
    };

    await storage.save(mockIdentity);
    const loaded = await storage.load();

    expect(loaded).not.toBeNull();
    expect(Array.from(loaded!.publicKey)).toEqual(Array.from(mockIdentity.publicKey));
    expect(Array.from(loaded!.secretKey)).toEqual(Array.from(mockIdentity.secretKey));
  });

  it('should overwrite previous identity on save', async () => {
    const identity1 = {
      publicKey: new Uint8Array([1, 2, 3, 4]),
      secretKey: new Uint8Array([5, 6, 7, 8]),
    };
    const identity2 = {
      publicKey: new Uint8Array([10, 20, 30, 40]),
      secretKey: new Uint8Array([50, 60, 70, 80]),
    };

    await storage.save(identity1);
    await storage.save(identity2);
    const loaded = await storage.load();

    expect(Array.from(loaded!.publicKey)).toEqual(Array.from(identity2.publicKey));
    expect(Array.from(loaded!.secretKey)).toEqual(Array.from(identity2.secretKey));
  });

  it('should preserve identity bytes exactly', async () => {
    // Test with all byte values 0-255
    const publicKey = new Uint8Array(32);
    const secretKey = new Uint8Array(64);
    for (let i = 0; i < 32; i++) {
      publicKey[i] = i * 8;
      secretKey[i] = i * 2;
      secretKey[i + 32] = 255 - i * 2;
    }

    await storage.save({ publicKey, secretKey });
    const loaded = await storage.load();

    expect(loaded).not.toBeNull();
    expect(loaded!.publicKey.length).toBe(32);
    expect(loaded!.secretKey.length).toBe(64);

    for (let i = 0; i < 32; i++) {
      expect(loaded!.publicKey[i]).toBe(publicKey[i]);
      expect(loaded!.secretKey[i]).toBe(secretKey[i]);
      expect(loaded!.secretKey[i + 32]).toBe(secretKey[i + 32]);
    }
  });

  it('should handle empty keys', async () => {
    const identity = {
      publicKey: new Uint8Array(0),
      secretKey: new Uint8Array(0),
    };

    await storage.save(identity);
    const loaded = await storage.load();

    expect(loaded).not.toBeNull();
    expect(loaded!.publicKey.length).toBe(0);
    expect(loaded!.secretKey.length).toBe(0);
  });
});

describe('LocalStorageAdapter', () => {
  let adapter: LocalStorageAdapter;
  let mockStorage: { [key: string]: string };

  beforeEach(() => {
    mockStorage = {};

    // Mock localStorage
    vi.stubGlobal('localStorage', {
      getItem: (key: string) => mockStorage[key] || null,
      setItem: (key: string, value: string) => {
        mockStorage[key] = value;
      },
    });

    adapter = new LocalStorageAdapter();
  });

  it('should return null when localStorage is empty', async () => {
    const identity = await adapter.load();
    expect(identity).toBeNull();
  });

  it('should save identity to localStorage', async () => {
    const mockIdentity = {
      publicKey: new Uint8Array([1, 2, 3, 4]),
      secretKey: new Uint8Array([5, 6, 7, 8]),
    };

    await adapter.save(mockIdentity);

    expect(mockStorage['tom-identity']).toBeDefined();
    const parsed = JSON.parse(mockStorage['tom-identity']);
    expect(parsed.publicKey).toBe('01020304');
    expect(parsed.secretKey).toBe('05060708');
  });

  it('should load identity from localStorage', async () => {
    mockStorage['tom-identity'] = JSON.stringify({
      publicKey: 'aabbccdd',
      secretKey: '11223344',
    });

    const loaded = await adapter.load();

    expect(loaded).not.toBeNull();
    expect(Array.from(loaded!.publicKey)).toEqual([0xaa, 0xbb, 0xcc, 0xdd]);
    expect(Array.from(loaded!.secretKey)).toEqual([0x11, 0x22, 0x33, 0x44]);
  });

  it('should round-trip identity correctly', async () => {
    const mockIdentity = {
      publicKey: new Uint8Array([0xde, 0xad, 0xbe, 0xef]),
      secretKey: new Uint8Array([0xca, 0xfe, 0xba, 0xbe]),
    };

    await adapter.save(mockIdentity);
    const loaded = await adapter.load();

    expect(Array.from(loaded!.publicKey)).toEqual(Array.from(mockIdentity.publicKey));
    expect(Array.from(loaded!.secretKey)).toEqual(Array.from(mockIdentity.secretKey));
  });
});

describe('FileStorageAdapter', () => {
  it('should construct with default path', () => {
    const adapter = new FileStorageAdapter();
    // Just verify it doesn't throw
    expect(adapter).toBeInstanceOf(FileStorageAdapter);
  });

  it('should construct with custom path', () => {
    const adapter = new FileStorageAdapter('/custom/path/identity.json');
    expect(adapter).toBeInstanceOf(FileStorageAdapter);
  });

  it('should return null when file does not exist', async () => {
    const adapter = new FileStorageAdapter('/nonexistent/path/identity.json');
    const identity = await adapter.load();
    expect(identity).toBeNull();
  });
});

describe('hex conversion', () => {
  // These test the internal toHex/fromHex functions indirectly through MemoryStorage

  it('should handle all byte values', async () => {
    const storage = new MemoryStorage();

    // Create array with all byte values 0-255
    const allBytes = new Uint8Array(256);
    for (let i = 0; i < 256; i++) {
      allBytes[i] = i;
    }

    await storage.save({ publicKey: allBytes, secretKey: allBytes });
    const loaded = await storage.load();

    expect(loaded).not.toBeNull();
    for (let i = 0; i < 256; i++) {
      expect(loaded!.publicKey[i]).toBe(i);
    }
  });

  it('should preserve leading zeros', async () => {
    const storage = new MemoryStorage();
    const identity = {
      publicKey: new Uint8Array([0, 0, 0, 1]),
      secretKey: new Uint8Array([0, 15, 16, 255]),
    };

    await storage.save(identity);
    const loaded = await storage.load();

    expect(Array.from(loaded!.publicKey)).toEqual([0, 0, 0, 1]);
    expect(Array.from(loaded!.secretKey)).toEqual([0, 15, 16, 255]);
  });

  it('should handle maximum byte value', async () => {
    const storage = new MemoryStorage();
    const identity = {
      publicKey: new Uint8Array([255, 255, 255, 255]),
      secretKey: new Uint8Array([255]),
    };

    await storage.save(identity);
    const loaded = await storage.load();

    expect(Array.from(loaded!.publicKey)).toEqual([255, 255, 255, 255]);
    expect(Array.from(loaded!.secretKey)).toEqual([255]);
  });
});
