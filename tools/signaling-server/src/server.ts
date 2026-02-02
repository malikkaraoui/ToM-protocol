// TEMPORARY: Bootstrap signaling server (ADR-002) — marked for elimination

import { WebSocketServer } from 'ws';
import type WebSocket from 'ws';
import type { Participant, SignalingMessage } from './index.js';

interface ConnectedNode {
  ws: WebSocket;
  nodeId: string;
  username: string;
}

export function createSignalingServer(port: number): { wss: WebSocketServer; close: () => void } {
  const wss = new WebSocketServer({ port });
  const nodes = new Map<string, ConnectedNode>();

  function broadcastParticipants(): void {
    const participants: Participant[] = Array.from(nodes.values()).map((n) => ({
      nodeId: n.nodeId,
      username: n.username,
    }));

    const message: SignalingMessage = {
      type: 'participants',
      participants,
    };

    const payload = JSON.stringify(message);
    for (const node of nodes.values()) {
      if (node.ws.readyState === node.ws.OPEN) {
        node.ws.send(payload);
      }
    }
  }

  wss.on('connection', (ws: WebSocket) => {
    let registeredNodeId: string | null = null;

    ws.on('message', (raw: Buffer) => {
      let msg: SignalingMessage;
      try {
        msg = JSON.parse(raw.toString());
      } catch {
        ws.send(JSON.stringify({ type: 'error', error: 'invalid JSON' } satisfies SignalingMessage));
        return;
      }

      if (msg.type === 'register') {
        if (!msg.nodeId || !msg.username) {
          ws.send(JSON.stringify({ type: 'error', error: 'missing nodeId or username' } satisfies SignalingMessage));
          return;
        }
        registeredNodeId = msg.nodeId;
        nodes.set(msg.nodeId, { ws, nodeId: msg.nodeId, username: msg.username });
        broadcastParticipants();
        return;
      }

      if (msg.type === 'signal') {
        if (!msg.to || !msg.from) {
          ws.send(JSON.stringify({ type: 'error', error: 'missing to or from' } satisfies SignalingMessage));
          return;
        }
        const target = nodes.get(msg.to);
        if (!target || target.ws.readyState !== target.ws.OPEN) {
          ws.send(JSON.stringify({ type: 'error', error: 'peer not found' } satisfies SignalingMessage));
          return;
        }
        // Forward without inspecting — relay only
        target.ws.send(JSON.stringify(msg));
        return;
      }
    });

    ws.on('close', () => {
      if (registeredNodeId) {
        nodes.delete(registeredNodeId);
        broadcastParticipants();
      }
    });
  });

  return {
    wss,
    close: () => {
      for (const node of nodes.values()) {
        node.ws.close();
      }
      nodes.clear();
      wss.close();
    },
  };
}
