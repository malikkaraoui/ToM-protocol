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
const sendBtn = document.getElementById('send-btn') as HTMLElement;
const statusBar = document.getElementById('status-bar') as HTMLElement;
const topologyStats = document.getElementById('topology-stats') as HTMLElement;

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
      addMessage(peer?.username ?? envelope.from.slice(0, 8), payload.text, false);
    }
  });

  client.onAck((messageId) => {
    const msgEl = document.querySelector(`[data-msg-id="${messageId}"] .meta`);
    if (msgEl) msgEl.textContent = 'delivered';
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

  try {
    await client.connect();
    loginEl.style.display = 'none';
    chatEl.style.display = 'block';
    nodeIdEl.textContent = `Node: ${client.getNodeId().slice(0, 16)}...`;
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
    el.appendChild(name);

    if (isOnline || isStale) {
      el.addEventListener('click', () => {
        selectedPeer = nodeId;
        renderParticipants();
      });
    }
    participantsEl.appendChild(el);
  }

  const total = knownPeers.size;
  const indirect = client?.getTopologyInstance()?.getIndirectPeers().length ?? 0;
  topologyStats.textContent = `${onlineCount} online · ${offlineCount} offline · ${indirect} indirect · ${total} total`;
}

function addMessage(from: string, text: string, isSent: boolean, msgId?: string): void {
  const el = document.createElement('div');
  el.className = `message ${isSent ? 'sent' : 'received'}`;
  if (msgId) el.dataset.msgId = msgId;
  el.innerHTML = `<div>${text}</div><div class="meta">${isSent ? 'sent' : from}</div>`;
  messagesEl.appendChild(el);
  messagesEl.scrollTop = messagesEl.scrollHeight;
}

async function sendMessage(): Promise<void> {
  if (!client || !selectedPeer) return;
  const text = messageInput.value.trim();
  if (!text) return;

  const onlinePeers = participants.filter((p) => {
    const peer = knownPeers.get(p.nodeId);
    return peer?.online && p.nodeId !== selectedPeer;
  });
  const relay = onlinePeers[0];
  const envelope = await client.sendMessage(selectedPeer, text, relay?.nodeId);

  if (envelope) {
    addMessage('You', text, true, envelope.id);
    messageInput.value = '';
  }
}

usernameInput.addEventListener('keydown', (e) => {
  if (e.key === 'Enter') joinBtn.click();
});

sendBtn.addEventListener('click', sendMessage);
messageInput.addEventListener('keydown', (e) => {
  if (e.key === 'Enter') sendMessage();
});
