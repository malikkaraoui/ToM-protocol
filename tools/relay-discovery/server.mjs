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

/** @type {{ requestsTotal: number, healthRequests: number, relaysRequests: number, metricsRequests: number, relayChecksTotal: number, cacheHits: number, cacheMisses: number, lastRefreshAt: number | null, lastError: string | null }} */
const stats = {
  requestsTotal: 0,
  healthRequests: 0,
  relaysRequests: 0,
  metricsRequests: 0,
  relayChecksTotal: 0,
  cacheHits: 0,
  cacheMisses: 0,
  lastRefreshAt: null,
  lastError: null,
};

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
  stats.relayChecksTotal += 1;
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
    stats.cacheHits += 1;
    return healthyCache.relays;
  }

  stats.cacheMisses += 1;

  const catalog = await loadRelayCatalog();
  const checks = await Promise.all(
    catalog.map(async (relay) => {
      const healthy = await isRelayHealthy(relay.url);
      return healthy ? relay : null;
    }),
  );

  const healthyRelays = checks.filter(Boolean);
  healthyCache = { at: now, relays: healthyRelays };
  stats.lastRefreshAt = now;
  return healthyRelays;
}

const server = createServer(async (req, res) => {
  if (!req.url) return json(res, 400, { error: 'missing URL' });
  stats.requestsTotal += 1;

  if (req.method === 'GET' && req.url === '/health') {
    stats.healthRequests += 1;
    return json(res, 200, {
      status: 'ok',
      service: 'relay-discovery',
      cache_ttl_ms: CACHE_TTL_MS,
      check_timeout_ms: CHECK_TIMEOUT_MS,
      cache_age_ms: healthyCache ? Date.now() - healthyCache.at : null,
      cached_relays: healthyCache ? healthyCache.relays.length : 0,
    });
  }

  if (req.method === 'GET' && req.url === '/relays') {
    stats.relaysRequests += 1;
    try {
      const relays = await computeHealthyRelays();
      stats.lastError = null;
      return json(res, 200, {
        relays,
        ttl_seconds: RESPONSE_TTL_SECONDS,
      });
    } catch (err) {
      stats.lastError = String(err?.message ?? err);
      return json(res, 500, {
        error: 'failed to compute relay list',
        detail: String(err?.message ?? err),
      });
    }
  }

  if (req.method === 'GET' && req.url === '/metrics') {
    stats.metricsRequests += 1;
    return json(res, 200, {
      status: 'ok',
      service: 'relay-discovery',
      counters: {
        requests_total: stats.requestsTotal,
        health_requests: stats.healthRequests,
        relays_requests: stats.relaysRequests,
        metrics_requests: stats.metricsRequests,
        relay_checks_total: stats.relayChecksTotal,
        cache_hits: stats.cacheHits,
        cache_misses: stats.cacheMisses,
      },
      cache: {
        ttl_ms: CACHE_TTL_MS,
        cached_relays: healthyCache ? healthyCache.relays.length : 0,
        last_refresh_at: stats.lastRefreshAt,
      },
      last_error: stats.lastError,
    });
  }

  return json(res, 404, {
    error: 'not found',
    endpoints: ['GET /health', 'GET /relays', 'GET /metrics'],
  });
});

server.listen(PORT, () => {
  console.log(`relay-discovery listening on http://0.0.0.0:${PORT}`);
  console.log(`relay catalog: ${RELAYS_FILE}`);
});
