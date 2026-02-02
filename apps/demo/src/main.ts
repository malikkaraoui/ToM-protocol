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

let client: TomClient | null = null;
let selectedPeer: string | null = null;
let participants: Array<{ nodeId: string; username: string }> = [];

joinBtn.addEventListener('click', async () => {
  const username = usernameInput.value.trim();
  if (!username) return;

  client = new TomClient({ signalingUrl: SIGNALING_URL, username });

  client.onStatus((status, detail) => {
    statusBar.textContent = detail ? `${status}: ${detail}` : status;
  });

  client.onParticipants((list) => {
    participants = list.filter((p) => p.nodeId !== client?.getNodeId());
    renderParticipants();
  });

  client.onMessage((envelope) => {
    const payload = envelope.payload as { text?: string };
    if (payload.text) {
      const sender = participants.find((p) => p.nodeId === envelope.from);
      addMessage(sender?.username ?? envelope.from.slice(0, 8), payload.text, false);
    }
  });

  client.onAck((messageId) => {
    const msgEl = document.querySelector(`[data-msg-id="${messageId}"] .meta`);
    if (msgEl) msgEl.textContent = 'delivered';
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
  for (const p of participants) {
    const el = document.createElement('div');
    el.className = `participant${selectedPeer === p.nodeId ? ' active' : ''}`;
    el.textContent = p.username;
    el.addEventListener('click', () => {
      selectedPeer = p.nodeId;
      renderParticipants();
    });
    participantsEl.appendChild(el);
  }
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

  // For PoC, use the first other participant as relay if available
  const relay = participants.find((p) => p.nodeId !== selectedPeer);
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
