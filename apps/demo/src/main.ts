/**
 * ToM Protocol Demo Application
 *
 * BOOTSTRAP PARTICIPATION (ADR-002, FR20, FR23)
 *
 * This browser tab acts as a persistent network node contributing to bootstrap:
 * - Creates a full ToM node with relay capability
 * - Can be assigned relay role by the network (ADR-007)
 * - Forwards messages for other nodes while active
 * - Participates in peer discovery and role assignment
 *
 * Per ADR-002, this uses temporary WebSocket signaling for bootstrap.
 * In Epic 7, bootstrap will transition to distributed DHT discovery.
 *
 * @see architecture.md#ADR-002 for bootstrap elimination roadmap
 * @see architecture.md#ADR-006 for unified node model
 */

import { TomClient } from 'tom-sdk';

const SIGNALING_URL = `ws://${window.location.hostname}:3001`;

const loginEl = document.getElementById('login') as HTMLElement;
const chatEl = document.getElementById('chat') as HTMLElement;
const usernameInput = document.getElementById('username-input') as HTMLInputElement;
const joinBtn = document.getElementById('join-btn') as HTMLElement;
const participantsEl = document.getElementById('participants') as HTMLElement;
const nodeIdEl = document.getElementById('node-id') as HTMLElement;
const messagesEl = document.getElementById('messages') as HTMLElement;
const messageInput = document.getElementById('message-input') as HTMLInputElement;
const statusBar = document.getElementById('status-bar') as HTMLElement;
const topologyStats = document.getElementById('topology-stats') as HTMLElement;
const myRoleEl = document.getElementById('my-role') as HTMLElement;

let client: TomClient | null = null;
let selectedPeer: string | null = null;
let participants: Array<{ nodeId: string; username: string }> = [];
const knownPeers = new Map<string, { username: string; online: boolean }>();

joinBtn.addEventListener('click', async () => {
  const username = usernameInput.value.trim();
  if (!username) return;

  client = new TomClient({ signalingUrl: SIGNALING_URL, username });

  client.onStatus((status, detail) => {
    statusBar.textContent = detail ? `${status}: ${detail}` : status;
    // Update stats on relay-related events
    if (status.startsWith('message:') || status.startsWith('direct-path:')) {
      renderParticipants();
    }
  });

  client.onParticipants((list) => {
    participants = list.filter((p) => p.nodeId !== client?.getNodeId());
    for (const p of participants) {
      knownPeers.set(p.nodeId, { username: p.username, online: true });
    }
    for (const [nodeId, peer] of knownPeers.entries()) {
      if (!participants.find((p) => p.nodeId === nodeId)) {
        peer.online = false;
      }
    }
    renderParticipants();
  });

  client.onMessage((envelope) => {
    const payload = envelope.payload as { text?: string };
    if (payload.text) {
      const peer = knownPeers.get(envelope.from);
      addMessage(peer?.username ?? envelope.from.slice(0, 8), payload.text, false, envelope.id);
      // Mark message as read (sends read receipt to sender)
      client?.markAsRead(envelope.id);
    }
    renderParticipants(); // Update stats after receiving message
  });

  // Enhanced status tracking: show full lifecycle
  client.onMessageStatusChanged((messageId, _prev, newStatus) => {
    updateMessageStatus(messageId, newStatus);
  });

  // Legacy ACK handler (fallback for backward compatibility)
  client.onAck((messageId) => {
    const msgEl = document.querySelector(`[data-msg-id="${messageId}"] .meta`);
    if (msgEl && msgEl.textContent === 'sent') {
      msgEl.textContent = 'delivered';
    }
  });

  // Read receipt: show "read" status
  client.onMessageRead((messageId) => {
    updateMessageStatus(messageId, 'read');
  });

  client.onPeerDiscovered((peer) => {
    knownPeers.set(peer.nodeId, { username: peer.username, online: true });
    renderParticipants();
  });

  client.onPeerDeparted((nodeId) => {
    const peer = knownPeers.get(nodeId);
    if (peer) {
      peer.online = false;
    }
    renderParticipants();
  });

  client.onPeerStale(() => {
    renderParticipants();
  });

  client.onRoleChanged((nodeId, roles) => {
    if (nodeId === client?.getNodeId()) {
      renderMyRole();
    }
    renderParticipants();
  });

  try {
    await client.connect();
    loginEl.style.display = 'none';
    chatEl.style.display = 'block';
    nodeIdEl.textContent = `Node: ${client.getNodeId().slice(0, 16)}...`;
    renderMyRole();
    // Periodic re-render to reflect heartbeat status changes
    setInterval(renderParticipants, 3000);
  } catch (err) {
    statusBar.textContent = `Connection failed: ${err}`;
  }
});

function renderParticipants(): void {
  participantsEl.innerHTML = '';

  const sortedPeers = Array.from(knownPeers.entries()).sort(([, a], [, b]) => {
    if (a.online === b.online) return 0;
    return a.online ? -1 : 1;
  });

  let onlineCount = 0;
  let offlineCount = 0;

  for (const [nodeId, peer] of sortedPeers) {
    const topology = client?.getTopologyInstance();
    const peerStatus = topology ? topology.getPeerStatus(nodeId) : peer.online ? 'online' : 'offline';
    const isOnline = peerStatus === 'online';
    const isStale = peerStatus === 'stale';

    if (isOnline || isStale) onlineCount++;
    else offlineCount++;

    const el = document.createElement('div');
    const classes = ['participant'];
    if (selectedPeer === nodeId) classes.push('active');
    if (!isOnline && !isStale) classes.push('offline');
    if (isStale) classes.push('stale');
    el.className = classes.join(' ');

    const dot = document.createElement('span');
    dot.className = `status-dot ${peerStatus}`;
    el.appendChild(dot);

    const name = document.createElement('span');
    name.textContent = peer.username;
    name.style.flex = '1';
    el.appendChild(name);

    // Role badge
    const peerRoles = client?.getPeerRoles(nodeId) ?? ['client'];
    if (peerRoles.includes('relay')) {
      const badge = document.createElement('span');
      badge.className = 'role-badge relay';
      badge.textContent = 'R';
      badge.title = 'Relay';
      el.appendChild(badge);
    }

    if (isOnline || isStale) {
      el.addEventListener('click', () => {
        selectedPeer = nodeId;
        renderParticipants();
      });
    }
    participantsEl.appendChild(el);
  }

  // Total includes self (knownPeers only has OTHER peers)
  const total = knownPeers.size + 1;

  // Relay count: other relays + self if we have relay role
  const otherRelays = client?.getTopologyInstance()?.getRelayNodes().length ?? 0;
  const myRoles = client?.getCurrentRoles() ?? [];
  const selfIsRelay = myRoles.includes('relay') ? 1 : 0;
  const relayCount = otherRelays + selfIsRelay;

  // Stats: relay ACKs received (own messages confirmed relayed) and messages forwarded for others
  const stats = client?.getRelayStats();
  const ackCount = stats?.relayAcksReceived ?? 0;
  const forwardedCount = stats?.messagesRelayed ?? 0;

  topologyStats.textContent = `${onlineCount} online · ${relayCount} relays · ${ackCount}/${stats?.ownMessagesSent ?? 0} relayed · ${forwardedCount} fwd · ${total} total`;
}

function renderMyRole(): void {
  if (!client) return;
  const roles = client.getCurrentRoles();
  const roleText = roles.join(', ');
  myRoleEl.textContent = `Role: ${roleText}`;
}

function addMessage(from: string, text: string, isSent: boolean, msgId?: string): void {
  const el = document.createElement('div');
  el.className = `message ${isSent ? 'sent' : 'received'}`;
  if (msgId) el.dataset.msgId = msgId;
  el.innerHTML = `<div>${text}</div><div class="meta">${isSent ? 'sent' : from}</div>`;
  messagesEl.appendChild(el);
  messagesEl.scrollTop = messagesEl.scrollHeight;
}

function updateMessageStatus(messageId: string, status: string): void {
  const msgEl = document.querySelector(`[data-msg-id="${messageId}"] .meta`);
  if (msgEl) {
    // Map status to display text with visual indicator
    const statusDisplay: Record<string, string> = {
      pending: '...',
      sent: 'sent',
      relayed: 'relayed',
      delivered: 'delivered',
      read: 'read ✓✓',
    };
    msgEl.textContent = statusDisplay[status] ?? status;
  }
}

async function sendMessage(): Promise<void> {
  if (!client || !selectedPeer) return;
  const text = messageInput.value.trim();
  if (!text) return;

  try {
    // Let SDK auto-select relay (no manual relay selection)
    const envelope = await client.sendMessage(selectedPeer, text);

    if (envelope) {
      addMessage('You', text, true, envelope.id);
      messageInput.value = '';
      renderParticipants(); // Update stats after sending message
    }
  } catch (err) {
    const error = err as Error;
    statusBar.textContent = `Send failed: ${error.message}`;
  }
}

usernameInput.addEventListener('keydown', (e) => {
  if (e.key === 'Enter') joinBtn.click();
});

const messageForm = document.getElementById('message-form') as HTMLFormElement;
messageForm.addEventListener('submit', (e) => {
  e.preventDefault();
  sendMessage();
});
