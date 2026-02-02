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

function waitForMessageOfType(ws: WebSocket, type: string): Promise<SignalingMessage> {
  return new Promise((resolve) => {
    const handler = (data: WebSocket.RawData) => {
      const msg = JSON.parse(data.toString());
      if (msg.type === type) {
        ws.off('message', handler);
        resolve(msg);
      }
    };
    ws.on('message', handler);
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

    const participantsPromise = waitForMessageOfType(ws, 'participants');
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
    await waitForMessageOfType(ws1, 'participants');

    const p1 = waitForMessageOfType(ws1, 'participants');
    const p2 = waitForMessageOfType(ws2, 'participants');
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
    await waitForMessageOfType(ws1, 'participants');
    send(ws2, { type: 'register', nodeId: 'bbb', username: 'bob' });
    await waitForMessageOfType(ws1, 'participants');
    await waitForMessageOfType(ws2, 'participants');

    const p = waitForMessageOfType(ws1, 'participants');
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
    await waitForMessageOfType(ws1, 'participants');
    send(ws2, { type: 'register', nodeId: 'bbb', username: 'bob' });
    await waitForMessageOfType(ws1, 'participants');
    await waitForMessageOfType(ws2, 'participants');

    const p = waitForMessageOfType(ws2, 'signal');
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
    await waitForMessageOfType(ws1, 'participants');

    const p = waitForMessageOfType(ws1, 'error');
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

  it('broadcasts presence join and leave events', async () => {
    const ws1 = await connectClient();
    const ws2 = await connectClient();
    clients.push(ws1, ws2);

    send(ws1, { type: 'register', nodeId: 'aaa', username: 'alice' });
    await waitForMessageOfType(ws1, 'participants');

    const presenceJoin = waitForMessageOfType(ws1, 'presence');
    send(ws2, { type: 'register', nodeId: 'bbb', username: 'bob' });

    const joinMsg = await presenceJoin;
    expect(joinMsg.action).toBe('join');
    expect(joinMsg.nodeId).toBe('bbb');
    expect(joinMsg.username).toBe('bob');

    const presenceLeave = waitForMessageOfType(ws1, 'presence');
    ws2.close();

    const leaveMsg = await presenceLeave;
    expect(leaveMsg.action).toBe('leave');
    expect(leaveMsg.nodeId).toBe('bbb');
  });

  it('broadcasts heartbeat to other nodes', async () => {
    const ws1 = await connectClient();
    const ws2 = await connectClient();
    clients.push(ws1, ws2);

    send(ws1, { type: 'register', nodeId: 'aaa', username: 'alice' });
    await waitForMessageOfType(ws1, 'participants');
    send(ws2, { type: 'register', nodeId: 'bbb', username: 'bob' });
    await waitForMessageOfType(ws2, 'participants');

    const p = waitForMessageOfType(ws2, 'heartbeat');
    send(ws1, { type: 'heartbeat' });

    const msg = await p;
    expect(msg.type).toBe('heartbeat');
    expect(msg.from).toBe('aaa');
  });
});
