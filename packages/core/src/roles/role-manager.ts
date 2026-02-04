import type { NetworkTopology, NodeRole } from '../discovery/network-topology.js';
import type { NodeId } from '../identity/index.js';

export interface RoleAssignment {
  nodeId: NodeId;
  roles: NodeRole[];
  assignedAt: number;
}

/** Metrics used for backup role scoring */
export interface NodeMetrics {
  /** How long the node has been online (ms) */
  timeOnlineMs: number;
  /** Bandwidth capacity score (0-100) */
  bandwidthScore: number;
  /** Contribution score based on relay/backup activity (0-100) */
  contributionScore: number;
}

export interface RoleManagerEvents {
  onRoleChanged: (nodeId: string, oldRoles: NodeRole[], newRoles: NodeRole[]) => void;
}

/** Re-evaluation interval (ms) */
const REEVALUATION_INTERVAL_MS = 30000;

/** Minimum time online before eligible for backup role (5 minutes) */
const MIN_TIME_ONLINE_FOR_BACKUP_MS = 5 * 60 * 1000;

/** Minimum backup score to be considered for backup role */
const MIN_BACKUP_SCORE = 30;

export class RoleManager {
  private assignments = new Map<NodeId, RoleAssignment>();
  private events: RoleManagerEvents;
  private reevaluationTimer: ReturnType<typeof setInterval> | null = null;
  private topology: NetworkTopology | null = null;
  private selfNodeId: NodeId | null = null;
  private nodeMetrics = new Map<NodeId, NodeMetrics>();
  private nodeStartTimes = new Map<NodeId, number>();

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

  // ============ Backup Role Management ============

  /** Record when a node comes online (for time-online tracking) */
  recordNodeOnline(nodeId: NodeId): void {
    if (!this.nodeStartTimes.has(nodeId)) {
      this.nodeStartTimes.set(nodeId, Date.now());
    }
  }

  /** Update metrics for a node */
  updateNodeMetrics(nodeId: NodeId, metrics: Partial<NodeMetrics>): void {
    const existing = this.nodeMetrics.get(nodeId) || {
      timeOnlineMs: 0,
      bandwidthScore: 50,
      contributionScore: 0,
    };
    this.nodeMetrics.set(nodeId, { ...existing, ...metrics });
  }

  /** Get metrics for a node */
  getNodeMetrics(nodeId: NodeId): NodeMetrics {
    const startTime = this.nodeStartTimes.get(nodeId);
    const timeOnlineMs = startTime ? Date.now() - startTime : 0;

    const stored = this.nodeMetrics.get(nodeId);
    return {
      timeOnlineMs,
      bandwidthScore: stored?.bandwidthScore ?? 50,
      contributionScore: stored?.contributionScore ?? 0,
    };
  }

  /**
   * Calculate backup score for a node.
   * Higher score = better backup candidate.
   * Score factors:
   * - Time online (30%): longer online = more reliable
   * - Bandwidth (40%): higher bandwidth = faster message delivery
   * - Contribution (30%): past relay/backup activity
   */
  calculateBackupScore(nodeId: NodeId): number {
    const metrics = this.getNodeMetrics(nodeId);

    // Time online score (0-100): cap at 1 hour for max score
    const maxTimeMs = 60 * 60 * 1000; // 1 hour
    const timeScore = Math.min(100, (metrics.timeOnlineMs / maxTimeMs) * 100);

    // Weighted score calculation
    const score = timeScore * 0.3 + metrics.bandwidthScore * 0.4 + metrics.contributionScore * 0.3;

    return Math.round(score);
  }

  /**
   * Check if a node is eligible for backup role.
   * Requirements:
   * - Minimum time online (5 minutes)
   * - Minimum backup score (30)
   */
  isEligibleForBackup(nodeId: NodeId): boolean {
    const metrics = this.getNodeMetrics(nodeId);

    if (metrics.timeOnlineMs < MIN_TIME_ONLINE_FOR_BACKUP_MS) {
      return false;
    }

    const score = this.calculateBackupScore(nodeId);
    return score >= MIN_BACKUP_SCORE;
  }

  /** Check if a node currently has the backup role */
  isBackupNode(nodeId: NodeId): boolean {
    const roles = this.getCurrentRoles(nodeId);
    return roles.includes('backup');
  }

  /**
   * Evaluate and assign backup role using deterministic consensus.
   * Similar to relay: lowest eligible NodeIds become backups.
   * Target: ceil(totalNodes / 2) backup nodes for redundancy.
   */
  evaluateBackupRole(nodeId: NodeId, topology: NetworkTopology): boolean {
    const allPeers = topology.getReachablePeers();
    const allNodeIds = allPeers.map((p) => p.nodeId);
    if (this.selfNodeId && !allNodeIds.includes(this.selfNodeId)) {
      allNodeIds.push(this.selfNodeId);
    }

    // Filter to only eligible nodes
    const eligibleNodeIds = allNodeIds.filter((id) => this.isEligibleForBackup(id));

    if (eligibleNodeIds.length === 0) {
      return false;
    }

    // Sort by backup score (descending), then by nodeId (for tie-breaking)
    eligibleNodeIds.sort((a, b) => {
      const scoreA = this.calculateBackupScore(a);
      const scoreB = this.calculateBackupScore(b);
      if (scoreA !== scoreB) {
        return scoreB - scoreA; // Higher score first
      }
      return a.localeCompare(b); // Lexicographic for determinism
    });

    // Target: up to half of eligible nodes can be backups (cascading redundancy)
    const totalNodes = allNodeIds.length;
    const targetBackupCount = Math.max(1, Math.ceil(totalNodes / 2));

    // Top N eligible nodes become backups
    const backupNodeIds = eligibleNodeIds.slice(0, targetBackupCount);
    const shouldBeBackup = backupNodeIds.includes(nodeId);

    // Get current roles and update
    const currentRoles = this.getCurrentRoles(nodeId);
    const hasBackup = currentRoles.includes('backup');

    if (shouldBeBackup && !hasBackup) {
      const newRoles: NodeRole[] = [...currentRoles, 'backup'];
      this.setRoles(nodeId, newRoles);
      console.log(
        `[RoleManager] Assigned backup role to ${nodeId.slice(0, 8)} ` + `(score=${this.calculateBackupScore(nodeId)})`,
      );
      return true;
    }

    if (!shouldBeBackup && hasBackup) {
      const newRoles = currentRoles.filter((r) => r !== 'backup') as NodeRole[];
      this.setRoles(nodeId, newRoles);
      console.log(`[RoleManager] Removed backup role from ${nodeId.slice(0, 8)}`);
      return false;
    }

    return shouldBeBackup;
  }

  /** Re-evaluate backup roles for all nodes */
  reassignBackupRoles(topology: NetworkTopology): Map<string, boolean> {
    const result = new Map<string, boolean>();
    const allPeers = topology.getReachablePeers();

    // Evaluate self first
    if (this.selfNodeId) {
      const isBackup = this.evaluateBackupRole(this.selfNodeId, topology);
      result.set(this.selfNodeId, isBackup);
    }

    for (const peer of allPeers) {
      const isBackup = this.evaluateBackupRole(peer.nodeId, topology);
      result.set(peer.nodeId, isBackup);
    }

    return result;
  }

  /** Get all nodes with backup role */
  getBackupNodes(): NodeId[] {
    const backups: NodeId[] = [];
    for (const [nodeId, assignment] of this.assignments) {
      if (assignment.roles.includes('backup')) {
        backups.push(nodeId);
      }
    }
    return backups;
  }

  /** Increment contribution score for a node (called when node relays/backs up messages) */
  incrementContributionScore(nodeId: NodeId, amount = 1): void {
    const metrics = this.getNodeMetrics(nodeId);
    const newScore = Math.min(100, metrics.contributionScore + amount);
    this.updateNodeMetrics(nodeId, { contributionScore: newScore });
  }
}
