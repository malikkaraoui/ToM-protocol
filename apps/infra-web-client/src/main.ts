type EndpointResult = {
  ok: boolean;
  status?: number;
  latencyMs?: number;
  error?: string;
  body?: unknown;
};

const relayUrlInput = document.getElementById('relayUrl') as HTMLInputElement;
const discoveryUrlInput = document.getElementById('discoveryUrl') as HTMLInputElement;
const refreshBtn = document.getElementById('refreshBtn') as HTMLButtonElement;
const autoBtn = document.getElementById('autoBtn') as HTMLButtonElement;

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

let timer: number | null = null;

function fmtStatus(ok: boolean): string {
  return ok ? '<span class="status-ok">OK</span>' : '<span class="status-ko">KO</span>';
}

function normalize(url: string): string {
  return url.trim().replace(/\/$/, '');
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
}

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
  }, 15000);
  autoBtn.textContent = 'Auto: ON';
});

relayUrlInput.addEventListener('change', () => {
  void refresh();
});

discoveryUrlInput.addEventListener('change', () => {
  void refresh();
});

void refresh();
