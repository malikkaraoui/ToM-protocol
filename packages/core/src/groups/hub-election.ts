/**
 * Hub Election (Action 1: Hub Failover & Resilience Groups)
 *
 * Implements automatic hub election when the current hub fails.
 * Uses deterministic selection (lowest nodeId) to prevent split-brain.
 *
 * Election process:
 * 1. Hub failure detected (missed heartbeats)
 * 2. Each member independently calculates new hub from candidate relays
 * 3. All members converge on same selection (deterministic)
 * 4. New hub announces itself and takes over
 *
 * @see group-hub.ts for GroupHub implementation
 * @see group-manager.ts for client-side handling
 */

import type { NodeId } from '../identity/index.js';
import type { GroupId, GroupInfo } from './group-types.js';

/** Candidate relay for hub election */
export interface HubCandidate {
  nodeId: NodeId;
  /** Relay capability confirmed */
  isRelay: boolean;
  /** Last seen timestamp */
  lastSeen: number;
  /** Score for ranking (higher = better) */
  score?: number;
}

/** Election result */
export interface ElectionResult {
  /** Elected hub nodeId (null if no candidates) */
  newHubId: NodeId | null;
  /** Why this hub was selected */
  reason: 'deterministic' | 'backup' | 'none';
  /** All candidates considered */
  candidates: HubCandidate[];
}

/** Events emitted by HubElection */
export interface HubElectionEvents {
  /** We were elected as the new hub */
  onElectedAsHub: (groupId: GroupId, groupInfo: GroupInfo) => void;
  /** New hub was elected (not us) */
  onHubElected: (groupId: GroupId, newHubId: NodeId) => void;
  /** Election failed (no candidates) */
  onElectionFailed: (groupId: GroupId, reason: string) => void;
}

/** Options for HubElection */
export interface HubElectionOptions {
  /** Maximum age for a candidate to be considered (ms) */
  maxCandidateAgeMs?: number;
  /** Minimum candidates required for election */
  minCandidates?: number;
}

/** Default max age: 60 seconds */
const DEFAULT_MAX_CANDIDATE_AGE_MS = 60_000;

/**
 * HubElection
 *
 * Handles hub election when current hub fails.
 * Uses deterministic selection based on nodeId ordering.
 */
export class HubElection {
  private localNodeId: NodeId;
  private events: Partial<HubElectionEvents>;
  private maxCandidateAgeMs: number;
  private minCandidates: number;

  /** Active elections (groupId -> election in progress) */
  private activeElections = new Map<
    GroupId,
    {
      startedAt: number;
      candidates: HubCandidate[];
      failedHubId: NodeId;
    }
  >();

  constructor(localNodeId: NodeId, events: Partial<HubElectionEvents> = {}, options: HubElectionOptions = {}) {
    this.localNodeId = localNodeId;
    this.events = events;
    this.maxCandidateAgeMs = options.maxCandidateAgeMs ?? DEFAULT_MAX_CANDIDATE_AGE_MS;
    this.minCandidates = options.minCandidates ?? 1;
  }

  /**
   * Initiate election for a group after hub failure
   *
   * @param groupId - Group that lost its hub
   * @param failedHubId - NodeId of the failed hub
   * @param candidates - Available relay candidates
   * @param groupInfo - Current group info (for state recovery)
   * @returns Election result
   */
  initiateElection(
    groupId: GroupId,
    failedHubId: NodeId,
    candidates: HubCandidate[],
    groupInfo: GroupInfo,
  ): ElectionResult {
    const now = Date.now();

    // Filter candidates: must be relay, recently seen, not the failed hub
    const eligibleCandidates = candidates.filter(
      (c) =>
        c.isRelay &&
        c.nodeId !== failedHubId &&
        now - c.lastSeen < this.maxCandidateAgeMs &&
        // Also consider ourselves if we're a relay
        (c.nodeId !== this.localNodeId || c.isRelay),
    );

    // Check for backup hub first
    if (groupInfo.backupHubId && eligibleCandidates.some((c) => c.nodeId === groupInfo.backupHubId)) {
      const newHubId = groupInfo.backupHubId;
      this.handleElectionResult(groupId, newHubId, groupInfo);
      return {
        newHubId,
        reason: 'backup',
        candidates: eligibleCandidates,
      };
    }

    // Check minimum candidates
    if (eligibleCandidates.length < this.minCandidates) {
      this.events.onElectionFailed?.(groupId, 'insufficient-candidates');
      return {
        newHubId: null,
        reason: 'none',
        candidates: eligibleCandidates,
      };
    }

    // Deterministic selection: sort by nodeId and pick first
    // This ensures all nodes select the same hub independently
    eligibleCandidates.sort((a, b) => a.nodeId.localeCompare(b.nodeId));
    const newHubId = eligibleCandidates[0].nodeId;

    // Store election state
    this.activeElections.set(groupId, {
      startedAt: now,
      candidates: eligibleCandidates,
      failedHubId,
    });

    // Handle result
    this.handleElectionResult(groupId, newHubId, groupInfo);

    return {
      newHubId,
      reason: 'deterministic',
      candidates: eligibleCandidates,
    };
  }

  /**
   * Handle election result
   */
  private handleElectionResult(groupId: GroupId, newHubId: NodeId, groupInfo: GroupInfo): void {
    // Clean up election state
    this.activeElections.delete(groupId);

    if (newHubId === this.localNodeId) {
      // We are the new hub
      this.events.onElectedAsHub?.(groupId, groupInfo);
    } else {
      // Someone else is the new hub
      this.events.onHubElected?.(groupId, newHubId);
    }
  }

  /**
   * Check if we should become the hub based on current candidates
   *
   * @param candidates - Available relay candidates (including self)
   * @param excludeNodeId - Node to exclude (usually the failed hub)
   * @returns true if we should become hub
   */
  shouldBecomeHub(candidates: HubCandidate[], excludeNodeId?: NodeId): boolean {
    const now = Date.now();

    // Filter eligible candidates
    const eligible = candidates.filter(
      (c) => c.isRelay && c.nodeId !== excludeNodeId && now - c.lastSeen < this.maxCandidateAgeMs,
    );

    if (eligible.length === 0) {
      return false;
    }

    // Sort deterministically
    eligible.sort((a, b) => a.nodeId.localeCompare(b.nodeId));

    // We're the hub if we're first
    return eligible[0].nodeId === this.localNodeId;
  }

  /**
   * Get the expected new hub from candidates
   */
  selectHub(candidates: HubCandidate[], excludeNodeId?: NodeId): NodeId | null {
    const now = Date.now();

    // Filter eligible candidates
    const eligible = candidates.filter(
      (c) => c.isRelay && c.nodeId !== excludeNodeId && now - c.lastSeen < this.maxCandidateAgeMs,
    );

    if (eligible.length === 0) {
      return null;
    }

    // Sort deterministically and return first
    eligible.sort((a, b) => a.nodeId.localeCompare(b.nodeId));
    return eligible[0].nodeId;
  }

  /**
   * Check if an election is in progress for a group
   */
  isElectionInProgress(groupId: GroupId): boolean {
    return this.activeElections.has(groupId);
  }

  /**
   * Cancel an in-progress election (e.g., if hub came back)
   */
  cancelElection(groupId: GroupId): boolean {
    return this.activeElections.delete(groupId);
  }

  /**
   * Get active election info
   */
  getElectionInfo(groupId: GroupId): { startedAt: number; candidates: HubCandidate[]; failedHubId: NodeId } | null {
    return this.activeElections.get(groupId) ?? null;
  }

  /**
   * Clear all elections (cleanup)
   */
  clear(): void {
    this.activeElections.clear();
  }
}
