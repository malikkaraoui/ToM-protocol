#!/usr/bin/env node
import { readFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { resolve } from 'node:path';

const PORT = Number(process.env.RELAY_DISCOVERY_PORT ?? 8080);
const RELAYS_FILE = resolve(process.env.RELAY_DISCOVERY_RELAYS_FILE ?? './tools/relay-discovery/relays.example.json');
const CHECK_TIMEOUT_MS = Number(process.env.RELAY_DISCOVERY_CHECK_TIMEOUT_MS ?? 2500);
const CACHE_TTL_MS = Number(process.env.RELAY_DISCOVERY_CACHE_TTL_MS ?? 30_000);
const RESPONSE_TTL_SECONDS = Number(process.env.RELAY_DISCOVERY_RESPONSE_TTL_SECONDS ?? 300);

/** @type {{ at: number, relays: any[] } | null} */
let healthyCache = null;

function json(res, status, payload) {
  const body = JSON.stringify(payload);
  res.writeHead(status, {
    'content-type': 'application/json; charset=utf-8',
    'cache-control': 'no-store',
  });
  res.end(body);
}

async function loadRelayCatalog() {
  const raw = await readFile(RELAYS_FILE, 'utf8');
  const data = JSON.parse(raw);
  if (!Array.isArray(data.relays)) {
    throw new Error('Invalid relay catalog: "relays" must be an array');
  }
  return data.relays;
}

async function fetchWithTimeout(url, timeoutMs) {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  try {
    return await fetch(url, { signal: controller.signal });
  } finally {
    clearTimeout(timer);
  }
}

async function isRelayHealthy(baseUrl) {
  const normalized = baseUrl.replace(/\/$/, '');
  const candidates = [`${normalized}/health`, `${normalized}/healthz`];

  for (const endpoint of candidates) {
    try {
      const res = await fetchWithTimeout(endpoint, CHECK_TIMEOUT_MS);
      if (!res.ok) continue;
      const contentType = res.headers.get('content-type') ?? '';
      if (contentType.includes('application/json')) {
        const payload = await res.json().catch(() => null);
        if (payload?.status === 'ok') return true;
      } else {
        // Accept plain 200 as healthy for compatibility.
        return true;
      }
    } catch {
      // try next endpoint
    }
  }
  return false;
}

async function computeHealthyRelays() {
  const now = Date.now();
  if (healthyCache && now - healthyCache.at < CACHE_TTL_MS) {
    return healthyCache.relays;
  }

  const catalog = await loadRelayCatalog();
  const checks = await Promise.all(
    catalog.map(async (relay) => {
      const healthy = await isRelayHealthy(relay.url);
      return healthy ? relay : null;
    }),
  );

  const healthyRelays = checks.filter(Boolean);
  healthyCache = { at: now, relays: healthyRelays };
  return healthyRelays;
}

const server = createServer(async (req, res) => {
  if (!req.url) return json(res, 400, { error: 'missing URL' });

  if (req.method === 'GET' && req.url === '/health') {
    return json(res, 200, {
      status: 'ok',
      service: 'relay-discovery',
      cache_ttl_ms: CACHE_TTL_MS,
      check_timeout_ms: CHECK_TIMEOUT_MS,
    });
  }

  if (req.method === 'GET' && req.url === '/relays') {
    try {
      const relays = await computeHealthyRelays();
      return json(res, 200, {
        relays,
        ttl_seconds: RESPONSE_TTL_SECONDS,
      });
    } catch (err) {
      return json(res, 500, {
        error: 'failed to compute relay list',
        detail: String(err?.message ?? err),
      });
    }
  }

  return json(res, 404, {
    error: 'not found',
    endpoints: ['GET /health', 'GET /relays'],
  });
});

server.listen(PORT, () => {
  console.log(`relay-discovery listening on http://0.0.0.0:${PORT}`);
  console.log(`relay catalog: ${RELAYS_FILE}`);
});
