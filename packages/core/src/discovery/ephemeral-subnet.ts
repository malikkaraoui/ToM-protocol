/**
 * Ephemeral Subnet Module (Story 7.2 - Sliding Genesis)
 *
 * Implements self-organizing subnets that form based on communication patterns
 * and dissolve when no longer useful. This enables the network to optimize
 * routing at a granular level without manual intervention.
 *
 * Key concepts:
 * - Subnets form when nodes frequently communicate with each other
 * - Each subnet has a "sliding genesis" - it exists only as long as it serves a purpose
 * - Subnets dissolve automatically when inactive or membership drops
 * - Routing can prefer intra-subnet paths for lower latency
 *
 * @see architecture.md for subnet design rationale
 */

import type { NodeId } from '../identity/keypair.js';

// ============================================
// Types
// ============================================

export interface SubnetInfo {
  /** Unique subnet identifier */
  subnetId: string;
  /** Member node IDs */
  members: Set<NodeId>;
  /** When this subnet was formed */
  formedAt: number;
  /** Last activity timestamp */
  lastActivity: number;
  /** Communication density score (higher = more active) */
  densityScore: number;
}

export interface CommunicationEdge {
  /** Source node */
  from: NodeId;
  /** Target node */
  to: NodeId;
  /** Number of messages exchanged */
  messageCount: number;
  /** Last communication timestamp */
  lastSeen: number;
}

export interface SubnetEvents {
  /** Called when a new subnet forms */
  onSubnetFormed: (subnet: SubnetInfo) => void;
  /** Called when a subnet dissolves */
  onSubnetDissolved: (subnetId: string, reason: 'inactive' | 'insufficient-members') => void;
  /** Called when a node joins a subnet */
  onNodeJoinedSubnet: (subnetId: string, nodeId: NodeId) => void;
  /** Called when a node leaves a subnet */
  onNodeLeftSubnet: (subnetId: string, nodeId: NodeId) => void;
}

export interface SubnetConfig {
  /** Minimum messages between a pair to consider them connected */
  minEdgeWeight?: number;
  /** Minimum cluster size to form a subnet */
  minSubnetSize?: number;
  /** Maximum subnet size */
  maxSubnetSize?: number;
  /** Inactivity timeout before subnet dissolution (ms) */
  inactivityTimeoutMs?: number;
  /** How often to evaluate subnet formation/dissolution (ms) */
  evaluationIntervalMs?: number;
  /** Communication edge decay time - edges older than this lose weight (ms) */
  edgeDecayTimeMs?: number;
}

// ============================================
// Ephemeral Subnet Manager
// ============================================

const DEFAULT_CONFIG: Required<SubnetConfig> = {
  minEdgeWeight: 3, // At least 3 messages to consider nodes connected
  minSubnetSize: 3, // Minimum 3 nodes to form a subnet
  maxSubnetSize: 10, // Maximum 10 nodes per subnet
  inactivityTimeoutMs: 5 * 60 * 1000, // 5 minutes
  evaluationIntervalMs: 30 * 1000, // Evaluate every 30 seconds
  edgeDecayTimeMs: 10 * 60 * 1000, // Edges decay after 10 minutes
};

export class EphemeralSubnetManager {
  private selfNodeId: NodeId;
  private events: SubnetEvents;
  private config: Required<SubnetConfig>;

  /** Communication graph - tracks message exchanges */
  private communicationGraph = new Map<string, CommunicationEdge>();
  /** Active subnets */
  private subnets = new Map<string, SubnetInfo>();
  /** Node to subnet mapping */
  private nodeSubnets = new Map<NodeId, string>();
  /** Evaluation interval timer */
  private evaluationInterval: ReturnType<typeof setInterval> | null = null;

  constructor(selfNodeId: NodeId, events: SubnetEvents, config?: SubnetConfig) {
    this.selfNodeId = selfNodeId;
    this.events = events;
    this.config = { ...DEFAULT_CONFIG, ...config };
  }

  /**
   * Start the subnet manager
   */
  start(): void {
    if (this.evaluationInterval) return;

    this.evaluationInterval = setInterval(() => {
      this.evaluateSubnets();
    }, this.config.evaluationIntervalMs);
  }

  /**
   * Stop the subnet manager
   */
  stop(): void {
    if (this.evaluationInterval) {
      clearInterval(this.evaluationInterval);
      this.evaluationInterval = null;
    }
  }

  /**
   * Record a communication between two nodes
   */
  recordCommunication(from: NodeId, to: NodeId): void {
    // Create edge key (sorted for consistency)
    const edgeKey = [from, to].sort().join(':');

    const existing = this.communicationGraph.get(edgeKey);
    if (existing) {
      existing.messageCount++;
      existing.lastSeen = Date.now();
    } else {
      this.communicationGraph.set(edgeKey, {
        from,
        to,
        messageCount: 1,
        lastSeen: Date.now(),
      });
    }

    // Update subnet activity if nodes are in the same subnet
    const fromSubnet = this.nodeSubnets.get(from);
    const toSubnet = this.nodeSubnets.get(to);
    if (fromSubnet && fromSubnet === toSubnet) {
      const subnet = this.subnets.get(fromSubnet);
      if (subnet) {
        subnet.lastActivity = Date.now();
      }
    }
  }

  /**
   * Evaluate and update subnets based on communication patterns
   */
  private evaluateSubnets(): void {
    const now = Date.now();

    // 1. Decay old edges
    this.decayEdges(now);

    // 2. Check for inactive subnets and dissolve them
    this.dissolveInactiveSubnets(now);

    // 3. Try to form new subnets from communication clusters
    this.formNewSubnets();
  }

  /**
   * Decay edges that are too old
   */
  private decayEdges(now: number): void {
    for (const [key, edge] of this.communicationGraph) {
      const age = now - edge.lastSeen;
      if (age > this.config.edgeDecayTimeMs) {
        // Reduce weight over time
        const decayFactor = Math.max(0, 1 - (age / this.config.edgeDecayTimeMs - 1));
        edge.messageCount = Math.floor(edge.messageCount * decayFactor);

        // Remove edge if weight drops to 0
        if (edge.messageCount <= 0) {
          this.communicationGraph.delete(key);
        }
      }
    }
  }

  /**
   * Dissolve subnets that are inactive or have too few members
   */
  private dissolveInactiveSubnets(now: number): void {
    for (const [subnetId, subnet] of this.subnets) {
      const inactiveTime = now - subnet.lastActivity;

      if (subnet.members.size < this.config.minSubnetSize) {
        this.dissolveSubnet(subnetId, 'insufficient-members');
      } else if (inactiveTime > this.config.inactivityTimeoutMs) {
        this.dissolveSubnet(subnetId, 'inactive');
      }
    }
  }

  /**
   * Try to form new subnets from communication clusters
   */
  private formNewSubnets(): void {
    // Build adjacency list from strong edges
    const adjacency = new Map<NodeId, Set<NodeId>>();

    for (const edge of this.communicationGraph.values()) {
      if (edge.messageCount < this.config.minEdgeWeight) continue;

      // Skip if both nodes are already in the same subnet
      const fromSubnet = this.nodeSubnets.get(edge.from);
      const toSubnet = this.nodeSubnets.get(edge.to);
      if (fromSubnet && fromSubnet === toSubnet) continue;

      // Add to adjacency
      if (!adjacency.has(edge.from)) adjacency.set(edge.from, new Set());
      if (!adjacency.has(edge.to)) adjacency.set(edge.to, new Set());
      adjacency.get(edge.from)!.add(edge.to);
      adjacency.get(edge.to)!.add(edge.from);
    }

    // Find clusters using simple BFS
    const visited = new Set<NodeId>();

    for (const [startNode] of adjacency) {
      if (visited.has(startNode)) continue;
      if (this.nodeSubnets.has(startNode)) continue;

      // BFS to find cluster
      const cluster = new Set<NodeId>();
      const queue = [startNode];

      while (queue.length > 0 && cluster.size < this.config.maxSubnetSize) {
        const node = queue.shift()!;
        if (visited.has(node)) continue;

        visited.add(node);
        cluster.add(node);

        const neighbors = adjacency.get(node);
        if (neighbors) {
          for (const neighbor of neighbors) {
            if (!visited.has(neighbor) && !this.nodeSubnets.has(neighbor)) {
              queue.push(neighbor);
            }
          }
        }
      }

      // Form subnet if cluster is large enough
      if (cluster.size >= this.config.minSubnetSize) {
        this.formSubnet(cluster);
      }
    }
  }

  /**
   * Form a new subnet from a cluster of nodes
   */
  private formSubnet(members: Set<NodeId>): void {
    const subnetId = `subnet-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;

    const subnet: SubnetInfo = {
      subnetId,
      members: new Set(members),
      formedAt: Date.now(),
      lastActivity: Date.now(),
      densityScore: this.calculateDensity(members),
    };

    this.subnets.set(subnetId, subnet);

    for (const nodeId of members) {
      this.nodeSubnets.set(nodeId, subnetId);
      this.events.onNodeJoinedSubnet(subnetId, nodeId);
    }

    this.events.onSubnetFormed(subnet);
  }

  /**
   * Dissolve a subnet
   */
  private dissolveSubnet(subnetId: string, reason: 'inactive' | 'insufficient-members'): void {
    const subnet = this.subnets.get(subnetId);
    if (!subnet) return;

    for (const nodeId of subnet.members) {
      this.nodeSubnets.delete(nodeId);
      this.events.onNodeLeftSubnet(subnetId, nodeId);
    }

    this.subnets.delete(subnetId);
    this.events.onSubnetDissolved(subnetId, reason);
  }

  /**
   * Calculate communication density of a node set
   */
  private calculateDensity(nodes: Set<NodeId>): number {
    let totalWeight = 0;
    const nodeArray = Array.from(nodes);

    for (let i = 0; i < nodeArray.length; i++) {
      for (let j = i + 1; j < nodeArray.length; j++) {
        const edgeKey = [nodeArray[i], nodeArray[j]].sort().join(':');
        const edge = this.communicationGraph.get(edgeKey);
        if (edge) {
          totalWeight += edge.messageCount;
        }
      }
    }

    // Normalize by potential edges
    const potentialEdges = (nodes.size * (nodes.size - 1)) / 2;
    return potentialEdges > 0 ? totalWeight / potentialEdges : 0;
  }

  /**
   * Remove a node from any subnet it's in
   */
  removeNode(nodeId: NodeId): void {
    const subnetId = this.nodeSubnets.get(nodeId);
    if (!subnetId) return;

    const subnet = this.subnets.get(subnetId);
    if (!subnet) return;

    subnet.members.delete(nodeId);
    this.nodeSubnets.delete(nodeId);
    this.events.onNodeLeftSubnet(subnetId, nodeId);

    // Check if subnet should dissolve
    if (subnet.members.size < this.config.minSubnetSize) {
      this.dissolveSubnet(subnetId, 'insufficient-members');
    }
  }

  /**
   * Get the subnet a node belongs to
   */
  getNodeSubnet(nodeId: NodeId): SubnetInfo | null {
    const subnetId = this.nodeSubnets.get(nodeId);
    if (!subnetId) return null;
    return this.subnets.get(subnetId) ?? null;
  }

  /**
   * Check if two nodes are in the same subnet
   */
  areInSameSubnet(nodeA: NodeId, nodeB: NodeId): boolean {
    const subnetA = this.nodeSubnets.get(nodeA);
    const subnetB = this.nodeSubnets.get(nodeB);
    return subnetA !== undefined && subnetA === subnetB;
  }

  /**
   * Get all active subnets
   */
  getAllSubnets(): SubnetInfo[] {
    return Array.from(this.subnets.values());
  }

  /**
   * Get subnet statistics
   */
  getStats(): {
    totalSubnets: number;
    totalNodesInSubnets: number;
    averageSubnetSize: number;
    communicationEdges: number;
  } {
    const subnets = this.getAllSubnets();
    const totalNodes = subnets.reduce((sum, s) => sum + s.members.size, 0);

    return {
      totalSubnets: subnets.length,
      totalNodesInSubnets: totalNodes,
      averageSubnetSize: subnets.length > 0 ? totalNodes / subnets.length : 0,
      communicationEdges: this.communicationGraph.size,
    };
  }

  /**
   * Force subnet evaluation (for testing purposes)
   */
  forceEvaluate(): void {
    this.evaluateSubnets();
  }
}
