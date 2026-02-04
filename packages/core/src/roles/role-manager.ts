import type { NetworkTopology, NodeRole } from '../discovery/network-topology.js';
import type { NodeId } from '../identity/index.js';

export interface RoleAssignment {
  nodeId: NodeId;
  roles: NodeRole[];
  assignedAt: number;
}

export interface RoleManagerEvents {
  onRoleChanged: (nodeId: string, oldRoles: NodeRole[], newRoles: NodeRole[]) => void;
}

/** Re-evaluation interval (ms) */
const REEVALUATION_INTERVAL_MS = 30000;

export class RoleManager {
  private assignments = new Map<NodeId, RoleAssignment>();
  private events: RoleManagerEvents;
  private reevaluationTimer: ReturnType<typeof setInterval> | null = null;
  private topology: NetworkTopology | null = null;
  private selfNodeId: NodeId | null = null;

  constructor(events: RoleManagerEvents) {
    this.events = events;
  }

  /** Bind to a topology instance for periodic re-evaluation */
  bindTopology(topology: NetworkTopology, selfNodeId?: NodeId): void {
    this.topology = topology;
    if (selfNodeId) {
      this.selfNodeId = selfNodeId;
    }
  }

  /** Start periodic role re-evaluation */
  start(): void {
    if (this.reevaluationTimer) return;
    this.reevaluationTimer = setInterval(() => {
      if (this.topology) {
        this.reassignRoles(this.topology, this.selfNodeId ?? undefined);
      }
    }, REEVALUATION_INTERVAL_MS);
  }

  /** Stop periodic re-evaluation */
  stop(): void {
    if (this.reevaluationTimer) {
      clearInterval(this.reevaluationTimer);
      this.reevaluationTimer = null;
    }
  }

  /**
   * Evaluate and assign roles for a single node using deterministic consensus.
   *
   * Consensus rule: Lowest NodeId wins (lexicographic ordering)
   * - All nodes independently compute the same relay list
   * - No race conditions or communication needed
   */
  evaluateNode(nodeId: NodeId, topology: NetworkTopology): NodeRole[] {
    // Use ALL known peers (from signaling), not just those with heartbeats
    // This ensures all nodes have the same view for consensus
    const allPeers = topology.getReachablePeers();

    // Build list of all node IDs (peers + self if we know selfNodeId)
    const allNodeIds = allPeers.map((p) => p.nodeId);
    if (this.selfNodeId && !allNodeIds.includes(this.selfNodeId)) {
      allNodeIds.push(this.selfNodeId);
    }

    // Sort lexicographically - deterministic ordering
    allNodeIds.sort();

    const totalNodes = allNodeIds.length;
    const targetRelayCount = Math.max(1, Math.ceil(totalNodes / 3));

    // First N nodes in sorted order become relays
    const relayNodeIds = allNodeIds.slice(0, targetRelayCount);
    const shouldBeRelay = relayNodeIds.includes(nodeId);

    const newRoles: NodeRole[] = shouldBeRelay ? ['client', 'relay'] : ['client'];

    // DEBUG: Log consensus calculation
    const sortedShort = allNodeIds.map((n) => n.slice(0, 8)).join(',');
    const electedShort = relayNodeIds.map((n) => n.slice(0, 8)).join(',');
    console.log(
      `[RoleManager] evaluateNode(${nodeId.slice(0, 8)}): ` +
        `total=${totalNodes}, relays=${targetRelayCount}, ` +
        `sorted=[${sortedShort}], elected=[${electedShort}], result=${newRoles.join(',')}`,
    );

    this.setRoles(nodeId, newRoles);
    return newRoles;
  }

  /** Re-evaluate roles for all peers in the topology plus self */
  reassignRoles(topology: NetworkTopology, selfNodeId?: NodeId): Map<string, NodeRole[]> {
    const result = new Map<string, NodeRole[]>();
    // Use ALL known peers for role assignment (same as evaluateNode)
    const allPeers = topology.getReachablePeers();

    // Cleanup stale assignments for peers no longer in topology
    this.cleanupStaleAssignments(topology);

    // Evaluate self first (if provided)
    if (selfNodeId) {
      const selfRoles = this.evaluateNode(selfNodeId, topology);
      result.set(selfNodeId, selfRoles);
    }

    for (const peer of allPeers) {
      const roles = this.evaluateNode(peer.nodeId, topology);
      result.set(peer.nodeId, roles);
    }

    return result;
  }

  /** Remove assignments for peers that are no longer in topology (but never remove self) */
  private cleanupStaleAssignments(topology: NetworkTopology): void {
    const toRemove: NodeId[] = [];

    for (const [nodeId] of this.assignments) {
      // Never remove self's assignment
      if (nodeId === this.selfNodeId) continue;

      // Only remove if peer is completely gone from topology (left via signaling)
      const peer = topology.getPeer(nodeId);
      if (!peer) {
        toRemove.push(nodeId);
      }
    }

    for (const nodeId of toRemove) {
      this.assignments.delete(nodeId);
    }
  }

  /** Get current roles for a node */
  getCurrentRoles(nodeId: NodeId): NodeRole[] {
    const assignment = this.assignments.get(nodeId);
    return assignment ? [...assignment.roles] : ['client'];
  }

  /** Set roles externally (e.g., from signaling broadcast) */
  setRolesFromNetwork(nodeId: NodeId, roles: NodeRole[]): void {
    this.setRoles(nodeId, roles);
  }

  /** Get the full assignment for a node */
  getAssignment(nodeId: NodeId): RoleAssignment | undefined {
    return this.assignments.get(nodeId);
  }

  /** Remove a node's assignment */
  removeAssignment(nodeId: NodeId): void {
    this.assignments.delete(nodeId);
  }

  private setRoles(nodeId: NodeId, newRoles: NodeRole[]): void {
    const existing = this.assignments.get(nodeId);
    const oldRoles = existing ? [...existing.roles] : (['client'] as NodeRole[]);

    const changed = oldRoles.length !== newRoles.length || oldRoles.some((r, i) => r !== newRoles[i]);

    console.log(
      `[RoleManager] setRoles(${nodeId.slice(0, 8)}): ` +
        `old=[${oldRoles.join(',')}] new=[${newRoles.join(',')}] changed=${changed}`,
    );

    if (changed) {
      this.assignments.set(nodeId, {
        nodeId,
        roles: [...newRoles],
        assignedAt: Date.now(),
      });
      console.log(`[RoleManager] FIRING onRoleChanged for ${nodeId.slice(0, 8)}`);
      this.events.onRoleChanged(nodeId, oldRoles, newRoles);
    } else if (!existing) {
      // First assignment, no change event needed
      this.assignments.set(nodeId, {
        nodeId,
        roles: [...newRoles],
        assignedAt: Date.now(),
      });
    }
  }
}
