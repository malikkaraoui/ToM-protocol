import { describe, expect, it } from 'vitest';
import type { MessageEnvelope } from './envelope.js';

describe('MessageEnvelope', () => {
  it('can construct a valid envelope object', () => {
    const envelope: MessageEnvelope = {
      id: 'msg-001',
      from: 'a'.repeat(64),
      to: 'b'.repeat(64),
      via: ['c'.repeat(64)],
      type: 'chat',
      payload: { text: 'hello' },
      timestamp: Date.now(),
      signature: 'deadbeef',
    };

    expect(envelope.id).toBe('msg-001');
    expect(envelope.from).toHaveLength(64);
    expect(envelope.to).toHaveLength(64);
    expect(envelope.via).toHaveLength(1);
    expect(envelope.type).toBe('chat');
    expect(envelope.payload).toEqual({ text: 'hello' });
    expect(typeof envelope.timestamp).toBe('number');
    expect(envelope.signature).toBe('deadbeef');
  });
});
