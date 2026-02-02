import { TOM_PROTOCOL_VERSION } from 'tom-sdk';
import { describe, expect, it } from 'vitest';

describe('tom-demo', () => {
  it('can import tom-sdk dependency', () => {
    expect(TOM_PROTOCOL_VERSION).toBe('0.0.1');
  });
});
