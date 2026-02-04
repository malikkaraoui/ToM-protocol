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
import { GameController, SnakeRenderer, isGamePayload } from './game/index';
import type { Direction, GameSessionState } from './game/index';

const SIGNALING_URL = `ws://${window.location.hostname}:3001`;

/** Format bytes to human-readable string */
function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes}B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)}KB`;
  return `${(bytes / (1024 * 1024)).toFixed(2)}MB`;
}

/** Detect if device has touch capability */
const isTouchDevice = 'ontouchstart' in window || navigator.maxTouchPoints > 0;

/** Get appropriate control instructions based on device */
function getControlsText(): string {
  return isTouchDevice ? 'Swipe to move' : 'Arrow keys or WASD to move';
}

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
const gameContainerEl = document.getElementById('game-container') as HTMLElement;
const gameCanvasEl = document.getElementById('game-canvas') as HTMLCanvasElement;
const gameControlsEl = document.getElementById('game-controls') as HTMLElement;
const messageFormEl = document.getElementById('message-form') as HTMLFormElement;

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
  isGameResult?: boolean;
}
const conversations = new Map<string, StoredMessage[]>();
const unreadCounts = new Map<string, number>();

// Game controller (Story 4.5)
let gameController: GameController | null = null;
let gameRenderer: SnakeRenderer | null = null;

// Pending game invitations by peerId
const pendingInvitations = new Map<string, string>(); // peerId -> gameId

joinBtn.addEventListener('click', async () => {
  const username = usernameInput.value.trim();
  if (!username) return;

  client = new TomClient({ signalingUrl: SIGNALING_URL, username });

  // Initialize game controller
  gameController = new GameController(client, {
    onSessionStateChange: handleGameSessionStateChange,
    onGameEnd: handleGameEnd,
    onInvitationReceived: handleGameInvitationReceived,
    onInvitationDeclined: handleGameInvitationDeclined,
    onConnectionQualityChange: (quality) => {
      gameRenderer?.setConnectionQuality(quality);
    },
  });

  // Initialize game renderer
  gameRenderer = new SnakeRenderer(gameCanvasEl, { gridSize: 20, cellSize: 20 });
  gameController.setRenderer(gameRenderer);

  client.onStatus((status, detail) => {
    statusBar.textContent = detail ? `${status}: ${detail}` : status;
    // Update stats on relay-related events
    if (status.startsWith('message:') || status.startsWith('direct-path:')) {
      renderParticipants();
    }
    // Track connection quality for game
    if (status === 'direct-path:lost' && gameController?.isInGame()) {
      gameController.setConnectionQuality('relay');
    } else if (status === 'direct-path:restored' && gameController?.isInGame()) {
      gameController.setConnectionQuality('direct');
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
    const payload = envelope.payload;
    const peer = knownPeers.get(envelope.from);
    const peerUsername = peer?.username ?? envelope.from.slice(0, 8);

    // Check if this is a game payload
    if (isGamePayload(payload)) {
      gameController?.handleGamePayload(payload, envelope.from, peerUsername);
      return;
    }

    // Regular chat message
    const chatPayload = payload as { text?: string };
    if (chatPayload.text) {
      const msg: StoredMessage = {
        id: envelope.id,
        text: chatPayload.text,
        isSent: false,
        status: peerUsername,
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
    // Handle peer disconnect during game
    if (gameController?.getSession()?.peerId === nodeId) {
      gameController.handlePeerDisconnect();
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
  const bytesRelayed = stats?.bytesRelayed ?? 0;
  const bytesSent = stats?.bytesSent ?? 0;

  topologyStats.textContent = `${onlineCount} online · ${relayCount} relays · ${ackCount}/${stats?.ownMessagesSent ?? 0} relayed · ${forwardedCount} fwd (${formatBytes(bytesRelayed)}) · ↑${formatBytes(bytesSent)} · ${total} total`;
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

  // Clear previous content
  chatHeaderEl.innerHTML = '';

  if (selectedPeer) {
    const peer = knownPeers.get(selectedPeer);
    const peerName = peer?.username ?? selectedPeer.slice(0, 8);

    // Add peer name
    const nameSpan = document.createElement('span');
    nameSpan.textContent = peerName;
    chatHeaderEl.appendChild(nameSpan);

    // Add game invite button if peer is online and not in game
    const topology = client?.getTopologyInstance();
    const peerStatus = topology?.getPeerStatus(selectedPeer) ?? 'offline';
    const isOnline = peerStatus === 'online' || peerStatus === 'stale';

    if (isOnline && gameController?.canStartGame()) {
      const inviteBtn = document.createElement('button');
      inviteBtn.className = 'game-invite-btn';
      inviteBtn.textContent = '🐍 Play';
      inviteBtn.title = 'Invite to Snake game';
      inviteBtn.addEventListener('click', () => {
        if (selectedPeer && peer) {
          gameController?.sendInvitation(selectedPeer, peer.username);
        }
      });
      chatHeaderEl.appendChild(inviteBtn);
    }

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

  // Render pending game invitation BEFORE messages (outside the loop)
  if (pendingInvitations.has(selectedPeer)) {
    const invEl = document.createElement('div');
    invEl.className = 'game-invitation';
    invEl.innerHTML = `
      <div class="game-invitation-title">🐍 Snake Game Invitation</div>
      <div>You've been invited to play Snake!</div>
      <div class="game-invitation-actions">
        <button class="game-accept-btn" data-peer="${selectedPeer}">Accept</button>
        <button class="game-decline-btn" data-peer="${selectedPeer}">Decline</button>
      </div>
    `;
    messagesEl.appendChild(invEl);

    // Add event listeners - capture selectedPeer to avoid non-null assertion
    const currentPeer = selectedPeer;
    invEl.querySelector('.game-accept-btn')?.addEventListener('click', () => {
      gameController?.acceptInvitation();
      if (currentPeer) pendingInvitations.delete(currentPeer);
      renderMessages();
    });
    invEl.querySelector('.game-decline-btn')?.addEventListener('click', () => {
      gameController?.declineInvitation();
      if (currentPeer) pendingInvitations.delete(currentPeer);
      renderMessages();
    });
  }

  for (const msg of msgs) {
    const el = document.createElement('div');

    // Game result messages have special styling
    if (msg.isGameResult) {
      el.className = 'message received game-result';
      el.innerHTML = `<div>${escapeHtml(msg.text)}</div>`;
      messagesEl.appendChild(el);
      continue;
    }

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

// ============================================
// Game Event Handlers (Story 4.5)
// ============================================

function handleGameSessionStateChange(state: GameSessionState): void {
  const isInGame = state === 'countdown' || state === 'playing' || state === 'ended';

  // Toggle between chat and game views
  if (isInGame) {
    messagesEl.style.display = 'none';
    messageFormEl.style.display = 'none';
    gameContainerEl.classList.add('active');
  } else {
    messagesEl.style.display = 'block';
    messageFormEl.style.display = 'flex';
    gameContainerEl.classList.remove('active');
  }

  // Update controls text based on state
  if (state === 'waiting-accept') {
    gameControlsEl.textContent = 'Waiting for opponent to accept...';
  } else if (state === 'countdown') {
    gameControlsEl.textContent = 'Get ready!';
  } else if (state === 'playing') {
    gameControlsEl.textContent = getControlsText();
  } else if (state === 'ended') {
    gameControlsEl.textContent = isTouchDevice ? 'Tap to return to chat' : 'Click to return to chat';
  }

  // Re-render header to show/hide invite button
  renderChatHeader();
}

function handleGameEnd(_winner: string, _reason: string, resultMessage: string): void {
  const session = gameController?.getSession();
  if (!session) return;

  // Add game result as a chat message
  if (!conversations.has(session.peerId)) {
    conversations.set(session.peerId, []);
  }
  conversations.get(session.peerId)?.push({
    id: `game-result-${Date.now()}`,
    text: resultMessage,
    isSent: false,
    status: '',
    isGameResult: true,
  });

  // Set up click handler to return to chat
  const returnToChat = () => {
    gameController?.endSession();
    gameCanvasEl.removeEventListener('click', returnToChat);
    renderMessages();
  };
  gameCanvasEl.addEventListener('click', returnToChat);
}

function handleGameInvitationReceived(peerId: string, _peerUsername: string, gameId: string): void {
  pendingInvitations.set(peerId, gameId);

  // If we're viewing this peer's conversation, show the invitation
  if (selectedPeer === peerId) {
    renderMessages();
  } else {
    // Increment unread count to notify user
    unreadCounts.set(peerId, (unreadCounts.get(peerId) ?? 0) + 1);
    renderParticipants();
  }
}

function handleGameInvitationDeclined(peerId: string): void {
  statusBar.textContent = 'Game invitation declined';
  pendingInvitations.delete(peerId);
  renderMessages();
}

// ============================================
// Keyboard Controls (Story 4.5)
// ============================================

document.addEventListener('keydown', (e) => {
  if (!gameController?.isInGame()) return;
  const session = gameController.getSession();
  if (!session || session.state !== 'playing') return;

  let direction: Direction | null = null;

  switch (e.key) {
    case 'ArrowUp':
    case 'w':
    case 'W':
      direction = 'up';
      break;
    case 'ArrowDown':
    case 's':
    case 'S':
      direction = 'down';
      break;
    case 'ArrowLeft':
    case 'a':
    case 'A':
      direction = 'left';
      break;
    case 'ArrowRight':
    case 'd':
    case 'D':
      direction = 'right';
      break;
  }

  if (direction) {
    e.preventDefault();
    gameController.handleLocalInput(direction);
  }
});

// ============================================
// Touch Controls (Story 4.5 - Task 8)
// ============================================

let touchStartX = 0;
let touchStartY = 0;

gameCanvasEl.addEventListener(
  'touchstart',
  (e) => {
    if (!gameController?.isInGame()) return;
    const session = gameController.getSession();
    if (session?.state === 'playing') {
      e.preventDefault(); // Prevent scrolling during game
    }
    const touch = e.touches[0];
    touchStartX = touch.clientX;
    touchStartY = touch.clientY;
  },
  { passive: false },
);

gameCanvasEl.addEventListener(
  'touchmove',
  (e) => {
    if (!gameController?.isInGame()) return;
    const session = gameController.getSession();
    if (session?.state === 'playing') {
      e.preventDefault(); // Prevent scrolling during game
    }
  },
  { passive: false },
);

gameCanvasEl.addEventListener('touchend', (e) => {
  if (!gameController?.isInGame()) return;
  const session = gameController.getSession();
  if (!session || session.state !== 'playing') return;

  const touch = e.changedTouches[0];
  const dx = touch.clientX - touchStartX;
  const dy = touch.clientY - touchStartY;

  // Require minimum swipe distance
  const minSwipe = 30;
  if (Math.abs(dx) < minSwipe && Math.abs(dy) < minSwipe) return;

  let direction: Direction;
  if (Math.abs(dx) > Math.abs(dy)) {
    direction = dx > 0 ? 'right' : 'left';
  } else {
    direction = dy > 0 ? 'down' : 'up';
  }

  gameController.handleLocalInput(direction);
});

// ============================================
// Form Handlers
// ============================================

usernameInput.addEventListener('keydown', (e) => {
  if (e.key === 'Enter') joinBtn.click();
});

messageFormEl.addEventListener('submit', (e) => {
  e.preventDefault();
  sendMessage();
});
