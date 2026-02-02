import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import WebSocket from 'ws';
import type { SignalingMessage } from './index.js';
import { createSignalingServer } from './server.js';

const PORT = 9123;

function connectClient(): Promise<WebSocket> {
  return new Promise((resolve, reject) => {
    const ws = new WebSocket(`ws://localhost:${PORT}`);
    ws.on('open', () => resolve(ws));
    ws.on('error', reject);
  });
}

function waitForMessage(ws: WebSocket): Promise<SignalingMessage> {
  return new Promise((resolve) => {
    ws.once('message', (data) => {
      resolve(JSON.parse(data.toString()));
    });
  });
}

function send(ws: WebSocket, msg: SignalingMessage): void {
  ws.send(JSON.stringify(msg));
}

describe('signaling server', () => {
  let close: () => void;
  const clients: WebSocket[] = [];

  beforeEach(() => {
    const server = createSignalingServer(PORT);
    close = server.close;
  });

  afterEach(() => {
    for (const c of clients) {
      if (c.readyState === WebSocket.OPEN) c.close();
    }
    clients.length = 0;
    close();
  });

  it('registers a node and broadcasts participant list', async () => {
    const ws = await connectClient();
    clients.push(ws);

    const participantsPromise = waitForMessage(ws);
    send(ws, { type: 'register', nodeId: 'aaa', username: 'alice' });

    const msg = await participantsPromise;
    expect(msg.type).toBe('participants');
    expect(msg.participants).toHaveLength(1);
    expect(msg.participants?.[0]).toEqual({ nodeId: 'aaa', username: 'alice' });
  });

  it('broadcasts updated list when second node joins', async () => {
    const ws1 = await connectClient();
    const ws2 = await connectClient();
    clients.push(ws1, ws2);

    send(ws1, { type: 'register', nodeId: 'aaa', username: 'alice' });
    await waitForMessage(ws1); // participants with 1 node

    const p1 = waitForMessage(ws1);
    const p2 = waitForMessage(ws2);
    send(ws2, { type: 'register', nodeId: 'bbb', username: 'bob' });

    const [msg1, msg2] = await Promise.all([p1, p2]);
    expect(msg1.participants).toHaveLength(2);
    expect(msg2.participants).toHaveLength(2);
  });

  it('broadcasts updated list when a node disconnects', async () => {
    const ws1 = await connectClient();
    const ws2 = await connectClient();
    clients.push(ws1, ws2);

    send(ws1, { type: 'register', nodeId: 'aaa', username: 'alice' });
    await waitForMessage(ws1);
    send(ws2, { type: 'register', nodeId: 'bbb', username: 'bob' });
    await waitForMessage(ws1); // 2 participants
    await waitForMessage(ws2);

    const p = waitForMessage(ws1);
    ws2.close();

    const msg = await p;
    expect(msg.participants).toHaveLength(1);
    expect(msg.participants?.[0].nodeId).toBe('aaa');
  });

  it('relays signal messages to target node', async () => {
    const ws1 = await connectClient();
    const ws2 = await connectClient();
    clients.push(ws1, ws2);

    send(ws1, { type: 'register', nodeId: 'aaa', username: 'alice' });
    await waitForMessage(ws1);
    send(ws2, { type: 'register', nodeId: 'bbb', username: 'bob' });
    await waitForMessage(ws1);
    await waitForMessage(ws2);

    const p = waitForMessage(ws2);
    send(ws1, { type: 'signal', from: 'aaa', to: 'bbb', payload: { sdp: 'offer-data' } });

    const msg = await p;
    expect(msg.type).toBe('signal');
    expect(msg.from).toBe('aaa');
    expect(msg.payload).toEqual({ sdp: 'offer-data' });
  });

  it('returns error for signal to unknown peer', async () => {
    const ws1 = await connectClient();
    clients.push(ws1);

    send(ws1, { type: 'register', nodeId: 'aaa', username: 'alice' });
    await waitForMessage(ws1);

    const p = waitForMessage(ws1);
    send(ws1, { type: 'signal', from: 'aaa', to: 'unknown', payload: {} });

    const msg = await p;
    expect(msg.type).toBe('error');
    expect(msg.error).toBe('peer not found');
  });

  it('returns error for invalid JSON', async () => {
    const ws = await connectClient();
    clients.push(ws);

    const p = waitForMessage(ws);
    ws.send('not json');

    const msg = await p;
    expect(msg.type).toBe('error');
    expect(msg.error).toBe('invalid JSON');
  });
});
