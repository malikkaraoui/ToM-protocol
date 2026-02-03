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

/** Minimum time a node must be online before being eligible for relay role (ms) */
const RELAY_ELIGIBILITY_THRESHOLD_MS = 5000;

/** Re-evaluation interval (ms) */
const REEVALUATION_INTERVAL_MS = 30000;

export class RoleManager {
  private assignments = new Map<NodeId, RoleAssignment>();
  private events: RoleManagerEvents;
  private reevaluationTimer: ReturnType<typeof setInterval> | null = null;
  private topology: NetworkTopology | null = null;

  constructor(events: RoleManagerEvents) {
    this.events = events;
  }

  /** Bind to a topology instance for periodic re-evaluation */
  bindTopology(topology: NetworkTopology): void {
    this.topology = topology;
  }

  /** Start periodic role re-evaluation */
  start(): void {
    if (this.reevaluationTimer) return;
    this.reevaluationTimer = setInterval(() => {
      if (this.topology) {
        this.reassignRoles(this.topology);
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

  /** Evaluate and assign roles for a single node */
  evaluateNode(nodeId: NodeId, topology: NetworkTopology): NodeRole[] {
    const peer = topology.getPeer(nodeId);
    if (!peer) return ['client'];

    const onlinePeers = topology.getOnlinePeers();
    const totalNodes = onlinePeers.length;
    const currentRelayCount = this.countRelays();

    const isEligibleForRelay = this.isRelayEligible(nodeId, topology);
    const targetRelayCount = Math.ceil(totalNodes / 3);

    let newRoles: NodeRole[] = ['client'];

    if (isEligibleForRelay && currentRelayCount < targetRelayCount) {
      // Network needs more relays — promote this node
      newRoles = ['client', 'relay'];
    } else {
      // Check if this node is already a relay
      const existing = this.assignments.get(nodeId);
      if (existing?.roles.includes('relay')) {
        const maxRelayCount = Math.ceil(totalNodes / 2);
        if (currentRelayCount > maxRelayCount) {
          // Too many relays — demote
          newRoles = ['client'];
        } else {
          // Keep relay role
          newRoles = ['client', 'relay'];
        }
      }
    }

    this.setRoles(nodeId, newRoles);
    return newRoles;
  }

  /** Re-evaluate roles for all peers in the topology */
  reassignRoles(topology: NetworkTopology): Map<string, NodeRole[]> {
    const result = new Map<string, NodeRole[]>();
    const onlinePeers = topology.getOnlinePeers();

    for (const peer of onlinePeers) {
      const roles = this.evaluateNode(peer.nodeId, topology);
      result.set(peer.nodeId, roles);
    }

    return result;
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

    if (changed) {
      this.assignments.set(nodeId, {
        nodeId,
        roles: [...newRoles],
        assignedAt: Date.now(),
      });
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

  private isRelayEligible(nodeId: NodeId, topology: NetworkTopology): boolean {
    const peer = topology.getPeer(nodeId);
    if (!peer) return false;

    const status = topology.getPeerStatus(nodeId);
    if (status !== 'online') return false;

    // Must have been online for at least the eligibility threshold
    const elapsed = Date.now() - peer.lastSeen;
    // lastSeen is updated regularly — if elapsed is small, node is active
    // We use assignedAt to check how long we've known the node
    const assignment = this.assignments.get(nodeId);
    if (assignment) {
      const timeSinceAssigned = Date.now() - assignment.assignedAt;
      return timeSinceAssigned >= RELAY_ELIGIBILITY_THRESHOLD_MS || assignment.roles.includes('relay');
    }

    // New node — not yet eligible
    return false;
  }

  private countRelays(): number {
    let count = 0;
    for (const assignment of this.assignments.values()) {
      if (assignment.roles.includes('relay')) count++;
    }
    return count;
  }
}
