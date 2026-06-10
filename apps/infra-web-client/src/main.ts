type EndpointResult = {
  ok: boolean;
  status?: number;
  latencyMs?: number;
  error?: string;
  body?: unknown;
};

type NodeStatus = {
  schema_version?: number;
  node?: string;
  node_id?: string;
  platform?: string;
  relay_url_active?: string;
  phase?: string;
  taille_reseau?: number;
  role?: string;
  relayeurs?: number;
  pairs_connectes?: string[];
  groupes?: Array<{ nom: string; membres: number }>;
  messages_envoyes?: number;
  messages_recus?: number;
  messages_echoues?: number;
  uptime_secondes?: number;
};

const relayUrlInput = document.getElementById('relayUrl') as HTMLInputElement;
const discoveryUrlInput = document.getElementById('discoveryUrl') as HTMLInputElement;
const refreshBtn = document.getElementById('refreshBtn') as HTMLButtonElement;
const autoBtn = document.getElementById('autoBtn') as HTMLButtonElement;
const nodeUrlInput = document.getElementById('nodeUrlInput') as HTMLInputElement;
const addNodeBtn = document.getElementById('addNodeBtn') as HTMLButtonElement;
const nodeCards = document.getElementById('nodeCards') as HTMLElement;

const relaySummary = document.getElementById('relaySummary') as HTMLElement;
const relayLines = document.getElementById('relayLines') as HTMLElement;
const discSummary = document.getElementById('discSummary') as HTMLElement;
const discLines = document.getElementById('discLines') as HTMLElement;
const relayCount = document.getElementById('relayCount') as HTMLElement;
const relayList = document.getElementById('relayList') as HTMLElement;
const snapshot = document.getElementById('snapshot') as HTMLElement;

const params = new URLSearchParams(window.location.search);
relayUrlInput.value = params.get('relay') ?? 'http://127.0.0.1:3340';
discoveryUrlInput.value = params.get('discovery') ?? 'http://127.0.0.1:8080';

const NODES_KEY = 'tom_node_endpoints';

function loadNodeEndpoints(): string[] {
  try {
    return JSON.parse(localStorage.getItem(NODES_KEY) ?? '[]') as string[];
  } catch {
    return [];
  }
}

function saveNodeEndpoints(endpoints: string[]): void {
  localStorage.setItem(NODES_KEY, JSON.stringify(endpoints));
}

function removeNode(url: string): void {
  const endpoints = loadNodeEndpoints().filter((e) => e !== url);
  saveNodeEndpoints(endpoints);
  void refresh();
}

let timer: number | null = null;

function fmtStatus(ok: boolean): string {
  return ok ? '<span class="status-ok">OK</span>' : '<span class="status-ko">KO</span>';
}

function normalize(url: string): string {
  return url.trim().replace(/\/$/, '');
}

const ESC_MAP: Record<string, string> = { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' };
function esc(s: unknown): string {
  return String(s ?? '').replace(/[&<>"']/g, (c) => ESC_MAP[c]!);
}

async function probeJson(url: string): Promise<EndpointResult> {
  const started = performance.now();
  try {
    const res = await fetch(url, { method: 'GET' });
    const latencyMs = Math.round(performance.now() - started);
    const text = await res.text();
    let body: unknown = text;
    try {
      body = JSON.parse(text);
    } catch {
      // keep raw text
    }
    return { ok: res.ok, status: res.status, latencyMs, body };
  } catch (error) {
    return {
      ok: false,
      error: error instanceof Error ? error.message : String(error),
      latencyMs: Math.round(performance.now() - started),
    };
  }
}

function setSummary(el: HTMLElement, label: string, okCount: number, total: number): void {
  const ratio = `${okCount}/${total}`;
  const cls = okCount === total ? 'status-ok' : okCount > 0 ? 'status-warn' : 'status-ko';
  el.innerHTML = `<span class="k">${label}</span><span class="${cls}">${ratio}</span>`;
}

function endpointLine(name: string, res: EndpointResult): string {
  const status = res.ok ? `HTTP ${res.status}` : (res.error ?? `HTTP ${res.status ?? 'ERR'}`);
  return `<div class="line"><span class="k">${name}</span><span>${fmtStatus(res.ok)} <span class="mono">${status} · ${res.latencyMs ?? '-'}ms</span></span></div>`;
}

function nodeHealth(data: NodeStatus | null): 'ok' | 'warn' | 'ko' {
  if (!data) return 'ko';
  if ((data.taille_reseau ?? 0) >= 2 && data.phase === 'Converged') return 'ok';
  if ((data.taille_reseau ?? 0) >= 1 || data.phase === 'RelayAssist') return 'warn';
  return 'ko';
}

function fmtUptime(s: number): string {
  if (s < 60) return `${s}s`;
  if (s < 3600) return `${Math.floor(s / 60)}m ${s % 60}s`;
  return `${Math.floor(s / 3600)}h ${Math.floor((s % 3600) / 60)}m`;
}

function renderNodeCard(url: string, result: EndpointResult): string {
  const data = result.ok ? (result.body as NodeStatus) : null;
  const health = result.ok ? nodeHealth(data) : 'ko';
  const dot = health === 'ok' ? '🟢' : health === 'warn' ? '🟡' : '🔴';
  const label = esc(data?.node ?? url);
  const shortId = esc(data?.node_id ? data.node_id.slice(0, 8) : '—');
  const phase = esc(data?.phase ?? '—');
  const peers = esc(data?.taille_reseau ?? '—');
  const role = esc(data?.role ?? '—');
  const uptime = esc(typeof data?.uptime_secondes === 'number' ? fmtUptime(data.uptime_secondes) : '—');
  const sent = esc(data?.messages_envoyes ?? '—');
  const recv = esc(data?.messages_recus ?? '—');
  const dropped = esc(data?.messages_echoues ?? '—');
  const platform = esc(data?.platform ?? '—');
  const relay = esc(data?.relay_url_active ?? '—');
  const latency = result.latencyMs != null ? `${result.latencyMs}ms` : '—';
  const groupsHtml =
    Array.isArray(data?.groupes) && data.groupes.length > 0
      ? data.groupes.map((g) => `<span class="badge">${esc(g.nom)} (${esc(g.membres)})</span>`).join(' ')
      : '<span class="k">—</span>';
  const errorHtml = !result.ok
    ? `<div class="node-error mono">${esc(result.error ?? `HTTP ${result.status ?? 'ERR'}`)}</div>`
    : '';

  return `
    <section class="card node-card" data-health="${health}">
      <div class="node-header">
        <span class="node-dot">${dot}</span>
        <span class="node-label">${label}</span>
        <span class="mono muted" style="font-size:11px">${shortId}</span>
        <span class="mono muted" style="font-size:11px;margin-left:auto">${platform} · ${latency}</span>
        <button class="node-remove" data-url="${esc(url)}" title="Retirer">×</button>
      </div>
      ${errorHtml}
      <div class="node-grid">
        <div class="node-stat"><span class="k">Phase</span><span>${phase}</span></div>
        <div class="node-stat"><span class="k">Peers</span><span>${peers}</span></div>
        <div class="node-stat"><span class="k">Rôle</span><span>${role}</span></div>
        <div class="node-stat"><span class="k">Uptime</span><span>${uptime}</span></div>
        <div class="node-stat"><span class="k">Envoyés</span><span>${sent}</span></div>
        <div class="node-stat"><span class="k">Reçus</span><span>${recv}</span></div>
        <div class="node-stat"><span class="k">Échoués</span><span>${dropped}</span></div>
        <div class="node-stat"><span class="k">Relay</span><span class="mono" style="font-size:11px">${relay}</span></div>
      </div>
      <div class="node-groups">${groupsHtml}</div>
    </section>`;
}

async function refreshNodes(): Promise<void> {
  const endpoints = loadNodeEndpoints();
  if (endpoints.length === 0) {
    nodeCards.innerHTML =
      '<div class="muted mono" style="padding:8px">Aucun nœud configuré — ajoutez une URL ci-dessus.</div>';
    return;
  }
  const results = await Promise.all(endpoints.map((url) => probeJson(url)));
  nodeCards.innerHTML = results.map((res, i) => renderNodeCard(endpoints[i]!, res)).join('');
}

async function refresh(): Promise<void> {
  const relay = normalize(relayUrlInput.value);
  const discovery = normalize(discoveryUrlInput.value);

  const [rReady, rHealth, rHealthz, dHealth, dRelays, dMetrics, dStatus] = await Promise.all([
    probeJson(`${relay}/ready`),
    probeJson(`${relay}/health`),
    probeJson(`${relay}/healthz`),
    probeJson(`${discovery}/health`),
    probeJson(`${discovery}/relays`),
    probeJson(`${discovery}/metrics`),
    probeJson(`${discovery}/status`),
  ]);

  const relayResults = [rReady, rHealth, rHealthz];
  const discResults = [dHealth, dRelays, dMetrics, dStatus];

  setSummary(relaySummary, 'State', relayResults.filter((x) => x.ok).length, relayResults.length);
  setSummary(discSummary, 'State', discResults.filter((x) => x.ok).length, discResults.length);

  relayLines.innerHTML = [
    endpointLine('/ready', rReady),
    endpointLine('/health', rHealth),
    endpointLine('/healthz', rHealthz),
  ].join('');

  discLines.innerHTML = [
    endpointLine('/health', dHealth),
    endpointLine('/relays', dRelays),
    endpointLine('/metrics', dMetrics),
    endpointLine('/status', dStatus),
  ].join('');

  const relaysPayload = dRelays.body as {
    relays?: Array<{ url?: string; region?: string; load?: number; latency_hint_ms?: number }>;
    ttl_seconds?: number;
  };
  const relays = Array.isArray(relaysPayload?.relays) ? relaysPayload.relays : [];

  relayCount.innerHTML = `<span class="k">Count</span><span>${relays.length}</span>`;
  relayList.innerHTML = relays.length
    ? relays
        .map((r) => {
          const region = r.region ?? 'unknown';
          const load = typeof r.load === 'number' ? r.load.toFixed(2) : '-';
          const lat = typeof r.latency_hint_ms === 'number' ? `${r.latency_hint_ms}ms` : '-';
          return `<div class="relay"><div><strong>${r.url ?? 'n/a'}</strong></div><div class="mono">region=${region} · load=${load} · latency_hint=${lat}</div></div>`;
        })
        .join('')
    : '<div class="mono">No relay discovered.</div>';

  const statusPayload = dStatus.body as Record<string, unknown>;
  snapshot.textContent = JSON.stringify(
    {
      timestamp: new Date().toISOString(),
      relay_ok: relayResults.every((r) => r.ok),
      discovery_ok: discResults.every((r) => r.ok),
      discovered_relays: relays.length,
      relays_ttl_seconds: relaysPayload?.ttl_seconds ?? null,
      status_endpoint: statusPayload ?? null,
    },
    null,
    2,
  );

  await refreshNodes();
}

nodeCards.addEventListener('click', (e) => {
  const btn = (e.target as HTMLElement).closest<HTMLElement>('.node-remove');
  if (btn) {
    const url = btn.getAttribute('data-url');
    if (url) removeNode(url);
  }
});

addNodeBtn.addEventListener('click', () => {
  const url = normalize(nodeUrlInput.value);
  if (!url) return;
  const endpoints = loadNodeEndpoints();
  if (!endpoints.includes(url)) {
    endpoints.push(url);
    saveNodeEndpoints(endpoints);
  }
  nodeUrlInput.value = '';
  void refresh();
});

nodeUrlInput.addEventListener('keydown', (e) => {
  if (e.key === 'Enter') addNodeBtn.click();
});

refreshBtn.addEventListener('click', () => {
  void refresh();
});

autoBtn.addEventListener('click', () => {
  if (timer !== null) {
    window.clearInterval(timer);
    timer = null;
    autoBtn.textContent = 'Auto: OFF';
    return;
  }
  timer = window.setInterval(() => {
    void refresh();
  }, 5000);
  autoBtn.textContent = 'Auto: ON (5s)';
});

relayUrlInput.addEventListener('change', () => {
  void refresh();
});

discoveryUrlInput.addEventListener('change', () => {
  void refresh();
});

void refresh();
