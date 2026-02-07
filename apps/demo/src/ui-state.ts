/**
 * UI State Manager with Observer Pattern (Action 3: Reactive UI & Hooks)
 *
 * Centralizes UI state management and provides reactive hooks for state changes.
 * All state changes trigger automatic UI refresh within < 500ms.
 */

export type UIStateEventType =
  | 'groups:changed'
  | 'members:changed'
  | 'invites:changed'
  | 'messages:changed'
  | 'participants:changed'
  | 'connection:changed'
  | 'selection:changed';

export interface UIStateEvent {
  type: UIStateEventType;
  timestamp: number;
  data?: unknown;
}

export type UIStateListener = (event: UIStateEvent) => void;

/**
 * Observable UI State Manager
 *
 * Uses the Observer pattern to notify subscribers when state changes.
 * Debounces rapid updates to prevent UI thrashing.
 */
export class UIStateManager {
  private listeners = new Map<UIStateEventType, Set<UIStateListener>>();
  private globalListeners = new Set<UIStateListener>();
  private pendingUpdates = new Map<UIStateEventType, ReturnType<typeof setTimeout>>();
  private debounceMs: number;

  constructor(options: { debounceMs?: number } = {}) {
    this.debounceMs = options.debounceMs ?? 50; // Default 50ms debounce
  }

  /**
   * Subscribe to a specific event type
   */
  on(eventType: UIStateEventType, listener: UIStateListener): () => void {
    if (!this.listeners.has(eventType)) {
      this.listeners.set(eventType, new Set());
    }
    this.listeners.get(eventType)!.add(listener);

    // Return unsubscribe function
    return () => {
      this.listeners.get(eventType)?.delete(listener);
    };
  }

  /**
   * Subscribe to all events
   */
  onAny(listener: UIStateListener): () => void {
    this.globalListeners.add(listener);
    return () => {
      this.globalListeners.delete(listener);
    };
  }

  /**
   * Convenience hook: Subscribe to group changes
   */
  onGroupsChanged(listener: () => void): () => void {
    return this.on('groups:changed', listener);
  }

  /**
   * Convenience hook: Subscribe to member changes
   */
  onMembersChanged(listener: () => void): () => void {
    return this.on('members:changed', listener);
  }

  /**
   * Convenience hook: Subscribe to invite changes
   */
  onInvitesChanged(listener: () => void): () => void {
    return this.on('invites:changed', listener);
  }

  /**
   * Convenience hook: Subscribe to message changes
   */
  onMessagesChanged(listener: () => void): () => void {
    return this.on('messages:changed', listener);
  }

  /**
   * Convenience hook: Subscribe to participant changes
   */
  onParticipantsChanged(listener: () => void): () => void {
    return this.on('participants:changed', listener);
  }

  /**
   * Convenience hook: Subscribe to selection changes
   */
  onSelectionChanged(listener: () => void): () => void {
    return this.on('selection:changed', listener);
  }

  /**
   * Emit an event with debouncing to prevent UI thrashing
   */
  emit(eventType: UIStateEventType, data?: unknown): void {
    // Cancel pending update for this event type
    const pending = this.pendingUpdates.get(eventType);
    if (pending) {
      clearTimeout(pending);
    }

    // Schedule debounced update
    const timeout = setTimeout(() => {
      this.pendingUpdates.delete(eventType);
      this.notifyListeners(eventType, data);
    }, this.debounceMs);

    this.pendingUpdates.set(eventType, timeout);
  }

  /**
   * Emit an event immediately without debouncing
   */
  emitImmediate(eventType: UIStateEventType, data?: unknown): void {
    // Cancel any pending debounced update
    const pending = this.pendingUpdates.get(eventType);
    if (pending) {
      clearTimeout(pending);
      this.pendingUpdates.delete(eventType);
    }

    this.notifyListeners(eventType, data);
  }

  /**
   * Emit multiple events at once (batch update)
   */
  emitBatch(events: UIStateEventType[]): void {
    // Dedupe events
    const uniqueEvents = [...new Set(events)];
    for (const eventType of uniqueEvents) {
      this.emit(eventType);
    }
  }

  /**
   * Force refresh all UI (emit all events immediately)
   */
  forceRefreshAll(): void {
    const allEvents: UIStateEventType[] = [
      'groups:changed',
      'members:changed',
      'invites:changed',
      'messages:changed',
      'participants:changed',
      'selection:changed',
    ];

    for (const eventType of allEvents) {
      this.emitImmediate(eventType);
    }
  }

  private notifyListeners(eventType: UIStateEventType, data?: unknown): void {
    const event: UIStateEvent = {
      type: eventType,
      timestamp: Date.now(),
      data,
    };

    // Notify type-specific listeners
    const typeListeners = this.listeners.get(eventType);
    if (typeListeners) {
      for (const listener of typeListeners) {
        try {
          listener(event);
        } catch (error) {
          console.error(`[UIStateManager] Error in listener for ${eventType}:`, error);
        }
      }
    }

    // Notify global listeners
    for (const listener of this.globalListeners) {
      try {
        listener(event);
      } catch (error) {
        console.error('[UIStateManager] Error in global listener:', error);
      }
    }
  }

  /**
   * Get the number of listeners for a specific event type
   */
  getListenerCount(eventType?: UIStateEventType): number {
    if (eventType) {
      return (this.listeners.get(eventType)?.size ?? 0) + this.globalListeners.size;
    }
    let total = this.globalListeners.size;
    for (const listeners of this.listeners.values()) {
      total += listeners.size;
    }
    return total;
  }

  /**
   * Remove all listeners
   */
  clear(): void {
    this.listeners.clear();
    this.globalListeners.clear();
    for (const timeout of this.pendingUpdates.values()) {
      clearTimeout(timeout);
    }
    this.pendingUpdates.clear();
  }
}

// Singleton instance for the app
let instance: UIStateManager | null = null;

export function getUIStateManager(): UIStateManager {
  if (!instance) {
    instance = new UIStateManager();
  }
  return instance;
}

export function resetUIStateManager(): void {
  instance?.clear();
  instance = null;
}
