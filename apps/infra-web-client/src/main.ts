// ToM · Réseau — tableau de bord zéro-config.
// Sonde une liste de "seeds" (Mac local + NAS), lit chaque /status, puis
// reconstruit tout le réseau à partir des pairs déclarés. Aucune saisie.

type NodeStatus = {
  node?: string;
  node_id?: string;
  platform?: string;
  node_appareil?: string;
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

// Endpoints sondés silencieusement. Couvre l'app Mac, des nœuds CLI locaux
// courants, et le NAS. Un seed muet est simplement ignoré.
const SEEDS = [
  'http://localhost:9091',
  'http://127.0.0.1:8085',
  'http://127.0.0.1:8086',
  'http://127.0.0.1:8080',
  'http://192.168.0.83:8085',
];

const REFRESH_MS = 2500;
const TIMEOUT_MS = 1400;

type LiveNode = { url: string; data: NodeStatus; latencyMs: number };

const $ = <T extends Element = HTMLElement>(id: string) => document.getElementById(id) as unknown as T;
const verdictEl = $('verdict');
const sublineEl = $('subline');
const liveCountEl = $('liveCount');
const pauseBtn = $('pauseBtn') as HTMLButtonElement;
const stageEl = $('stage');
const topoEl = $('topo') as unknown as SVGSVGElement;
const emptyHintEl = $('emptyHint');
const stripEl = $('strip');
const nodesEl = $('nodes');
const nodesHintEl = $('nodesHint');
const footEl = $('foot');

const ESC: Record<string, string> = { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' };
const esc = (s: unknown) => String(s ?? '').replace(/[&<>"']/g, (c) => ESC[c]!);

async function probe(url: string): Promise<LiveNode | null> {
  const ctrl = new AbortController();
  const t = window.setTimeout(() => ctrl.abort(), TIMEOUT_MS);
  const started = performance.now();
  try {
    const res = await fetch(`${url}/status`, { signal: ctrl.signal });
    if (!res.ok) return null;
    const data = (await res.json()) as NodeStatus;
    if (!data || !data.node_id) return null;
    return { url, data, latencyMs: Math.round(performance.now() - started) };
  } catch {
    return null;
  } finally {
    window.clearTimeout(t);
  }
}

function appearance(label: string): { icon: string; kind: string } {
  const l = label.toLowerCase();
  if (l.includes('nas') || l.includes('relay') || l.includes('serv')) return { icon: '🛰️', kind: 'nas' };
  if (l.includes('mac')) return { icon: '💻', kind: 'mac' };
  if (l.includes('tv') || l.includes('apple tv')) return { icon: '📺', kind: 'tv' };
  if (l.includes('pad')) return { icon: '▢', kind: 'ipad' };
  if (l.includes('phone')) return { icon: '📱', kind: 'iphone' };
  return { icon: '📡', kind: 'peer' };
}

// Phase brute du protocole → langage humain.
function humanPhase(phase?: string, peers = 0): { txt: string; health: 'ok' | 'warn' | 'down' } {
  const p = (phase ?? '').toLowerCase();
  if (p.includes('converg') || p === 'connecte' || p.includes('connect')) {
    return { txt: peers > 0 ? 'Connecté' : 'En ligne (seul)', health: peers > 0 ? 'ok' : 'warn' };
  }
  if (p.includes('relay')) return { txt: 'Via relais', health: 'ok' };
  if (p.includes('amorc') || p.includes('boot') || p.includes('discov')) return { txt: 'Démarrage…', health: 'warn' };
  if (!p) return { txt: 'En ligne', health: peers > 0 ? 'ok' : 'warn' };
  return { txt: phase as string, health: peers > 0 ? 'ok' : 'warn' };
}

const STATE_VARS: Record<string, string> = {
  ok: '--state:var(--ok);--state-glow:var(--glow-ok);--state-bg:var(--ok-dim);',
  warn: '--state:var(--warn);--state-glow:rgba(255,183,77,.4);--state-bg:var(--warn-dim);',
  down: '--state:var(--ko);--state-glow:rgba(255,97,120,.4);--state-bg:var(--ko-dim);',
};

function fmtUptime(s?: number): string {
  if (typeof s !== 'number') return '—';
  if (s < 60) return `${s}s`;
  if (s < 3600) return `${Math.floor(s / 60)}min`;
  if (s < 86400) return `${Math.floor(s / 3600)}h${String(Math.floor((s % 3600) / 60)).padStart(2, '0')}`;
  return `${Math.floor(s / 86400)}j ${Math.floor((s % 86400) / 3600)}h`;
}

function fmtNum(n?: number): string {
  if (typeof n !== 'number') return '—';
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
  return String(n);
}

type ResolvedNode = {
  id: string;
  short: string;
  label: string;
  icon: string;
  live: LiveNode | null;
  health: 'ok' | 'warn' | 'down';
  phaseTxt: string;
  peers: number;
};

// Agrège seeds joignables + pairs déclarés → vue unifiée du réseau.
function buildNetwork(live: LiveNode[]): ResolvedNode[] {
  const byId = new Map<string, LiveNode>();
  for (const ln of live) byId.set(ln.data.node_id!, ln);

  const ids = new Set<string>();
  for (const ln of live) {
    ids.add(ln.data.node_id!);
    for (const p of ln.data.pairs_connectes ?? []) ids.add(p);
  }

  const nodes: ResolvedNode[] = [];
  for (const id of ids) {
    const ln = byId.get(id) ?? null;
    const label = ln?.data.node ?? 'Pair distant';
    const { icon } = appearance(label);
    const peers = ln ? (ln.data.taille_reseau ?? ln.data.pairs_connectes?.length ?? 0) : 0;
    const ph = ln ? humanPhase(ln.data.phase, peers) : { txt: 'Vu par le réseau', health: 'ok' as const };
    nodes.push({
      id,
      short: id.slice(0, 8),
      label: ln ? label : `Pair · ${id.slice(0, 6)}`,
      icon: ln ? icon : '📡',
      live: ln,
      health: ph.health,
      phaseTxt: ph.txt,
      peers,
    });
  }
  // Live d'abord, puis par label.
  nodes.sort((a, b) => (a.live ? 0 : 1) - (b.live ? 0 : 1) || a.label.localeCompare(b.label));
  return nodes;
}

function uniqueEdges(live: LiveNode[]): Array<[string, string]> {
  const seen = new Set<string>();
  const edges: Array<[string, string]> = [];
  for (const ln of live) {
    const a = ln.data.node_id!;
    for (const b of ln.data.pairs_connectes ?? []) {
      const key = [a, b].sort().join('|');
      if (seen.has(key)) continue;
      seen.add(key);
      edges.push([a, b]);
    }
  }
  return edges;
}

const SVGNS = 'http://www.w3.org/2000/svg';
function svg(tag: string, attrs: Record<string, string | number>): SVGElement {
  const el = document.createElementNS(SVGNS, tag);
  for (const [k, v] of Object.entries(attrs)) el.setAttribute(k, String(v));
  return el;
}

function renderTopology(nodes: ResolvedNode[], edges: Array<[string, string]>): void {
  topoEl.innerHTML = '';
  const W = 1000;
  const H = 380;
  const cx = W / 2;
  const cy = H / 2;
  const R = nodes.length <= 1 ? 0 : Math.min(150, 90 + nodes.length * 12);
  const pos = new Map<string, { x: number; y: number }>();
  nodes.forEach((n, i) => {
    if (nodes.length === 1) {
      pos.set(n.id, { x: cx, y: cy });
      return;
    }
    const a = (i / nodes.length) * Math.PI * 2 - Math.PI / 2;
    pos.set(n.id, { x: cx + Math.cos(a) * R, y: cy + Math.sin(a) * R });
  });

  // Edges (sous les nœuds)
  for (const [a, b] of edges) {
    const pa = pos.get(a);
    const pb = pos.get(b);
    if (!pa || !pb) continue;
    const line = svg('line', { x1: pa.x, y1: pa.y, x2: pb.x, y2: pb.y });
    line.setAttribute('class', 'edge lit');
    topoEl.appendChild(line);
    // Pulse qui voyage le long du lien
    const dot = svg('circle', { r: 2.6, class: 'pulse' });
    const anim = svg('animateMotion', { dur: `${2.2 + Math.random()}s`, repeatCount: 'indefinite' });
    anim.setAttribute('path', `M ${pa.x} ${pa.y} L ${pb.x} ${pb.y}`);
    dot.appendChild(anim);
    topoEl.appendChild(dot);
  }

  // Nodes
  for (const n of nodes) {
    const p = pos.get(n.id)!;
    const color = n.health === 'ok' ? 'var(--ok)' : n.health === 'warn' ? 'var(--warn)' : 'var(--ko)';
    const g = svg('g', { class: 'node-hit' });
    const ring = svg('circle', { cx: p.x, cy: p.y, r: 26 });
    ring.setAttribute('class', 'node-ring');
    ring.setAttribute('stroke', color);
    const core = svg('circle', { cx: p.x, cy: p.y, r: n.live ? 19 : 13 });
    core.setAttribute('class', 'node-core');
    core.setAttribute('fill', n.live ? color : 'var(--faint)');
    core.setAttribute('color', color);
    core.setAttribute('opacity', n.live ? '0.95' : '0.5');
    const ico = svg('text', { x: p.x, y: p.y + 6, 'text-anchor': 'middle', 'font-size': 18 });
    ico.textContent = n.icon;
    const name = svg('text', { x: p.x, y: p.y + 46, 'text-anchor': 'middle' });
    name.setAttribute('class', 'node-name');
    name.textContent = n.label;
    const meta = svg('text', { x: p.x, y: p.y + 61, 'text-anchor': 'middle' });
    meta.setAttribute('class', 'node-meta');
    meta.textContent = n.short;
    g.append(ring, core, ico, name, meta);
    topoEl.appendChild(g);
  }
}

function renderStrip(nodes: ResolvedNode[], live: LiveNode[]): void {
  const online = nodes.length;
  const reachable = live.length;
  const sent = live.reduce((s, l) => s + (l.data.messages_envoyes ?? 0), 0);
  const recv = live.reduce((s, l) => s + (l.data.messages_recus ?? 0), 0);
  const groups = new Set<string>();
  for (const l of live) for (const g of l.data.groupes ?? []) groups.add(g.nom);

  const cells = [
    { lab: 'Nœuds sur le réseau', val: String(online), unit: '', spark: '🌐' },
    { lab: 'Avec métriques', val: String(reachable), unit: `/ ${online}`, spark: '📊' },
    { lab: 'Messages échangés', val: fmtNum(sent + recv), unit: '', spark: '✉️' },
    { lab: 'Groupes actifs', val: String(groups.size), unit: '', spark: '👥' },
  ];
  stripEl.innerHTML = cells
    .map(
      (c) => `<div class="metric"><span class="spark">${c.spark}</span>
        <div class="lab">${c.lab}</div>
        <div class="val">${c.val}${c.unit ? `<small>${c.unit}</small>` : ''}</div></div>`,
    )
    .join('');
}

function renderNodeCard(n: ResolvedNode, i: number): string {
  const stateCss = STATE_VARS[n.health];
  const d = n.live?.data;
  if (!n.live || !d) {
    return `<article class="node seen" style="${stateCss};animation-delay:${i * 45}ms">
      <div class="node-top">
        <div class="avatar">${n.icon}</div>
        <div class="node-id"><div class="nm">${esc(n.label)}</div>
          <div class="sub mono">${esc(n.short)}</div></div>
        <span class="pill">vu</span>
      </div>
      <div class="node-body"><div class="seen-note">
        Présent sur le réseau (annoncé par un autre nœud). Pas de page d'état exposée —
        normal pour un iPhone, iPad ou Apple&nbsp;TV.</div></div>
    </article>`;
  }
  const sent = fmtNum(d.messages_envoyes);
  const recv = fmtNum(d.messages_recus);
  const groups =
    Array.isArray(d.groupes) && d.groupes.length
      ? `<div class="groups">${d.groupes
          .map((g) => `<span class="gchip">${esc(g.nom)} · ${esc(g.membres)}</span>`)
          .join('')}</div>`
      : '';
  const relay = d.relay_url_active ? esc(d.relay_url_active) : 'auto (découverte)';
  return `<article class="node" style="${stateCss};animation-delay:${i * 45}ms">
    <div class="node-top">
      <div class="avatar">${n.icon}</div>
      <div class="node-id">
        <div class="nm">${esc(n.label)}</div>
        <div class="sub mono">${esc(n.short)} · ${esc(n.live.latencyMs)}ms</div>
      </div>
      <span class="pill">${esc(n.phaseTxt)}</span>
    </div>
    <div class="node-body">
      <div class="duo">
        <div class="stat"><div class="k">Pairs connectés</div><div class="v">${esc(n.peers)}</div></div>
        <div class="stat"><div class="k">En ligne depuis</div><div class="v">${fmtUptime(d.uptime_secondes)}</div></div>
        <div class="stat"><div class="k">Envoyés</div><div class="v">${sent}</div></div>
        <div class="stat"><div class="k">Reçus</div><div class="v">${recv}</div></div>
      </div>
      <div class="meta-row"><span>Rôle · ${esc(d.role ?? '—')}</span><span class="mono">${relay}</span></div>
      ${groups}
    </div>
  </article>`;
}

function setVerdict(nodes: ResolvedNode[], live: LiveNode[]): void {
  if (live.length === 0) {
    stageEl.classList.add('is-empty');
    verdictEl.className = 'verdict is-down';
    verdictEl.innerHTML = '<em>Hors ligne</em>';
    sublineEl.textContent = 'Aucun nœud joignable depuis ce Mac.';
    liveCountEl.textContent = '0 nœud';
    return;
  }
  stageEl.classList.remove('is-empty');
  const total = nodes.length;
  const healthy = nodes.filter((n) => n.health === 'ok').length;
  const warn = nodes.some((n) => n.health === 'warn');
  verdictEl.className = `verdict${warn && healthy < total ? ' is-warn' : ''}`;
  verdictEl.innerHTML = `<em>${total}</em> nœud${total > 1 ? 's' : ''} sur le réseau`;
  sublineEl.textContent =
    total > 1
      ? `Les nœuds se voient mutuellement${warn ? ' — un ou plusieurs sont encore en démarrage.' : ' et échangent des messages.'}`
      : 'Un seul nœud en ligne pour l’instant — lance-en un autre pour voir le maillage.';
  liveCountEl.textContent = `${live.length} actif${live.length > 1 ? 's' : ''}`;
}

let paused = false;
let timer: number | null = null;

async function tick(): Promise<void> {
  const probes = await Promise.all(SEEDS.map(probe));
  const live = probes.filter((x): x is LiveNode => x !== null);
  // Dédoublonne les seeds qui pointent le même node_id (ex. localhost vs IP).
  const seenIds = new Set<string>();
  const uniqLive = live.filter((l) => (seenIds.has(l.data.node_id!) ? false : seenIds.add(l.data.node_id!)));

  const nodes = buildNetwork(uniqLive);
  const edges = uniqueEdges(uniqLive);

  setVerdict(nodes, uniqLive);
  renderTopology(nodes, edges);
  renderStrip(nodes, uniqLive);
  nodesEl.innerHTML = nodes.map(renderNodeCard).join('');
  nodesHintEl.textContent = `découverte auto · maj ${new Date().toLocaleTimeString('fr-FR')}`;
  footEl.innerHTML = `ToM · réseau pair-à-pair décentralisé — chaque appareil est nœud et relais. Rafraîchissement toutes les ${REFRESH_MS / 1000}s.`;
  emptyHintEl.textContent = 'Lance l’app TomNode sur ce Mac (port 9091) ou vérifie que le NAS est en ligne.';
}

function loop(): void {
  if (paused) return;
  void tick();
  timer = window.setTimeout(loop, REFRESH_MS);
}

pauseBtn.addEventListener('click', () => {
  paused = !paused;
  pauseBtn.textContent = paused ? 'reprendre' : 'pause';
  if (!paused) loop();
  else if (timer) window.clearTimeout(timer);
});

void tick();
timer = window.setTimeout(loop, REFRESH_MS);
