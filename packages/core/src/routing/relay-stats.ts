export interface RelayStatsData {
  messagesRelayed: number;
  ownMessagesSent: number;
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

export class RelayStats {
  private messagesRelayed = 0;
  private ownMessagesSent = 0;
  private lastRelayTimestamp = 0;
  private lastOwnMessageTimestamp = 0;
  private capacityThreshold: number;
  private events: Partial<RelayStatsEvents>;

  constructor(options: RelayStatsOptions = {}) {
    this.capacityThreshold = options.capacityThreshold ?? 10;
    this.events = options.events ?? {};
  }

  recordRelay(): void {
    this.messagesRelayed++;
    this.lastRelayTimestamp = Date.now();
    this.checkCapacity();
  }

  recordOwnMessage(): void {
    this.ownMessagesSent++;
    this.lastOwnMessageTimestamp = Date.now();
  }

  getStats(): RelayStatsData {
    const ratio = this.ownMessagesSent > 0 ? this.messagesRelayed / this.ownMessagesSent : this.messagesRelayed;

    return {
      messagesRelayed: this.messagesRelayed,
      ownMessagesSent: this.ownMessagesSent,
      relayToOwnRatio: Math.round(ratio * 100) / 100,
      lastRelayTimestamp: this.lastRelayTimestamp,
      lastOwnMessageTimestamp: this.lastOwnMessageTimestamp,
    };
  }

  private checkCapacity(): void {
    const stats = this.getStats();

    // Only warn if we've sent at least 1 own message and ratio exceeds threshold
    if (this.ownMessagesSent > 0 && stats.relayToOwnRatio > this.capacityThreshold) {
      this.events.onCapacityWarning?.(
        stats,
        `relay:own ratio ${stats.relayToOwnRatio} exceeds threshold ${this.capacityThreshold}`,
      );
    }

    // Also warn if we've relayed many messages but never sent our own (pure relay)
    if (this.ownMessagesSent === 0 && this.messagesRelayed > 20) {
      this.events.onCapacityWarning?.(stats, 'node is acting as pure relay without own messaging');
    }
  }

  reset(): void {
    this.messagesRelayed = 0;
    this.ownMessagesSent = 0;
    this.lastRelayTimestamp = 0;
    this.lastOwnMessageTimestamp = 0;
  }
}
