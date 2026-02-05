import { beforeEach, describe, expect, it, vi } from 'vitest';
import { PeerGossip, isPeerGossipMessage } from './peer-gossip.js';
import type { GossipPeerInfo, PeerGossipEvents } from './peer-gossip.js';

describe('PeerGossip', () => {
  let gossip: PeerGossip;
  let events: PeerGossipEvents;
  let discoveredPeers: GossipPeerInfo[];
  let requestedFrom: string[];

  beforeEach(() => {
    discoveredPeers = [];
    requestedFrom = [];
    events = {
      onPeersDiscovered: (peers, _via) => {
        discoveredPeers.push(...peers);
      },
      onPeerListRequested: (from, _requestId) => {
        requestedFrom.push(from);
      },
    };
    gossip = new PeerGossip('self-node-id', 'alice', events);
  });

  describe('addBootstrapPeer', () => {
    it('should add a peer from bootstrap', () => {
      gossip.addBootstrapPeer({
        nodeId: 'peer-1',
        username: 'bob',
        encryptionKey: 'key-1',
      });

      const peers = gossip.getKnownPeers();
      expect(peers).toHaveLength(1);
      expect(peers[0].nodeId).toBe('peer-1');
      expect(peers[0].discoverySource).toBe('bootstrap');
    });

    it('should not add self as a peer', () => {
      gossip.addBootstrapPeer({
        nodeId: 'self-node-id',
        username: 'alice',
      });

      expect(gossip.getKnownPeers()).toHaveLength(0);
    });

    it('should update existing peer info while keeping discovery source', () => {
      gossip.addBootstrapPeer({
        nodeId: 'peer-1',
        username: 'bob',
      });

      gossip.addBootstrapPeer({
        nodeId: 'peer-1',
        username: 'bob-updated',
        encryptionKey: 'new-key',
      });

      const peers = gossip.getKnownPeers();
      expect(peers).toHaveLength(1);
      expect(peers[0].username).toBe('bob-updated');
      expect(peers[0].encryptionKey).toBe('new-key');
      expect(peers[0].discoverySource).toBe('bootstrap');
    });
  });

  describe('createPeerListRequest', () => {
    it('should create a valid request message', () => {
      gossip.setSelfEncryptionKey('my-enc-key');
      const request = gossip.createPeerListRequest();

      expect(request.type).toBe('peer-list-request');
      expect(request.requestId).toMatch(/^gossip-\d+-[a-z0-9]+$/);
      expect(request.from?.nodeId).toBe('self-node-id');
      expect(request.from?.username).toBe('alice');
      expect(request.from?.encryptionKey).toBe('my-enc-key');
    });
  });

  describe('createPeerListResponse', () => {
    it('should include known peers in response', () => {
      gossip.addBootstrapPeer({ nodeId: 'peer-1', username: 'bob' });
      gossip.addBootstrapPeer({ nodeId: 'peer-2', username: 'charlie' });

      const response = gossip.createPeerListResponse('req-123');

      expect(response.type).toBe('peer-list-response');
      expect(response.requestId).toBe('req-123');
      expect(response.peers).toHaveLength(2);
    });

    it('should not include self in response', () => {
      gossip.addBootstrapPeer({ nodeId: 'peer-1', username: 'bob' });

      const response = gossip.createPeerListResponse('req-123');

      expect(response.peers?.some((p) => p.nodeId === 'self-node-id')).toBe(false);
    });

    it('should respect maxPeersPerResponse', () => {
      const smallGossip = new PeerGossip('self', 'alice', events, {
        maxPeersPerResponse: 2,
      });

      for (let i = 0; i < 5; i++) {
        smallGossip.addBootstrapPeer({ nodeId: `peer-${i}`, username: `user-${i}` });
      }

      const response = smallGossip.createPeerListResponse('req-123');
      expect(response.peers?.length).toBeLessThanOrEqual(2);
    });
  });

  describe('handleMessage', () => {
    it('should respond to peer list request', () => {
      gossip.addBootstrapPeer({ nodeId: 'peer-1', username: 'bob' });

      const request = {
        type: 'peer-list-request' as const,
        requestId: 'req-123',
        from: {
          nodeId: 'requester',
          username: 'charlie',
          discoverySource: 'bootstrap' as const,
          discoveredAt: Date.now(),
        },
      };

      const response = gossip.handleMessage(request, 'requester');

      expect(response?.type).toBe('peer-list-response');
      expect(response?.requestId).toBe('req-123');
      expect(requestedFrom).toContain('requester');
    });

    it('should add requester to known peers from request', () => {
      const request = {
        type: 'peer-list-request' as const,
        requestId: 'req-123',
        from: {
          nodeId: 'new-peer',
          username: 'newbie',
          encryptionKey: 'new-key',
          discoverySource: 'bootstrap' as const,
          discoveredAt: Date.now(),
        },
      };

      gossip.handleMessage(request, 'new-peer');

      const peers = gossip.getKnownPeers();
      const newPeer = peers.find((p) => p.nodeId === 'new-peer');
      expect(newPeer).toBeDefined();
      expect(newPeer?.discoverySource).toBe('gossip');
    });

    it('should discover new peers from response', () => {
      const response = {
        type: 'peer-list-response' as const,
        requestId: 'req-123',
        peers: [
          {
            nodeId: 'discovered-1',
            username: 'disc1',
            encryptionKey: 'key1',
            discoverySource: 'bootstrap' as const,
            discoveredAt: Date.now(),
          },
          {
            nodeId: 'discovered-2',
            username: 'disc2',
            discoverySource: 'gossip' as const,
            discoveredAt: Date.now(),
          },
        ],
      };

      gossip.handleMessage(response, 'responder');

      expect(discoveredPeers).toHaveLength(2);
      expect(discoveredPeers[0].discoverySource).toBe('gossip'); // Overridden
      expect(gossip.isGossipDiscovered('discovered-1')).toBe(true);
    });

    it('should not add already known peers', () => {
      gossip.addBootstrapPeer({ nodeId: 'existing', username: 'existing' });

      const response = {
        type: 'peer-list-response' as const,
        requestId: 'req-123',
        peers: [
          {
            nodeId: 'existing',
            username: 'existing-updated',
            discoverySource: 'gossip' as const,
            discoveredAt: Date.now(),
          },
        ],
      };

      gossip.handleMessage(response, 'responder');

      expect(discoveredPeers).toHaveLength(0); // No new peers
      expect(gossip.isGossipDiscovered('existing')).toBe(false); // Still bootstrap
    });

    it('should not add self from response', () => {
      const response = {
        type: 'peer-list-response' as const,
        requestId: 'req-123',
        peers: [
          {
            nodeId: 'self-node-id',
            username: 'alice',
            discoverySource: 'bootstrap' as const,
            discoveredAt: Date.now(),
          },
        ],
      };

      gossip.handleMessage(response, 'responder');

      expect(gossip.getKnownPeers()).toHaveLength(0);
    });
  });

  describe('connection tracking', () => {
    it('should track connected peers', () => {
      gossip.markConnected('peer-1');
      gossip.markConnected('peer-2');

      const stats = gossip.getStats();
      expect(stats.connectedPeers).toBe(2);
    });

    it('should remove disconnected peers from connected set', () => {
      gossip.markConnected('peer-1');
      gossip.markDisconnected('peer-1');

      const stats = gossip.getStats();
      expect(stats.connectedPeers).toBe(0);
    });
  });

  describe('removePeer', () => {
    it('should remove peer from all tracking', () => {
      gossip.addBootstrapPeer({ nodeId: 'peer-1', username: 'bob' });
      gossip.markConnected('peer-1');

      gossip.removePeer('peer-1');

      expect(gossip.getKnownPeers()).toHaveLength(0);
      expect(gossip.getStats().connectedPeers).toBe(0);
    });
  });

  describe('getStats', () => {
    it('should return accurate discovery statistics', () => {
      // Add 2 bootstrap peers
      gossip.addBootstrapPeer({ nodeId: 'boot-1', username: 'b1' });
      gossip.addBootstrapPeer({ nodeId: 'boot-2', username: 'b2' });

      // Simulate discovering 1 peer via gossip
      const response = {
        type: 'peer-list-response' as const,
        requestId: 'req-123',
        peers: [
          {
            nodeId: 'gossip-1',
            username: 'g1',
            discoverySource: 'gossip' as const,
            discoveredAt: Date.now(),
          },
        ],
      };
      gossip.handleMessage(response, 'boot-1');

      gossip.markConnected('boot-1');

      const stats = gossip.getStats();
      expect(stats.totalPeers).toBe(3);
      expect(stats.bootstrapPeers).toBe(2);
      expect(stats.gossipPeers).toBe(1);
      expect(stats.connectedPeers).toBe(1);
    });
  });

  describe('getPeersToGossipWith', () => {
    it('should return connected peers not recently requested', () => {
      gossip.markConnected('peer-1');
      gossip.markConnected('peer-2');

      const peers = gossip.getPeersToGossipWith();
      expect(peers).toContain('peer-1');
      expect(peers).toContain('peer-2');
    });

    it('should exclude peers requested recently', () => {
      const shortIntervalGossip = new PeerGossip('self', 'alice', events, {
        minRequestIntervalMs: 60000, // 60 seconds
      });
      shortIntervalGossip.markConnected('peer-1');

      // Mark as recently requested
      const request = shortIntervalGossip.createPeerListRequest();
      shortIntervalGossip.markRequestSent('peer-1', request.requestId);

      const peers = shortIntervalGossip.getPeersToGossipWith();
      expect(peers).not.toContain('peer-1');
    });
  });
});

describe('isPeerGossipMessage', () => {
  it('should return true for peer-list-request', () => {
    expect(
      isPeerGossipMessage({
        type: 'peer-list-request',
        requestId: 'req-123',
      }),
    ).toBe(true);
  });

  it('should return true for peer-list-response', () => {
    expect(
      isPeerGossipMessage({
        type: 'peer-list-response',
        requestId: 'req-123',
        peers: [],
      }),
    ).toBe(true);
  });

  it('should return false for invalid messages', () => {
    expect(isPeerGossipMessage(null)).toBe(false);
    expect(isPeerGossipMessage({})).toBe(false);
    expect(isPeerGossipMessage({ type: 'other' })).toBe(false);
    expect(isPeerGossipMessage({ type: 'peer-list-request' })).toBe(false); // Missing requestId
  });
});
