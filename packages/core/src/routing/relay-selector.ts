import type { NetworkTopology, PeerInfo } from '../discovery/network-topology.js';
import type { NodeId } from '../identity/index.js';

export interface RelaySelectionResult {
  relayId: string | null;
  reason: 'best-available' | 'direct-path' | 'no-relays-available' | 'recipient-is-self' | 'no-peers';
}

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
   */
  selectBestRelay(to: NodeId, topology: NetworkTopology): RelaySelectionResult {
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
      // Must be online
      return topology.getPeerStatus(relay.nodeId) === 'online';
    });

    // If no online relays available
    if (onlineRelays.length === 0) {
      return { relayId: null, reason: 'no-relays-available' };
    }

    // Sort by lastSeen (most recent first)
    const sortedRelays = onlineRelays.sort((a, b) => b.lastSeen - a.lastSeen);

    // Return the best relay
    const bestRelay = sortedRelays[0] as PeerInfo;
    return { relayId: bestRelay.nodeId, reason: 'best-available' };
  }
}
