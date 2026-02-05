import type { NetworkTopology, PeerInfo } from '../discovery/network-topology.js';
import type { NodeId } from '../identity/index.js';

export interface RelaySelectionResult {
  relayId: string | null;
  reason: 'best-available' | 'direct-fallback' | 'no-relays-available' | 'recipient-is-self' | 'no-peers';
}

/** Result from multi-relay path selection */
export interface RelayPathResult {
  /** Ordered list of relay IDs to traverse (empty = direct) */
  path: string[];
  /** Reason for path selection */
  reason: 'direct' | 'single-relay' | 'multi-relay' | 'no-path' | 'recipient-is-self';
}

/** Maximum relay chain depth to prevent infinite loops */
export const MAX_RELAY_DEPTH = 4;

export interface RelaySelectorOptions {
  selfNodeId: NodeId;
}

export class RelaySelector {
  private selfNodeId: NodeId;

  constructor(options: RelaySelectorOptions) {
    this.selfNodeId = options.selfNodeId;
  }

  /**
   * Select the best available relay for sending a message to a recipient.
   * Selection criteria (in order of priority):
   * 1. Must have 'relay' role
   * 2. Must be online (not stale or offline)
   * 3. Prefer most recently seen (highest lastSeen)
   *
   * @param to - The recipient node ID
   * @param topology - Network topology containing peer information
   * @param excludeRelays - Optional set of relay IDs to exclude (e.g., failed relays)
   */
  selectBestRelay(to: NodeId, topology: NetworkTopology, excludeRelays?: Set<string>): RelaySelectionResult {
    // Edge case: sending to self
    if (to === this.selfNodeId) {
      return { relayId: null, reason: 'recipient-is-self' };
    }

    // Edge case: empty topology
    if (topology.size() === 0) {
      return { relayId: null, reason: 'no-peers' };
    }

    // Check if recipient is directly reachable (exists in topology and is online)
    const recipient = topology.getPeer(to);
    if (recipient && topology.getPeerStatus(to) === 'online') {
      // In current iteration, we still use relay even for direct peers
      // But if no relays available, we can fall back to direct
    }

    // Get all relay nodes
    const relayNodes = topology.getRelayNodes();

    // Filter to only online relays (not stale, not offline)
    const onlineRelays = relayNodes.filter((relay) => {
      // Don't select self as relay
      if (relay.nodeId === this.selfNodeId) return false;
      // Don't select recipient as relay
      if (relay.nodeId === to) return false;
      // Exclude failed/unavailable relays
      if (excludeRelays?.has(relay.nodeId)) return false;
      // Must be online
      return topology.getPeerStatus(relay.nodeId) === 'online';
    });

    // If no online relays available
    if (onlineRelays.length === 0) {
      // Fallback: in minimal networks (2-3 nodes), allow direct sending
      // This handles bootstrap phase before relays are assigned
      if (recipient && topology.getPeerStatus(to) === 'online') {
        return { relayId: null, reason: 'direct-fallback' };
      }
      return { relayId: null, reason: 'no-relays-available' };
    }

    // Sort by lastSeen (most recent first)
    const sortedRelays = onlineRelays.sort((a, b) => b.lastSeen - a.lastSeen);

    // Return the best relay
    const bestRelay = sortedRelays[0] as PeerInfo;
    return { relayId: bestRelay.nodeId, reason: 'best-available' };
  }

  /**
   * Select an alternate relay for rerouting, excluding failed relays.
   * This is used when the primary relay fails and we need to reroute.
   *
   * @param to - The recipient node ID
   * @param topology - Network topology containing peer information
   * @param failedRelays - Set of relay IDs that have failed
   */
  selectAlternateRelay(to: NodeId, topology: NetworkTopology, failedRelays: Set<string>): RelaySelectionResult {
    return this.selectBestRelay(to, topology, failedRelays);
  }

  /**
   * Select a multi-relay path to reach a recipient.
   * Uses BFS to find the shortest path through relay nodes.
   *
   * Path selection strategy:
   * 1. If recipient is directly reachable via a relay we can reach, use single relay
   * 2. If recipient is reachable via a relay that's reachable via another relay, chain them
   * 3. Max depth of MAX_RELAY_DEPTH to prevent infinite loops
   */
  selectPathToRecipient(to: NodeId, topology: NetworkTopology): RelayPathResult {
    // Edge case: sending to self
    if (to === this.selfNodeId) {
      return { path: [], reason: 'recipient-is-self' };
    }

    // Edge case: empty topology
    if (topology.size() === 0) {
      return { path: [], reason: 'no-path' };
    }

    // Check if recipient is directly reachable (we have direct connection)
    const recipient = topology.getPeer(to);
    if (recipient && topology.getPeerStatus(to) === 'online') {
      // Recipient is directly reachable, no relays needed
      return { path: [], reason: 'direct' };
    }

    // Get all online relays
    const onlineRelays = topology.getRelayNodes().filter((relay) => {
      if (relay.nodeId === this.selfNodeId) return false;
      if (relay.nodeId === to) return false;
      return topology.getPeerStatus(relay.nodeId) === 'online';
    });

    if (onlineRelays.length === 0) {
      return { path: [], reason: 'no-path' };
    }

    // Simple case: check if any single relay can reach the recipient
    // In current architecture, if recipient was announced via a relay, that relay knows them
    for (const relay of onlineRelays) {
      // For now, assume any online relay can forward to any recipient
      // This will be refined when we have proper reachability info
      return { path: [relay.nodeId], reason: 'single-relay' };
    }

    // BFS for multi-relay path (when we have reachability info)
    const path = this.findMultiRelayPath(to, onlineRelays, topology);
    if (path.length > 0) {
      return { path, reason: 'multi-relay' };
    }

    return { path: [], reason: 'no-path' };
  }

  /**
   * Find a multi-relay path using BFS.
   * This is used when the recipient isn't directly reachable by any single relay.
   */
  private findMultiRelayPath(to: NodeId, relays: PeerInfo[], topology: NetworkTopology): string[] {
    // Build adjacency: which relays can reach which other relays/recipients
    const canReach = new Map<string, Set<string>>();

    for (const relay of relays) {
      const reachable = new Set<string>();
      // A relay can reach anyone in the topology (simplified model)
      // In a more sophisticated model, we'd use reachableVia info
      for (const peer of topology.getReachablePeers()) {
        if (peer.nodeId !== relay.nodeId && peer.nodeId !== this.selfNodeId) {
          reachable.add(peer.nodeId);
        }
      }
      canReach.set(relay.nodeId, reachable);
    }

    // BFS from our directly connected relays
    const visited = new Set<string>();
    const queue: { nodeId: string; path: string[] }[] = [];

    // Start with relays we can directly reach
    for (const relay of relays) {
      queue.push({ nodeId: relay.nodeId, path: [relay.nodeId] });
      visited.add(relay.nodeId);
    }

    while (queue.length > 0) {
      const current = queue.shift()!;

      // Check depth limit
      if (current.path.length >= MAX_RELAY_DEPTH) {
        continue;
      }

      const reachableFromCurrent = canReach.get(current.nodeId);
      if (!reachableFromCurrent) continue;

      // Check if current relay can reach the target
      if (reachableFromCurrent.has(to)) {
        return current.path;
      }

      // Explore next level of relays
      for (const nextRelay of relays) {
        if (!visited.has(nextRelay.nodeId) && reachableFromCurrent.has(nextRelay.nodeId)) {
          visited.add(nextRelay.nodeId);
          queue.push({
            nodeId: nextRelay.nodeId,
            path: [...current.path, nextRelay.nodeId],
          });
        }
      }
    }

    return []; // No path found
  }
}
