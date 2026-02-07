/**
 * UI State Manager Tests (Action 3: Reactive UI & Hooks)
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  type UIStateEvent,
  type UIStateEventType,
  UIStateManager,
  getUIStateManager,
  resetUIStateManager,
} from './ui-state';

describe('UIStateManager', () => {
  let manager: UIStateManager;

  beforeEach(() => {
    vi.useFakeTimers();
    manager = new UIStateManager({ debounceMs: 50 });
  });

  afterEach(() => {
    manager.clear();
    vi.useRealTimers();
  });

  describe('event subscription', () => {
    it('should subscribe to specific event types', () => {
      const listener = vi.fn();
      manager.on('groups:changed', listener);

      manager.emitImmediate('groups:changed');
      expect(listener).toHaveBeenCalledTimes(1);
      expect(listener).toHaveBeenCalledWith(
        expect.objectContaining({
          type: 'groups:changed',
          timestamp: expect.any(Number),
        }),
      );
    });

    it('should not call listener for different event types', () => {
      const listener = vi.fn();
      manager.on('groups:changed', listener);

      manager.emitImmediate('members:changed');
      expect(listener).not.toHaveBeenCalled();
    });

    it('should subscribe to all events with onAny', () => {
      const listener = vi.fn();
      manager.onAny(listener);

      manager.emitImmediate('groups:changed');
      manager.emitImmediate('members:changed');
      expect(listener).toHaveBeenCalledTimes(2);
    });

    it('should unsubscribe when calling returned function', () => {
      const listener = vi.fn();
      const unsubscribe = manager.on('groups:changed', listener);

      manager.emitImmediate('groups:changed');
      expect(listener).toHaveBeenCalledTimes(1);

      unsubscribe();
      manager.emitImmediate('groups:changed');
      expect(listener).toHaveBeenCalledTimes(1);
    });
  });

  describe('convenience hooks', () => {
    it('should provide onGroupsChanged hook', () => {
      const listener = vi.fn();
      manager.onGroupsChanged(listener);

      manager.emitImmediate('groups:changed');
      expect(listener).toHaveBeenCalledTimes(1);
    });

    it('should provide onMembersChanged hook', () => {
      const listener = vi.fn();
      manager.onMembersChanged(listener);

      manager.emitImmediate('members:changed');
      expect(listener).toHaveBeenCalledTimes(1);
    });

    it('should provide onInvitesChanged hook', () => {
      const listener = vi.fn();
      manager.onInvitesChanged(listener);

      manager.emitImmediate('invites:changed');
      expect(listener).toHaveBeenCalledTimes(1);
    });

    it('should provide onMessagesChanged hook', () => {
      const listener = vi.fn();
      manager.onMessagesChanged(listener);

      manager.emitImmediate('messages:changed');
      expect(listener).toHaveBeenCalledTimes(1);
    });

    it('should provide onParticipantsChanged hook', () => {
      const listener = vi.fn();
      manager.onParticipantsChanged(listener);

      manager.emitImmediate('participants:changed');
      expect(listener).toHaveBeenCalledTimes(1);
    });

    it('should provide onSelectionChanged hook', () => {
      const listener = vi.fn();
      manager.onSelectionChanged(listener);

      manager.emitImmediate('selection:changed');
      expect(listener).toHaveBeenCalledTimes(1);
    });
  });

  describe('debouncing', () => {
    it('should debounce rapid emissions', () => {
      const listener = vi.fn();
      manager.on('groups:changed', listener);

      // Emit multiple times rapidly
      manager.emit('groups:changed');
      manager.emit('groups:changed');
      manager.emit('groups:changed');

      // Listener not called yet (debounced)
      expect(listener).not.toHaveBeenCalled();

      // Advance time past debounce
      vi.advanceTimersByTime(60);

      // Should only be called once
      expect(listener).toHaveBeenCalledTimes(1);
    });

    it('should emit immediately when requested', () => {
      const listener = vi.fn();
      manager.on('groups:changed', listener);

      manager.emitImmediate('groups:changed');
      expect(listener).toHaveBeenCalledTimes(1);
    });

    it('should cancel pending debounced update when emitting immediately', () => {
      const listener = vi.fn();
      manager.on('groups:changed', listener);

      manager.emit('groups:changed');
      manager.emitImmediate('groups:changed');

      expect(listener).toHaveBeenCalledTimes(1);

      // Advance time - should not call again
      vi.advanceTimersByTime(100);
      expect(listener).toHaveBeenCalledTimes(1);
    });
  });

  describe('batch emissions', () => {
    it('should emit multiple event types in batch', () => {
      const groupsListener = vi.fn();
      const membersListener = vi.fn();
      manager.on('groups:changed', groupsListener);
      manager.on('members:changed', membersListener);

      manager.emitBatch(['groups:changed', 'members:changed']);
      vi.advanceTimersByTime(60);

      expect(groupsListener).toHaveBeenCalledTimes(1);
      expect(membersListener).toHaveBeenCalledTimes(1);
    });

    it('should dedupe duplicate event types in batch', () => {
      const listener = vi.fn();
      manager.on('groups:changed', listener);

      manager.emitBatch(['groups:changed', 'groups:changed', 'groups:changed']);
      vi.advanceTimersByTime(60);

      expect(listener).toHaveBeenCalledTimes(1);
    });
  });

  describe('forceRefreshAll', () => {
    it('should emit all event types immediately', () => {
      const listeners: Record<UIStateEventType, ReturnType<typeof vi.fn>> = {
        'groups:changed': vi.fn(),
        'members:changed': vi.fn(),
        'invites:changed': vi.fn(),
        'messages:changed': vi.fn(),
        'participants:changed': vi.fn(),
        'connection:changed': vi.fn(),
        'selection:changed': vi.fn(),
      };

      for (const [eventType, listener] of Object.entries(listeners)) {
        manager.on(eventType as UIStateEventType, listener);
      }

      manager.forceRefreshAll();

      // All listeners except connection:changed should be called
      expect(listeners['groups:changed']).toHaveBeenCalledTimes(1);
      expect(listeners['members:changed']).toHaveBeenCalledTimes(1);
      expect(listeners['invites:changed']).toHaveBeenCalledTimes(1);
      expect(listeners['messages:changed']).toHaveBeenCalledTimes(1);
      expect(listeners['participants:changed']).toHaveBeenCalledTimes(1);
      expect(listeners['selection:changed']).toHaveBeenCalledTimes(1);
    });
  });

  describe('event data', () => {
    it('should pass data to listeners', () => {
      const listener = vi.fn();
      manager.on('groups:changed', listener);

      const data = { groupId: 'test-123', action: 'created' };
      manager.emitImmediate('groups:changed', data);

      expect(listener).toHaveBeenCalledWith(
        expect.objectContaining({
          type: 'groups:changed',
          data,
        }),
      );
    });
  });

  describe('error handling', () => {
    it('should continue calling other listeners if one throws', () => {
      const errorListener = vi.fn(() => {
        throw new Error('Test error');
      });
      const normalListener = vi.fn();

      manager.on('groups:changed', errorListener);
      manager.on('groups:changed', normalListener);

      // Should not throw
      expect(() => manager.emitImmediate('groups:changed')).not.toThrow();
      expect(normalListener).toHaveBeenCalledTimes(1);
    });
  });

  describe('listener count', () => {
    it('should return correct listener count for specific event', () => {
      manager.on('groups:changed', vi.fn());
      manager.on('groups:changed', vi.fn());
      manager.on('members:changed', vi.fn());

      expect(manager.getListenerCount('groups:changed')).toBe(2);
      expect(manager.getListenerCount('members:changed')).toBe(1);
    });

    it('should include global listeners in count', () => {
      manager.on('groups:changed', vi.fn());
      manager.onAny(vi.fn());

      expect(manager.getListenerCount('groups:changed')).toBe(2);
    });

    it('should return total count when no event type specified', () => {
      manager.on('groups:changed', vi.fn());
      manager.on('members:changed', vi.fn());
      manager.onAny(vi.fn());

      expect(manager.getListenerCount()).toBe(3);
    });
  });

  describe('clear', () => {
    it('should remove all listeners', () => {
      const listener = vi.fn();
      manager.on('groups:changed', listener);
      manager.onAny(listener);

      manager.clear();
      manager.emitImmediate('groups:changed');

      expect(listener).not.toHaveBeenCalled();
    });

    it('should cancel pending updates', () => {
      const listener = vi.fn();
      manager.on('groups:changed', listener);

      manager.emit('groups:changed');
      manager.clear();
      vi.advanceTimersByTime(100);

      expect(listener).not.toHaveBeenCalled();
    });
  });
});

describe('singleton instance', () => {
  afterEach(() => {
    resetUIStateManager();
  });

  it('should return the same instance', () => {
    const instance1 = getUIStateManager();
    const instance2 = getUIStateManager();
    expect(instance1).toBe(instance2);
  });

  it('should create new instance after reset', () => {
    const instance1 = getUIStateManager();
    resetUIStateManager();
    const instance2 = getUIStateManager();
    expect(instance1).not.toBe(instance2);
  });
});
