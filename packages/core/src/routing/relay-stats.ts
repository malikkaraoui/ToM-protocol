export interface RelayStatsData {
  /** Messages this node forwarded for others (acting as relay) */
  messagesRelayed: number;
  /** Own messages sent by this node */
  ownMessagesSent: number;
  /** Relay ACKs received (own messages confirmed relayed) */
  relayAcksReceived: number;
  /** Bytes relayed for others */
  bytesRelayed: number;
  /** Bytes sent in own messages */
  bytesSent: number;
  relayToOwnRatio: number;
  lastRelayTimestamp: number;
  lastOwnMessageTimestamp: number;
}

export interface RelayStatsEvents {
  onCapacityWarning: (stats: RelayStatsData, reason: string) => void;
}

export interface RelayStatsOptions {
  capacityThreshold?: number; // ratio of relay:own that triggers warning (default: 10)
  events?: Partial<RelayStatsEvents>;
}

/** Cooldown between capacity warnings (10 seconds) @security prevents event storm */
const WARNING_COOLDOWN_MS = 10 * 1000;

/** Maximum counter value to prevent overflow (1 billion) @security prevents DoS */
const MAX_COUNTER_VALUE = 1_000_000_000;

export class RelayStats {
  private messagesRelayed = 0;
  private ownMessagesSent = 0;
  private relayAcksReceived = 0;
  private bytesRelayed = 0;
  private bytesSent = 0;
  private lastRelayTimestamp = 0;
  private lastOwnMessageTimestamp = 0;
  private capacityThreshold: number;
  private events: Partial<RelayStatsEvents>;
  /** Last time a warning was emitted @security prevents event storm */
  private lastWarningAt = 0;

  constructor(options: RelayStatsOptions = {}) {
    this.capacityThreshold = options.capacityThreshold ?? 10;
    this.events = options.events ?? {};
  }

  recordRelay(byteSize?: number): void {
    // Bound counter to prevent overflow
    if (this.messagesRelayed < MAX_COUNTER_VALUE) {
      this.messagesRelayed++;
    }
    if (byteSize && this.bytesRelayed < MAX_COUNTER_VALUE) {
      this.bytesRelayed += byteSize;
    }
    this.lastRelayTimestamp = Date.now();
    this.checkCapacity();
  }

  recordOwnMessage(byteSize?: number): void {
    // Bound counter to prevent overflow
    if (this.ownMessagesSent < MAX_COUNTER_VALUE) {
      this.ownMessagesSent++;
    }
    if (byteSize && this.bytesSent < MAX_COUNTER_VALUE) {
      this.bytesSent += byteSize;
    }
    this.lastOwnMessageTimestamp = Date.now();
  }

  recordRelayAck(): void {
    // Bound counter to prevent overflow
    if (this.relayAcksReceived < MAX_COUNTER_VALUE) {
      this.relayAcksReceived++;
    }
  }

  getStats(): RelayStatsData {
    const ratio = this.ownMessagesSent > 0 ? this.messagesRelayed / this.ownMessagesSent : this.messagesRelayed;

    return {
      messagesRelayed: this.messagesRelayed,
      ownMessagesSent: this.ownMessagesSent,
      relayAcksReceived: this.relayAcksReceived,
      bytesRelayed: this.bytesRelayed,
      bytesSent: this.bytesSent,
      relayToOwnRatio: Math.round(ratio * 100) / 100,
      lastRelayTimestamp: this.lastRelayTimestamp,
      lastOwnMessageTimestamp: this.lastOwnMessageTimestamp,
    };
  }

  /**
   * Check capacity and emit warnings if thresholds exceeded.
   * @security Cooldown prevents event storm under high load
   */
  private checkCapacity(): void {
    const now = Date.now();

    // Cooldown: don't spam warnings
    if (now - this.lastWarningAt < WARNING_COOLDOWN_MS) {
      return;
    }

    const stats = this.getStats();
    let shouldWarn = false;
    let reason = '';

    // Only warn if we've sent at least 1 own message and ratio exceeds threshold
    if (this.ownMessagesSent > 0 && stats.relayToOwnRatio > this.capacityThreshold) {
      shouldWarn = true;
      reason = `relay:own ratio ${stats.relayToOwnRatio} exceeds threshold ${this.capacityThreshold}`;
    }

    // Also warn if we've relayed many messages but never sent our own (pure relay)
    if (this.ownMessagesSent === 0 && this.messagesRelayed > 20) {
      shouldWarn = true;
      reason = 'node is acting as pure relay without own messaging';
    }

    if (shouldWarn) {
      this.lastWarningAt = now;
      this.events.onCapacityWarning?.(stats, reason);
    }
  }

  reset(): void {
    this.messagesRelayed = 0;
    this.ownMessagesSent = 0;
    this.relayAcksReceived = 0;
    this.bytesRelayed = 0;
    this.bytesSent = 0;
    this.lastRelayTimestamp = 0;
    this.lastOwnMessageTimestamp = 0;
    this.lastWarningAt = 0;
  }
}
