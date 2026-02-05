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

/** Detect if device has touch capability */
const isTouchDevice = 'ontouchstart' in window || navigator.maxTouchPoints > 0;

/** Get appropriate control instructions based on device */
function getControlsText(): string {
  return isTouchDevice ? 'Glissez pour bouger' : 'Flèches ou WASD pour bouger';
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
  isGroupInvite?: boolean;
  groupInviteData?: { groupId: string; groupName: string };
}
const conversations = new Map<string, StoredMessage[]>();
const unreadCounts = new Map<string, number>();

// Game controller (Story 4.5)
let gameController: GameController | null = null;
let gameRenderer: SnakeRenderer | null = null;

// Pending game invitations by peerId
const pendingInvitations = new Map<string, string>(); // peerId -> gameId

// Group state (Story 4.6)
let selectedGroup: string | null = null;
const groupUnreadCounts = new Map<string, number>();
const groupListEl = document.getElementById('group-list') as HTMLElement;
const createGroupBtn = document.getElementById('create-group-btn') as HTMLButtonElement;
const createGroupModal = document.getElementById('create-group-modal') as HTMLElement;
const groupNameInput = document.getElementById('group-name-input') as HTMLInputElement;
const createGroupCancelBtn = document.getElementById('create-group-cancel-btn') as HTMLButtonElement;
const createGroupConfirmBtn = document.getElementById('create-group-confirm-btn') as HTMLButtonElement;

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

  // Group event handlers (Story 4.6)
  client.onGroupCreated((group) => {
    console.log(`[Demo] Group created: ${group.name}`);
    renderGroups();
  });

  client.onGroupInvite((groupId, groupName, inviterId, inviterUsername) => {
    // Add invitation as a message in the conversation with the inviter
    if (!conversations.has(inviterId)) {
      conversations.set(inviterId, []);
    }

    // Store inviter info if not known
    if (!knownPeers.has(inviterId)) {
      knownPeers.set(inviterId, { username: inviterUsername, online: true });
    }

    const inviteMsg: StoredMessage = {
      id: `group-invite-${groupId}-${Date.now()}`,
      text: `Invitation au groupe "${groupName}"`,
      isSent: false,
      status: inviterUsername,
      isGroupInvite: true,
      groupInviteData: { groupId, groupName },
    };
    conversations.get(inviterId)?.push(inviteMsg);

    // If viewing this conversation, re-render
    if (selectedPeer === inviterId) {
      renderMessages();
    } else {
      // Increment unread count
      unreadCounts.set(inviterId, (unreadCounts.get(inviterId) ?? 0) + 1);
    }

    renderParticipants();
    renderGroups();
  });

  client.onGroupMemberJoined((groupId, member) => {
    console.log(`[Demo] ${member.username} joined group ${groupId}`);
    renderGroups();
    if (selectedGroup === groupId) {
      renderGroupChatHeader(); // Update member count
      renderGroupMessages();
    }
  });

  client.onGroupMemberLeft((groupId, _nodeId, username) => {
    console.log(`[Demo] ${username} left group ${groupId}`);
    renderGroups();
    if (selectedGroup === groupId) {
      renderGroupMessages();
    }
  });

  client.onGroupMessage((groupId, message) => {
    console.log(`[Demo] Group message in ${groupId}: ${message.text}`);
    if (selectedGroup === groupId) {
      renderGroupMessages();
    } else {
      // Increment unread count for group
      groupUnreadCounts.set(groupId, (groupUnreadCounts.get(groupId) ?? 0) + 1);
      renderGroups();
    }
  });

  // Note: Public group announcements disabled - invitations only via direct 1-to-1 channels

  try {
    await client.connect();
    loginEl.style.display = 'none';
    chatEl.style.display = 'block';
    nodeIdEl.textContent = `Nœud : ${client.getNodeId().slice(0, 16)}...`;
    renderMyRole();
    renderChatHeader();
    // Periodic re-render to reflect heartbeat status changes
    setInterval(renderParticipants, 3000);
  } catch (err) {
    statusBar.textContent = `Échec de connexion : ${err}`;
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

  // Relay count: other relays + self if we have relay role
  const otherRelays = client?.getTopologyInstance()?.getRelayNodes().length ?? 0;
  const myRoles = client?.getCurrentRoles() ?? [];
  const selfIsRelay = myRoles.includes('relay') ? 1 : 0;
  const relayCount = otherRelays + selfIsRelay;

  // Stats for contribution/usage equilibrium score (Story 5.4)
  const stats = client?.getRelayStats();
  const forwardedCount = stats?.messagesRelayed ?? 0;
  const equilibriumScore = stats?.relayToOwnRatio ?? 0;
  const scoreDisplay =
    equilibriumScore >= 1
      ? `⚡${equilibriumScore.toFixed(1)}`
      : equilibriumScore > 0
        ? `${equilibriumScore.toFixed(1)}`
        : '0';

  topologyStats.textContent = `${onlineCount} en ligne · ${relayCount} relais · Score: ${scoreDisplay} · ↑${stats?.ownMessagesSent ?? 0} ↔${forwardedCount}`;
}

function selectPeer(nodeId: string): void {
  selectedPeer = nodeId;
  selectedGroup = null; // Deselect group when selecting a peer
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
  renderGroups(); // Update group selection UI
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
    chatHeaderEl.textContent = 'Sélectionnez un contact';
    chatHeaderEl.style.display = 'block';
  }
}

function renderMyRole(): void {
  if (!client) return;
  const roles = client.getCurrentRoles();
  const roleText = roles.join(', ');
  console.log(`[Demo] renderMyRole: getCurrentRoles() returned [${roleText}]`);
  myRoleEl.textContent = `Rôle : ${roleText}`;
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
      <div class="game-invitation-title">🐍 Invitation au jeu Snake</div>
      <div>On vous invite à jouer au Snake !</div>
      <div class="game-invitation-actions">
        <button class="game-accept-btn" data-peer="${selectedPeer}">Accepter</button>
        <button class="game-decline-btn" data-peer="${selectedPeer}">Refuser</button>
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

    // Group invite messages (Story 4.6 - invitations in chat)
    if (msg.isGroupInvite && msg.groupInviteData) {
      const inviteData = msg.groupInviteData;
      const isAlreadyJoined = !!client?.getGroup(inviteData.groupId);

      el.className = 'message received group-invite-message';
      el.dataset.msgId = msg.id;

      if (isAlreadyJoined) {
        el.innerHTML = `
          <div class="group-invite-card-title">👥 Invitation au groupe</div>
          <div class="group-invite-card-info">${escapeHtml(inviteData.groupName)}</div>
          <div class="group-invite-card-info" style="color: #00ff88;">✓ Déjà rejoint</div>
        `;
      } else {
        el.innerHTML = `
          <div class="group-invite-card-title">👥 Invitation au groupe</div>
          <div class="group-invite-card-info">${escapeHtml(inviteData.groupName)}</div>
          <div class="group-invite-card-actions">
            <button class="group-accept-btn" data-group-id="${inviteData.groupId}">Rejoindre</button>
            <button class="group-decline-btn" data-group-id="${inviteData.groupId}">Refuser</button>
          </div>
        `;

        // Add event listeners after appending
        messagesEl.appendChild(el);

        const acceptBtn = el.querySelector('.group-accept-btn') as HTMLButtonElement;
        const declineBtn = el.querySelector('.group-decline-btn') as HTMLButtonElement;

        acceptBtn?.addEventListener('click', async () => {
          // Disable buttons and show loading state
          acceptBtn.disabled = true;
          declineBtn.disabled = true;
          acceptBtn.textContent = 'En cours...';

          const result = await client?.acceptGroupInvite(inviteData.groupId);
          if (result) {
            acceptBtn.textContent = '✓ Rejoint';
            declineBtn.style.display = 'none';
          } else {
            acceptBtn.textContent = 'Rejoindre';
            acceptBtn.disabled = false;
            declineBtn.disabled = false;
          }
          renderGroups();
        });
        declineBtn?.addEventListener('click', () => {
          client?.declineGroupInvite(inviteData.groupId);
          renderMessages();
          renderGroups();
        });
        continue;
      }

      messagesEl.appendChild(el);
      continue;
    }

    el.className = `message ${msg.isSent ? 'sent' : 'received'}`;
    el.dataset.msgId = msg.id;

    const statusDisplay: Record<string, string> = {
      pending: '...',
      sent: 'envoyé',
      relayed: 'relayé',
      delivered: 'livré',
      read: 'lu ✓✓',
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
    statusBar.textContent = `Échec d'envoi : ${error.message}`;
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
    gameControlsEl.textContent = "En attente de l'adversaire...";
  } else if (state === 'countdown') {
    gameControlsEl.textContent = 'Préparez-vous !';
  } else if (state === 'playing') {
    gameControlsEl.textContent = getControlsText();
  } else if (state === 'ended') {
    gameControlsEl.textContent = isTouchDevice ? 'Touchez pour revenir au chat' : 'Cliquez pour revenir au chat';
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
  statusBar.textContent = 'Invitation refusée';
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
  if (selectedGroup) {
    sendGroupMessage();
  } else {
    sendMessage();
  }
});

// ============================================
// Group Functions (Story 4.6)
// ============================================

function renderGroups(): void {
  if (!groupListEl || !client) return;
  groupListEl.innerHTML = '';

  // Note: Invitations are shown in the 1-to-1 chat with the inviter, not here

  // Render my groups (already joined)
  const groups = client.getGroups();
  for (const group of groups) {
    const el = document.createElement('div');
    const classes = ['group-item'];
    if (selectedGroup === group.groupId) classes.push('active');
    el.className = classes.join(' ');

    const unread = groupUnreadCounts.get(group.groupId) ?? 0;
    const unreadBadge = unread > 0 ? `<span class="unread-badge">${unread > 9 ? '9+' : unread}</span>` : '';

    el.innerHTML = `
      <div class="group-icon">👥</div>
      <div class="group-name">${escapeHtml(group.name)}</div>
      ${unreadBadge}
      <div class="group-members">${group.members.length}</div>
    `;

    el.addEventListener('click', () => {
      selectGroup(group.groupId);
    });

    groupListEl.appendChild(el);
  }

  // Note: Public groups discovery disabled - join only via direct invitations
}

function selectGroup(groupId: string): void {
  selectedGroup = groupId;
  selectedPeer = null; // Deselect peer
  groupUnreadCounts.set(groupId, 0);
  renderParticipants();
  renderGroups();
  renderGroupChatHeader();
  renderGroupMessages();
}

function renderGroupChatHeader(): void {
  if (!chatHeaderEl || !client || !selectedGroup) return;
  const group = client.getGroup(selectedGroup);
  if (!group) return;

  chatHeaderEl.innerHTML = '';

  // Group name
  const nameSpan = document.createElement('span');
  nameSpan.textContent = `👥 ${group.name}`;
  chatHeaderEl.appendChild(nameSpan);

  // Member count
  const membersSpan = document.createElement('span');
  membersSpan.style.marginLeft = '12px';
  membersSpan.style.fontSize = '11px';
  membersSpan.style.color = '#888';
  membersSpan.textContent = `${group.members.length} membres`;
  chatHeaderEl.appendChild(membersSpan);

  // Invite button - only show for group creator (admin)
  if (group.createdBy === client.getNodeId()) {
    const inviteBtn = document.createElement('button');
    inviteBtn.className = 'game-invite-btn';
    inviteBtn.style.marginLeft = '12px';
    inviteBtn.textContent = '+ Inviter';
    inviteBtn.addEventListener('click', () => {
      showInviteModal(group.groupId);
    });
    chatHeaderEl.appendChild(inviteBtn);
  }

  // Leave button
  const leaveBtn = document.createElement('button');
  leaveBtn.className = 'game-invite-btn';
  leaveBtn.style.marginLeft = 'auto';
  leaveBtn.style.background = '#ff4444';
  leaveBtn.textContent = 'Quitter';
  leaveBtn.addEventListener('click', async () => {
    if (selectedGroup) {
      await client?.leaveGroup(selectedGroup);
      selectedGroup = null;
      chatHeaderEl.textContent = 'Sélectionnez un contact';
      messagesEl.innerHTML = '';
      renderGroups();
    }
  });
  chatHeaderEl.appendChild(leaveBtn);

  chatHeaderEl.style.display = 'flex';
  chatHeaderEl.style.alignItems = 'center';
}

function renderGroupMessages(): void {
  if (!messagesEl || !client || !selectedGroup) return;
  messagesEl.innerHTML = '';

  const messages = client.getGroupMessages(selectedGroup);
  const myNodeId = client.getNodeId();

  for (const msg of messages) {
    const isSent = msg.senderId === myNodeId;
    const el = document.createElement('div');
    el.className = `message ${isSent ? 'sent' : 'received'}`;

    const senderDisplay = isSent
      ? ''
      : `<div class="meta" style="color: #00d4ff; margin-bottom: 4px;">${escapeHtml(msg.senderUsername)}</div>`;

    el.innerHTML = `
      ${senderDisplay}
      <div>${escapeHtml(msg.text)}</div>
      <div class="meta">${new Date(msg.sentAt).toLocaleTimeString()}</div>
    `;
    messagesEl.appendChild(el);
  }

  messagesEl.scrollTop = messagesEl.scrollHeight;
}

async function sendGroupMessage(): Promise<void> {
  if (!client || !selectedGroup) return;
  const text = messageInput.value.trim();
  if (!text) return;

  try {
    const success = await client.sendGroupMessage(selectedGroup, text);
    if (success) {
      messageInput.value = '';
      renderGroupMessages();
    }
  } catch (err) {
    const error = err as Error;
    statusBar.textContent = `Échec d'envoi groupe : ${error.message}`;
  }
}

// Modal handlers
createGroupBtn?.addEventListener('click', () => {
  createGroupModal?.classList.add('active');
  groupNameInput?.focus();
});

createGroupCancelBtn?.addEventListener('click', () => {
  createGroupModal?.classList.remove('active');
  if (groupNameInput) groupNameInput.value = '';
});

createGroupConfirmBtn?.addEventListener('click', async () => {
  const name = groupNameInput?.value.trim();
  if (!name || !client) return;

  // Get a relay to use as hub
  const relays = client.getTopologyInstance()?.getRelayNodes() ?? [];
  const myRoles = client.getCurrentRoles();
  const hubRelayId = myRoles.includes('relay') ? client.getNodeId() : relays[0];

  if (!hubRelayId) {
    statusBar.textContent = 'Aucun relais disponible pour le hub';
    return;
  }

  const group = await client.createGroup(name, []);
  if (group) {
    createGroupModal?.classList.remove('active');
    if (groupNameInput) groupNameInput.value = '';
    selectGroup(group.groupId);
  }
});

groupNameInput?.addEventListener('keydown', (e) => {
  if (e.key === 'Enter') {
    e.preventDefault();
    createGroupConfirmBtn?.click();
  } else if (e.key === 'Escape') {
    createGroupCancelBtn?.click();
  }
});

// ============================================
// Invite to Group Modal
// ============================================

const inviteModal = document.getElementById('invite-modal') as HTMLElement;
const inviteModalList = document.getElementById('invite-modal-list') as HTMLElement;
const inviteModalCloseBtn = document.getElementById('invite-modal-close-btn') as HTMLButtonElement;

function showInviteModal(groupId: string): void {
  if (!inviteModal || !inviteModalList || !client) return;

  const group = client.getGroup(groupId);
  if (!group) return;

  // Get contacts not already in group
  const memberIds = new Set(group.members.map((m) => m.nodeId));
  const availableContacts = Array.from(knownPeers.entries()).filter(([nodeId]) => !memberIds.has(nodeId));

  inviteModalList.innerHTML = '';

  if (availableContacts.length === 0) {
    inviteModalList.innerHTML = '<div style="color: #888; padding: 12px;">Aucun contact disponible</div>';
  } else {
    for (const [nodeId, peer] of availableContacts) {
      const item = document.createElement('div');
      item.style.cssText = 'padding: 10px 12px; cursor: pointer; border-radius: 4px; margin-bottom: 4px;';
      item.textContent = peer.username;
      item.addEventListener('mouseenter', () => {
        item.style.background = '#0f3460';
      });
      item.addEventListener('mouseleave', () => {
        item.style.background = 'transparent';
      });
      item.addEventListener('click', async () => {
        await client?.inviteToGroup(groupId, nodeId, peer.username);
        inviteModal.classList.remove('active');
        statusBar.textContent = `Invitation envoyée à ${peer.username}`;
      });
      inviteModalList.appendChild(item);
    }
  }

  inviteModal.classList.add('active');
}

inviteModalCloseBtn?.addEventListener('click', () => {
  inviteModal?.classList.remove('active');
});
