import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { OfflineDetector, type OfflineDetectorEvents } from './offline-detector.js';

describe('OfflineDetector', () => {
  let detector: OfflineDetector;
  let events: OfflineDetectorEvents;

  beforeEach(() => {
    vi.useFakeTimers();
    events = {
      onPeerOffline: vi.fn(),
      onPeerOnline: vi.fn(),
    };
    detector = new OfflineDetector(events, 100); // 100ms debounce for faster tests
  });

  afterEach(() => {
    detector.destroy();
    vi.useRealTimers();
  });

  describe('peer activity tracking', () => {
    it('should track peer activity and update lastSeen', () => {
      const nodeId = 'peer-1';
      detector.recordPeerActivity(nodeId);

      expect(detector.getLastSeen(nodeId)).toBeDefined();
      expect(detector.isOffline(nodeId)).toBe(false);
    });

    it('should not emit offline event on activity', () => {
      detector.recordPeerActivity('peer-1');
      expect(events.onPeerOffline).not.toHaveBeenCalled();
    });
  });

  describe('offline detection', () => {
    it('should mark peer as offline on handlePeerDeparted', () => {
      const nodeId = 'peer-1';
      detector.recordPeerActivity(nodeId);
      detector.handlePeerDeparted(nodeId);

      expect(detector.isOffline(nodeId)).toBe(true);
      expect(events.onPeerOffline).toHaveBeenCalledWith(nodeId, expect.any(Number));
    });

    it('should not double-emit offline event', () => {
      const nodeId = 'peer-1';
      detector.handlePeerDeparted(nodeId);
      detector.handlePeerDeparted(nodeId);

      expect(events.onPeerOffline).toHaveBeenCalledTimes(1);
    });

    it('should store offline peer info', () => {
      const nodeId = 'peer-1';
      const activityTime = Date.now();
      detector.recordPeerActivity(nodeId);

      vi.advanceTimersByTime(1000);
      detector.handlePeerDeparted(nodeId);

      const info = detector.getOfflinePeerInfo(nodeId);
      expect(info).toBeDefined();
      expect(info?.nodeId).toBe(nodeId);
      expect(info?.lastSeen).toBeGreaterThanOrEqual(activityTime);
      expect(info?.detectedAt).toBeGreaterThan(info?.lastSeen ?? 0);
    });

    it('should return all offline peers', () => {
      detector.handlePeerDeparted('peer-1');
      detector.handlePeerDeparted('peer-2');

      const offlinePeers = detector.getOfflinePeers();
      expect(offlinePeers).toHaveLength(2);
      expect(offlinePeers.map((p) => p.nodeId)).toContain('peer-1');
      expect(offlinePeers.map((p) => p.nodeId)).toContain('peer-2');
    });
  });

  describe('reconnection detection', () => {
    it('should emit onPeerOnline when offline peer has activity (after debounce)', () => {
      const nodeId = 'peer-1';
      detector.handlePeerDeparted(nodeId);
      expect(detector.isOffline(nodeId)).toBe(true);

      detector.recordPeerActivity(nodeId);

      // Before debounce timeout - should still be offline
      expect(detector.isOffline(nodeId)).toBe(true);
      expect(events.onPeerOnline).not.toHaveBeenCalled();

      // After debounce timeout
      vi.advanceTimersByTime(150);

      expect(detector.isOffline(nodeId)).toBe(false);
      expect(events.onPeerOnline).toHaveBeenCalledWith(nodeId);
    });

    it('should emit onPeerOnline when markPeerOnline is called for offline peer', () => {
      const nodeId = 'peer-1';
      detector.handlePeerDeparted(nodeId);

      detector.markPeerOnline(nodeId);
      vi.advanceTimersByTime(150);

      expect(detector.isOffline(nodeId)).toBe(false);
      expect(events.onPeerOnline).toHaveBeenCalledWith(nodeId);
    });

    it('should debounce rapid reconnect/disconnect cycles', () => {
      const nodeId = 'peer-1';
      detector.handlePeerDeparted(nodeId);

      // Rapid reconnect
      detector.recordPeerActivity(nodeId);
      vi.advanceTimersByTime(50);

      // Disconnect again before debounce completes
      detector.handlePeerDeparted(nodeId);

      // Advance past original debounce time
      vi.advanceTimersByTime(100);

      // Should NOT have emitted onPeerOnline because we disconnected again
      expect(events.onPeerOnline).not.toHaveBeenCalled();
      expect(detector.isOffline(nodeId)).toBe(true);
    });

    it('should cancel pending online transition on disconnect', () => {
      const nodeId = 'peer-1';
      detector.handlePeerDeparted(nodeId);

      // Start reconnection
      detector.recordPeerActivity(nodeId);
      vi.advanceTimersByTime(50);

      // Disconnect again
      detector.handlePeerDeparted(nodeId);
      vi.advanceTimersByTime(200);

      // onPeerOnline should NOT have been called
      expect(events.onPeerOnline).not.toHaveBeenCalled();
    });
  });

  describe('peer removal', () => {
    it('should remove peer from tracking', () => {
      const nodeId = 'peer-1';
      detector.recordPeerActivity(nodeId);
      detector.handlePeerDeparted(nodeId);

      detector.removePeer(nodeId);

      expect(detector.isOffline(nodeId)).toBe(false);
      expect(detector.getLastSeen(nodeId)).toBeUndefined();
      expect(detector.getOfflinePeerInfo(nodeId)).toBeUndefined();
    });

    it('should cancel pending online timer on removal', () => {
      const nodeId = 'peer-1';
      detector.handlePeerDeparted(nodeId);
      detector.recordPeerActivity(nodeId);

      // Remove before debounce completes
      detector.removePeer(nodeId);
      vi.advanceTimersByTime(200);

      // Should not emit onPeerOnline after removal
      expect(events.onPeerOnline).not.toHaveBeenCalled();
    });
  });

  describe('destroy', () => {
    it('should clear all state and timers', () => {
      detector.recordPeerActivity('peer-1');
      detector.handlePeerDeparted('peer-2');

      // Start a pending online transition
      detector.handlePeerDeparted('peer-3');
      detector.recordPeerActivity('peer-3');

      detector.destroy();

      expect(detector.getOfflinePeers()).toHaveLength(0);
      expect(detector.getLastSeen('peer-1')).toBeUndefined();

      // Advance time - no events should fire
      vi.advanceTimersByTime(200);
      expect(events.onPeerOnline).not.toHaveBeenCalled();
    });
  });
});
