import type { NodeId } from '../identity/index.js';
import type { BackupStore } from './backup-store.js';

/** Threshold for triggering replication (30%) */
export const REPLICATION_THRESHOLD = 30;

/** Critical threshold for self-deletion (10%) */
export const DELETION_THRESHOLD = 10;

/** Viability check interval (30 seconds) */
const VIABILITY_CHECK_INTERVAL_MS = 30 * 1000;

/** Factors used to compute viability score */
export interface ViabilityFactors {
  /** Connection stability score (0-100) - higher if stable */
  connectionStability: number;
  /** Host bandwidth capacity (0-100) */
  bandwidthCapacity: number;
  /** Host contribution score (0-100) */
  contributionScore: number;
}

export interface MessageViabilityEvents {
  /** Emitted when a message needs replication to better hosts */
  onReplicationNeeded: (messageId: string, currentScore: number) => void;
  /** Emitted when a message should self-delete (host dying) */
  onSelfDeleteNeeded: (messageId: string, currentScore: number) => void;
}

/**
 * MessageViability - Computes and monitors message viability scores.
 *
 * Per ADR-009 "Virus Metaphor":
 * - Messages monitor their own viability continuously
 * - When score drops below 30%, replicate to better host
 * - When score drops below 10%, self-delete (before host dies)
 */
export class MessageViability {
  private events: MessageViabilityEvents;
  private backupStore: BackupStore;
  private hostFactors: ViabilityFactors;
  private recipientTimezones = new Map<NodeId, number>();
  private checkInterval: ReturnType<typeof setInterval> | null = null;

  constructor(events: MessageViabilityEvents, backupStore: BackupStore, hostFactors?: Partial<ViabilityFactors>) {
    this.events = events;
    this.backupStore = backupStore;
    this.hostFactors = {
      connectionStability: 50, // Default: moderate stability
      bandwidthCapacity: 50, // Default: moderate bandwidth
      contributionScore: 50, // Default: moderate contribution
      ...hostFactors,
    };
  }

  /** Start periodic viability checking */
  start(): void {
    if (this.checkInterval) return;
    this.checkInterval = setInterval(() => {
      this.checkAllMessages();
    }, VIABILITY_CHECK_INTERVAL_MS);
  }

  /** Stop periodic viability checking */
  stop(): void {
    if (this.checkInterval) {
      clearInterval(this.checkInterval);
      this.checkInterval = null;
    }
  }

  /** Update host factors (e.g., when connection stability changes) */
  updateHostFactors(factors: Partial<ViabilityFactors>): void {
    this.hostFactors = { ...this.hostFactors, ...factors };
  }

  /** Set known timezone for a recipient (offset in hours from UTC) */
  setRecipientTimezone(recipientId: NodeId, timezoneOffsetHours: number): void {
    this.recipientTimezones.set(recipientId, timezoneOffsetHours);
  }

  /** Get the current host timezone offset (from local system) */
  getHostTimezoneOffset(): number {
    return -new Date().getTimezoneOffset() / 60;
  }

  /**
   * Calculate viability score for a specific message.
   *
   * Score factors (weighted):
   * - Timezone alignment (25%): Similar timezone = likely online at same time
   * - Connection stability (30%): Stable host = message likely survives
   * - Bandwidth capacity (25%): High bandwidth = faster delivery
   * - Contribution score (20%): Active node = better network citizen
   */
  calculateScore(messageId: string): number {
    const msg = this.backupStore.getMessage(messageId);
    if (!msg) return 0;

    // Calculate timezone alignment
    const timezoneScore = this.calculateTimezoneAlignment(msg.recipientId);

    // Weighted score calculation
    const score =
      timezoneScore * 0.25 +
      this.hostFactors.connectionStability * 0.3 +
      this.hostFactors.bandwidthCapacity * 0.25 +
      this.hostFactors.contributionScore * 0.2;

    return Math.round(Math.max(0, Math.min(100, score)));
  }

  /**
   * Calculate timezone alignment score (0-100)
   *
   * Handles 24h wrap-around: +11 and -11 are only 2h apart, not 22h.
   * Max difference is 12h (opposite sides of the globe).
   */
  private calculateTimezoneAlignment(recipientId: NodeId): number {
    const recipientTz = this.recipientTimezones.get(recipientId);
    if (recipientTz === undefined) {
      // Unknown timezone - assume neutral (50%)
      return 50;
    }

    const hostTz = this.getHostTimezoneOffset();
    const rawDiff = Math.abs(hostTz - recipientTz);

    // Handle 24h wrap-around: if difference > 12, actual distance is 24 - diff
    // e.g., +11 and -11 → rawDiff = 22 → actualDiff = 24 - 22 = 2
    const actualDiff = Math.min(rawDiff, 24 - rawDiff);

    // Perfect alignment (same timezone) = 100
    // 12 hours difference = 0
    const score = Math.max(0, 100 - (actualDiff / 12) * 100);
    return Math.round(score);
  }

  /** Check a single message and emit events if thresholds crossed */
  checkMessage(messageId: string): number {
    const score = this.calculateScore(messageId);

    // Update the stored score
    this.backupStore.updateViabilityScore(messageId, score);

    // Check thresholds
    if (score <= DELETION_THRESHOLD) {
      console.log(`[MessageViability] Message ${messageId.slice(0, 8)} below deletion threshold (${score}%)`);
      this.events.onSelfDeleteNeeded(messageId, score);
    } else if (score <= REPLICATION_THRESHOLD) {
      console.log(`[MessageViability] Message ${messageId.slice(0, 8)} below replication threshold (${score}%)`);
      this.events.onReplicationNeeded(messageId, score);
    }

    return score;
  }

  /** Check all stored messages */
  checkAllMessages(): void {
    const messages = this.backupStore.getAllMessages();
    for (const msg of messages) {
      this.checkMessage(msg.envelope.id);
    }
  }

  /**
   * Simulate host degradation (for testing or when detecting issues).
   * Reduces connection stability which affects all message scores.
   */
  degradeHostStability(amount: number): void {
    const newStability = Math.max(0, this.hostFactors.connectionStability - amount);
    this.updateHostFactors({ connectionStability: newStability });

    console.log(`[MessageViability] Host stability degraded to ${newStability}%`);

    // Re-check all messages after degradation
    this.checkAllMessages();
  }

  /**
   * Simulate host improvement (e.g., after reconnection).
   */
  improveHostStability(amount: number): void {
    const newStability = Math.min(100, this.hostFactors.connectionStability + amount);
    this.updateHostFactors({ connectionStability: newStability });

    console.log(`[MessageViability] Host stability improved to ${newStability}%`);
  }

  /** Get current host factors (for debugging/monitoring) */
  getHostFactors(): ViabilityFactors {
    return { ...this.hostFactors };
  }
}
