import { describe, expect, it } from 'vitest';
import {
  type MessageReadHandler,
  type MessageStatus,
  type MessageStatusChangedHandler,
  type MessageStatusEntry,
  TomClient,
} from './index.js';

describe('tom-sdk exports', () => {
  it('exports MessageStatus type', () => {
    // Type assertion - if this compiles, the type is exported correctly
    const status: MessageStatus = 'read';
    expect(status).toBe('read');
  });

  it('exports MessageStatusEntry type', () => {
    // Type assertion - if this compiles, the type is exported correctly
    const entry: MessageStatusEntry = {
      messageId: 'test',
      to: 'recipient',
      status: 'pending',
      timestamps: { pending: Date.now() },
    };
    expect(entry.status).toBe('pending');
  });

  it('exports MessageStatusChangedHandler type', () => {
    // Type assertion - if this compiles, the type is exported correctly
    const handler: MessageStatusChangedHandler = (_id, _prev, _new) => {};
    expect(typeof handler).toBe('function');
  });

  it('exports MessageReadHandler type', () => {
    // Type assertion - if this compiles, the type is exported correctly
    const handler: MessageReadHandler = (_id, _readAt, _from) => {};
    expect(typeof handler).toBe('function');
  });

  it('exports TomClient class', () => {
    expect(TomClient).toBeDefined();
    expect(typeof TomClient).toBe('function');
  });
});

describe('TomClient API', () => {
  it('has markAsRead method', () => {
    const client = new TomClient({
      signalingUrl: 'ws://localhost:3001',
      username: 'test',
    });
    expect(typeof client.markAsRead).toBe('function');
  });

  it('has onMessageStatusChanged method', () => {
    const client = new TomClient({
      signalingUrl: 'ws://localhost:3001',
      username: 'test',
    });
    expect(typeof client.onMessageStatusChanged).toBe('function');
  });

  it('has onMessageRead method', () => {
    const client = new TomClient({
      signalingUrl: 'ws://localhost:3001',
      username: 'test',
    });
    expect(typeof client.onMessageRead).toBe('function');
  });

  it('has getMessageStatus method', () => {
    const client = new TomClient({
      signalingUrl: 'ws://localhost:3001',
      username: 'test',
    });
    expect(typeof client.getMessageStatus).toBe('function');
  });

  it('getMessageStatus returns undefined for unknown message', () => {
    const client = new TomClient({
      signalingUrl: 'ws://localhost:3001',
      username: 'test',
    });
    expect(client.getMessageStatus('unknown-id')).toBeUndefined();
  });

  it('markAsRead returns false for unknown message', () => {
    const client = new TomClient({
      signalingUrl: 'ws://localhost:3001',
      username: 'test',
    });
    // Not connected, so no messageOrigins - should return false
    expect(client.markAsRead('unknown-id')).toBe(false);
  });
});
