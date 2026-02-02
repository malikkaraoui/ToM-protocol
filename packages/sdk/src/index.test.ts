import { describe, expect, it } from 'vitest';
import { TOM_PROTOCOL_VERSION } from './index.js';

describe('tom-sdk', () => {
  it('re-exports protocol version from core', () => {
    expect(TOM_PROTOCOL_VERSION).toBe('0.0.1');
  });
});
