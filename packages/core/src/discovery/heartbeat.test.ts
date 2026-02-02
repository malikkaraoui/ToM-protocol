import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { HeartbeatManager } from './heartbeat.js';

describe('HeartbeatManager', () => {
  let onPeerStale: ReturnType<typeof vi.fn>;
  let onPeerDeparted: ReturnType<typeof vi.fn>;
  let broadcastHeartbeat: ReturnType<typeof vi.fn>;
  let manager: HeartbeatManager;

  beforeEach(() => {
    vi.useFakeTimers();
    onPeerStale = vi.fn();
    onPeerDeparted = vi.fn();
    broadcastHeartbeat = vi.fn();
    manager = new HeartbeatManager(
      { sendHeartbeat: vi.fn(), broadcastHeartbeat },
      { onPeerStale, onPeerDeparted },
      5000,
      3000,
    );
  });

  afterEach(() => {
    manager.stop();
    vi.useRealTimers();
  });

  it('should send periodic heartbeats', () => {
    manager.start();
    vi.advanceTimersByTime(5000);
    expect(broadcastHeartbeat).toHaveBeenCalledTimes(1);
    vi.advanceTimersByTime(5000);
    expect(broadcastHeartbeat).toHaveBeenCalledTimes(2);
  });

  it('should record heartbeat and keep peer alive', () => {
    manager.start();
    manager.trackPeer('peer-1');
    // Peer sends heartbeat before timeout
    vi.advanceTimersByTime(2000);
    manager.recordHeartbeat('peer-1');
    vi.advanceTimersByTime(2000);
    expect(onPeerStale).not.toHaveBeenCalled();
    expect(onPeerDeparted).not.toHaveBeenCalled();
  });

  it('should emit stale when no heartbeat for 3s', () => {
    manager.start();
    manager.trackPeer('peer-1');
    vi.advanceTimersByTime(4000); // > 3s timeout
    expect(onPeerStale).toHaveBeenCalledWith('peer-1');
  });

  it('should emit departed when no heartbeat for 6s', () => {
    manager.start();
    manager.trackPeer('peer-1');
    vi.advanceTimersByTime(7000); // > 6s (timeout * 2)
    expect(onPeerDeparted).toHaveBeenCalledWith('peer-1');
  });

  it('should not emit stale again after recording heartbeat', () => {
    manager.start();
    manager.trackPeer('peer-1');
    vi.advanceTimersByTime(4000);
    expect(onPeerStale).toHaveBeenCalledTimes(1);
    manager.recordHeartbeat('peer-1');
    vi.advanceTimersByTime(2000);
    expect(onPeerStale).toHaveBeenCalledTimes(1);
  });

  it('should stop tracking untracked peer', () => {
    manager.start();
    manager.trackPeer('peer-1');
    manager.untrackPeer('peer-1');
    vi.advanceTimersByTime(10000);
    expect(onPeerStale).not.toHaveBeenCalled();
    expect(onPeerDeparted).not.toHaveBeenCalled();
  });

  it('should cleanup on stop', () => {
    manager.start();
    manager.trackPeer('peer-1');
    manager.stop();
    vi.advanceTimersByTime(10000);
    expect(broadcastHeartbeat).not.toHaveBeenCalled();
    expect(onPeerStale).not.toHaveBeenCalled();
  });
});
