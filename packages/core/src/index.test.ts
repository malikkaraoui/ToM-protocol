import { describe, expect, it } from 'vitest';
import { TOM_PROTOCOL_VERSION } from './index.js';

describe('tom-protocol core', () => {
  it('exports protocol version', () => {
    expect(TOM_PROTOCOL_VERSION).toBe('0.0.1');
  });
});
