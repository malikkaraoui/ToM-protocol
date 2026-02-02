import type { NodeIdentity } from './keypair.js';

declare const localStorage:
  | {
      getItem(key: string): string | null;
      setItem(key: string, value: string): void;
    }
  | undefined;

export interface IdentityStorage {
  save(identity: NodeIdentity): Promise<void>;
  load(): Promise<NodeIdentity | null>;
}

function toHex(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('');
}

function fromHex(hex: string): Uint8Array {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < hex.length; i += 2) {
    bytes[i / 2] = Number.parseInt(hex.substring(i, i + 2), 16);
  }
  return bytes;
}

interface StoredIdentity {
  publicKey: string;
  secretKey: string;
}

export class MemoryStorage implements IdentityStorage {
  private stored: StoredIdentity | null = null;

  async save(identity: NodeIdentity): Promise<void> {
    this.stored = {
      publicKey: toHex(identity.publicKey),
      secretKey: toHex(identity.secretKey),
    };
  }

  async load(): Promise<NodeIdentity | null> {
    if (!this.stored) return null;
    return {
      publicKey: fromHex(this.stored.publicKey),
      secretKey: fromHex(this.stored.secretKey),
    };
  }
}

const LOCAL_STORAGE_KEY = 'tom-identity';

export class LocalStorageAdapter implements IdentityStorage {
  async save(identity: NodeIdentity): Promise<void> {
    if (typeof localStorage === 'undefined') {
      throw new Error('localStorage is not available in this environment');
    }
    const data: StoredIdentity = {
      publicKey: toHex(identity.publicKey),
      secretKey: toHex(identity.secretKey),
    };
    localStorage.setItem(LOCAL_STORAGE_KEY, JSON.stringify(data));
  }

  async load(): Promise<NodeIdentity | null> {
    if (typeof localStorage === 'undefined') {
      return null;
    }
    const raw = localStorage.getItem(LOCAL_STORAGE_KEY);
    if (!raw) return null;
    const data: StoredIdentity = JSON.parse(raw);
    return {
      publicKey: fromHex(data.publicKey),
      secretKey: fromHex(data.secretKey),
    };
  }
}

export class FileStorageAdapter implements IdentityStorage {
  private filePath: string;

  constructor(filePath?: string) {
    this.filePath = filePath ?? this.defaultPath();
  }

  private defaultPath(): string {
    const home = typeof process !== 'undefined' ? process.env.HOME || process.env.USERPROFILE || '.' : '.';
    return `${home}/.tom/identity.json`;
  }

  async save(identity: NodeIdentity): Promise<void> {
    const { mkdir, writeFile } = await import('node:fs/promises');
    const { dirname } = await import('node:path');
    const data: StoredIdentity = {
      publicKey: toHex(identity.publicKey),
      secretKey: toHex(identity.secretKey),
    };
    await mkdir(dirname(this.filePath), { recursive: true });
    await writeFile(this.filePath, JSON.stringify(data, null, 2), 'utf-8');
  }

  async load(): Promise<NodeIdentity | null> {
    const { readFile } = await import('node:fs/promises');
    try {
      const raw = await readFile(this.filePath, 'utf-8');
      const data: StoredIdentity = JSON.parse(raw);
      return {
        publicKey: fromHex(data.publicKey),
        secretKey: fromHex(data.secretKey),
      };
    } catch {
      return null;
    }
  }
}
