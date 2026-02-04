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

import { TomClient, formatLatency } from 'tom-sdk';

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
const chatHeaderEl = document.getElementById('chat-header') as HTMLElement;
const pathToggleEl = document.getElementById('path-toggle') as HTMLInputElement;

// Path visualization toggle state (Story 4.3 - FR14)
let showPathDetails = localStorage.getItem('tom-show-path-details') === 'true';
pathToggleEl.checked = showPathDetails;
pathToggleEl.addEventListener('change', () => {
  showPathDetails = pathToggleEl.checked;
  localStorage.setItem('tom-show-path-details', String(showPathDetails));
  renderMessages();
});

let client: TomClient | null = null;
let selectedPeer: string | null = null;
let participants: Array<{ nodeId: string; username: string }> = [];
const knownPeers = new Map<string, { username: string; online: boolean }>();

// Store messages per conversation
interface StoredMessage {
  id: string;
  text: string;
  isSent: boolean;
  status: string;
}
const conversations = new Map<string, StoredMessage[]>();
const unreadCounts = new Map<string, number>();

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
      const msg: StoredMessage = {
        id: envelope.id,
        text: payload.text,
        isSent: false,
        status: peer?.username ?? envelope.from.slice(0, 8),
      };

      // Store in conversation
      if (!conversations.has(envelope.from)) {
        conversations.set(envelope.from, []);
      }
      conversations.get(envelope.from)?.push(msg);

      // If this conversation is active, render it
      if (selectedPeer === envelope.from) {
        renderMessages();
        client?.markAsRead(envelope.id);
      } else {
        // Increment unread count
        unreadCounts.set(envelope.from, (unreadCounts.get(envelope.from) ?? 0) + 1);
      }
    }
    renderParticipants();
  });

  // Enhanced status tracking: show full lifecycle
  client.onMessageStatusChanged((messageId, _prev, newStatus) => {
    updateMessageStatus(messageId, newStatus);
  });

  // Legacy ACK handler (fallback for backward compatibility)
  client.onAck((messageId) => {
    updateStoredMessageStatus(messageId, 'delivered');
  });

  // Read receipt: show "read" status
  client.onMessageRead((messageId) => {
    updateStoredMessageStatus(messageId, 'read');
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

  client.onRoleChanged((nodeId, newRoles) => {
    console.log(`[Demo] onRoleChanged: ${nodeId.slice(0, 8)} -> ${newRoles.join(',')}`);
    if (nodeId === client?.getNodeId()) {
      console.log(`[Demo] It's ME! Updating my role display to: ${newRoles.join(',')}`);
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
    renderChatHeader();
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

  for (const [nodeId, peer] of sortedPeers) {
    const topology = client?.getTopologyInstance();
    const peerStatus = topology ? topology.getPeerStatus(nodeId) : peer.online ? 'online' : 'offline';
    const isOnline = peerStatus === 'online';
    const isStale = peerStatus === 'stale';

    if (isOnline || isStale) onlineCount++;

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

    // Unread badge
    const unread = unreadCounts.get(nodeId) ?? 0;
    if (unread > 0) {
      const badge = document.createElement('span');
      badge.className = 'unread-badge';
      badge.textContent = unread > 9 ? '9+' : String(unread);
      el.appendChild(badge);
    }

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
        selectPeer(nodeId);
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

function selectPeer(nodeId: string): void {
  selectedPeer = nodeId;
  // Clear unread count for this peer
  unreadCounts.set(nodeId, 0);
  // Mark all messages as read
  const msgs = conversations.get(nodeId) ?? [];
  for (const msg of msgs) {
    if (!msg.isSent) {
      client?.markAsRead(msg.id);
    }
  }
  renderParticipants();
  renderChatHeader();
  renderMessages();
}

function renderChatHeader(): void {
  if (!chatHeaderEl) return;
  if (selectedPeer) {
    const peer = knownPeers.get(selectedPeer);
    chatHeaderEl.textContent = peer?.username ?? selectedPeer.slice(0, 8);
    chatHeaderEl.style.display = 'block';
  } else {
    chatHeaderEl.textContent = 'Select a contact';
    chatHeaderEl.style.display = 'block';
  }
}

function renderMyRole(): void {
  if (!client) return;
  const roles = client.getCurrentRoles();
  const roleText = roles.join(', ');
  console.log(`[Demo] renderMyRole: getCurrentRoles() returned [${roleText}]`);
  myRoleEl.textContent = `Role: ${roleText}`;
}

function renderMessages(): void {
  messagesEl.innerHTML = '';
  if (!selectedPeer) return;

  const msgs = conversations.get(selectedPeer) ?? [];
  for (const msg of msgs) {
    const el = document.createElement('div');
    el.className = `message ${msg.isSent ? 'sent' : 'received'}`;
    el.dataset.msgId = msg.id;

    const statusDisplay: Record<string, string> = {
      pending: '...',
      sent: 'sent',
      relayed: 'relayed',
      delivered: 'delivered',
      read: 'read ✓✓',
    };
    const statusText = msg.isSent ? (statusDisplay[msg.status] ?? msg.status) : '';

    // Path visualization (Story 4.3)
    let pathHtml = '';
    if (showPathDetails && !msg.isSent) {
      const pathInfo = client?.getPathInfo(msg.id);
      if (pathInfo) {
        const routeIcon = pathInfo.routeType === 'direct' ? '⚡' : '🔀';
        const routeClass = pathInfo.routeType === 'direct' ? 'direct' : '';
        const relayHopsText = pathInfo.relayHops.length > 0 ? ` via ${pathInfo.relayHops.join(' → ')}` : '';
        pathHtml = `<div class="path-info visible">
          <span class="route-type ${routeClass}">${routeIcon} ${pathInfo.routeType}</span>
          <span class="relay-hops">${relayHopsText}</span>
          <span class="latency">${formatLatency(pathInfo.latencyMs)}</span>
        </div>`;
      }
    }

    el.innerHTML = `<div>${escapeHtml(msg.text)}</div><div class="meta">${statusText}</div>${pathHtml}`;
    messagesEl.appendChild(el);
  }
  messagesEl.scrollTop = messagesEl.scrollHeight;
}

function escapeHtml(text: string): string {
  const div = document.createElement('div');
  div.textContent = text;
  return div.innerHTML;
}

function updateMessageStatus(messageId: string, status: string): void {
  updateStoredMessageStatus(messageId, status);
}

function updateStoredMessageStatus(messageId: string, status: string): void {
  // Find and update in stored conversations
  for (const msgs of conversations.values()) {
    const msg = msgs.find((m) => m.id === messageId);
    if (msg?.isSent) {
      msg.status = status;
      break;
    }
  }
  // Re-render if visible
  if (selectedPeer) {
    const msgs = conversations.get(selectedPeer) ?? [];
    if (msgs.some((m) => m.id === messageId)) {
      renderMessages();
    }
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
      const msg: StoredMessage = {
        id: envelope.id,
        text,
        isSent: true,
        status: 'sent',
      };

      // Store in conversation
      if (!conversations.has(selectedPeer)) {
        conversations.set(selectedPeer, []);
      }
      conversations.get(selectedPeer)?.push(msg);

      renderMessages();
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
