import { describe, expect, it } from 'vitest';
import { TomError } from './tom-error.js';
import type { TomErrorCode } from './tom-error.js';

describe('TomError', () => {
  it('extends Error', () => {
    const error = new TomError('TRANSPORT_FAILED', 'connection lost');
    expect(error).toBeInstanceOf(Error);
    expect(error).toBeInstanceOf(TomError);
  });

  it('has correct name, code, and message', () => {
    const error = new TomError('PEER_UNREACHABLE', 'peer offline');
    expect(error.name).toBe('TomError');
    expect(error.code).toBe('PEER_UNREACHABLE');
    expect(error.message).toBe('peer offline');
  });

  it('supports optional context', () => {
    const error = new TomError('INVALID_ENVELOPE', 'missing field', { field: 'from' });
    expect(error.context).toEqual({ field: 'from' });
  });

  it('context is undefined when not provided', () => {
    const error = new TomError('CRYPTO_FAILED', 'bad key');
    expect(error.context).toBeUndefined();
  });

  it('supports all error codes', () => {
    const codes: TomErrorCode[] = [
      'TRANSPORT_FAILED',
      'PEER_UNREACHABLE',
      'SIGNALING_TIMEOUT',
      'INVALID_ENVELOPE',
      'IDENTITY_MISSING',
      'RELAY_REJECTED',
      'CRYPTO_FAILED',
    ];
    for (const code of codes) {
      const error = new TomError(code, 'test');
      expect(error.code).toBe(code);
    }
  });
});
