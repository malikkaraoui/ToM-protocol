import { describe, expect, it } from 'vitest';
import { SIGNALING_SERVER_VERSION } from './index.js';

describe('signaling-server', () => {
  it('exports server version', () => {
    expect(SIGNALING_SERVER_VERSION).toBe('0.0.1');
  });
});
