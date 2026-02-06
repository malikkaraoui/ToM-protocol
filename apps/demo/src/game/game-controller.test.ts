/**
 * Game Controller Tests (Story 4.5 - Code Review Fix #3)
 *
 * Tests for GameController session management, invitation flow,
 * state synchronization, and connection resilience.
 */

import type { TomClient } from 'tom-sdk';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { GameController, type GameControllerEvents, type GameSession } from './game-controller';
import type {
  GameAcceptPayload,
  GameDeclinePayload,
  GameEndPayload,
  GameInputPayload,
  GameInvitePayload,
  GameReadyPayload,
  GameStatePayload,
} from './game-types';

// Mock TomClient
function createMockClient(): TomClient {
  return {
    sendPayload: vi.fn().mockResolvedValue(undefined),
    getNodeId: vi.fn().mockReturnValue('mock-node-id'),
  } as unknown as TomClient;
}

describe('GameController', () => {
  let controller: GameController;
  let mockClient: TomClient;
  let events: GameControllerEvents;

  beforeEach(() => {
    vi.useFakeTimers();
    mockClient = createMockClient();
    events = {
      onSessionStateChange: vi.fn(),
      onGameStateUpdate: vi.fn(),
      onGameEnd: vi.fn(),
      onInvitationReceived: vi.fn(),
      onInvitationDeclined: vi.fn(),
      onConnectionQualityChange: vi.fn(),
    };
    controller = new GameController(mockClient, events);
  });

  afterEach(() => {
    controller.endSession();
    vi.useRealTimers();
  });

  describe('initialization', () => {
    it('should create controller with no active session', () => {
      expect(controller.getSession()).toBeNull();
      expect(controller.isInGame()).toBe(false);
      expect(controller.canStartGame()).toBe(true);
    });
  });

  describe('invitation flow - sender (P1)', () => {
    it('should send invitation and create session as P1', async () => {
      await controller.sendInvitation('peer-123', 'Alice');

      expect(mockClient.sendPayload).toHaveBeenCalledWith(
        'peer-123',
        expect.objectContaining({
          type: 'game-invite',
          gameType: 'snake',
          gridSize: 20,
          tickMs: 100,
        }),
      );

      const session = controller.getSession();
      expect(session).not.toBeNull();
      expect(session?.localPlayer).toBe('p1');
      expect(session?.state).toBe('waiting-accept');
      expect(session?.peerId).toBe('peer-123');
      expect(session?.peerUsername).toBe('Alice');
    });

    it('should not send invitation when already in game', async () => {
      await controller.sendInvitation('peer-123', 'Alice');
      await controller.sendInvitation('peer-456', 'Bob');

      // Should only have called sendPayload once
      expect(mockClient.sendPayload).toHaveBeenCalledTimes(1);
    });

    it('should emit session state change on invitation', async () => {
      await controller.sendInvitation('peer-123', 'Alice');

      expect(events.onSessionStateChange).toHaveBeenCalledWith('waiting-accept', expect.any(Object));
    });
  });

  describe('invitation flow - receiver (P2)', () => {
    it('should handle incoming invitation and create session as P2', () => {
      const invite: GameInvitePayload = {
        type: 'game-invite',
        gameId: 'game-123',
        gameType: 'snake',
        gridSize: 20,
        tickMs: 100,
      };

      controller.handleGamePayload(invite, 'peer-123', 'Alice');

      const session = controller.getSession();
      expect(session).not.toBeNull();
      expect(session?.localPlayer).toBe('p2');
      expect(session?.state).toBe('invited');
      expect(events.onInvitationReceived).toHaveBeenCalledWith('peer-123', 'Alice', 'game-123');
    });

    it('should accept invitation and wait for game start (GPT-5.2 fix)', async () => {
      const invite: GameInvitePayload = {
        type: 'game-invite',
        gameId: 'game-123',
        gameType: 'snake',
        gridSize: 20,
        tickMs: 100,
      };

      controller.handleGamePayload(invite, 'peer-123', 'Alice');
      await controller.acceptInvitation();

      // P2 sends game-accept
      expect(mockClient.sendPayload).toHaveBeenCalledWith(
        'peer-123',
        expect.objectContaining({
          type: 'game-accept',
          gameId: 'game-123',
        }),
      );

      // GPT-5.2 Fix: P2 now waits for first game-state instead of running countdown
      const session = controller.getSession();
      expect(session?.state).toBe('waiting-game-start');

      // P2 also sends game-ready
      expect(mockClient.sendPayload).toHaveBeenCalledWith(
        'peer-123',
        expect.objectContaining({
          type: 'game-ready',
          gameId: 'game-123',
        }),
      );
    });

    it('should decline invitation and end session', async () => {
      const invite: GameInvitePayload = {
        type: 'game-invite',
        gameId: 'game-123',
        gameType: 'snake',
        gridSize: 20,
        tickMs: 100,
      };

      controller.handleGamePayload(invite, 'peer-123', 'Alice');
      await controller.declineInvitation();

      expect(mockClient.sendPayload).toHaveBeenCalledWith(
        'peer-123',
        expect.objectContaining({
          type: 'game-decline',
          gameId: 'game-123',
        }),
      );

      expect(controller.getSession()).toBeNull();
    });

    it('should auto-decline if already in game', async () => {
      // First invitation
      const invite1: GameInvitePayload = {
        type: 'game-invite',
        gameId: 'game-123',
        gameType: 'snake',
        gridSize: 20,
        tickMs: 100,
      };
      controller.handleGamePayload(invite1, 'peer-123', 'Alice');
      await controller.acceptInvitation();

      // Second invitation while in game
      const invite2: GameInvitePayload = {
        type: 'game-invite',
        gameId: 'game-456',
        gameType: 'snake',
        gridSize: 20,
        tickMs: 100,
      };
      controller.handleGamePayload(invite2, 'peer-789', 'Bob');

      // Should have sent decline
      expect(mockClient.sendPayload).toHaveBeenCalledWith(
        'peer-789',
        expect.objectContaining({
          type: 'game-decline',
          gameId: 'game-456',
        }),
      );
    });
  });

  describe('session state management', () => {
    it('should transition through states correctly for P1', async () => {
      // Send invitation
      await controller.sendInvitation('peer-123', 'Alice');
      expect(controller.getSession()?.state).toBe('waiting-accept');

      // Receive accept
      const accept: GameAcceptPayload = { type: 'game-accept', gameId: controller.getSession()!.gameId };
      controller.handleGamePayload(accept, 'peer-123', 'Alice');
      expect(controller.getSession()?.state).toBe('waiting-ready');

      // Receive ready
      const ready: GameReadyPayload = { type: 'game-ready', gameId: controller.getSession()!.gameId };
      controller.handleGamePayload(ready, 'peer-123', 'Alice');
      expect(controller.getSession()?.state).toBe('countdown');
    });

    it('should handle decline from peer', async () => {
      await controller.sendInvitation('peer-123', 'Alice');

      const decline: GameDeclinePayload = { type: 'game-decline', gameId: controller.getSession()!.gameId };
      controller.handleGamePayload(decline, 'peer-123', 'Alice');

      expect(controller.getSession()).toBeNull();
      expect(events.onInvitationDeclined).toHaveBeenCalledWith('peer-123');
    });
  });

  describe('security - sender verification', () => {
    it('should reject accept from wrong peer', async () => {
      await controller.sendInvitation('peer-123', 'Alice');
      const gameId = controller.getSession()!.gameId;

      // Accept from different peer
      const accept: GameAcceptPayload = { type: 'game-accept', gameId };
      controller.handleGamePayload(accept, 'peer-WRONG', 'Hacker');

      // State should not change
      expect(controller.getSession()?.state).toBe('waiting-accept');
    });

    it('should reject state updates from wrong peer', async () => {
      // Set up P2 session in playing state
      const invite: GameInvitePayload = {
        type: 'game-invite',
        gameId: 'game-123',
        gameType: 'snake',
        gridSize: 20,
        tickMs: 100,
      };
      controller.handleGamePayload(invite, 'peer-123', 'Alice');
      await controller.acceptInvitation();

      // Force to playing state for test
      const session = controller.getSession();
      if (session) {
        (session as GameSession).state = 'playing';
      }

      const stateUpdate: GameStatePayload = {
        type: 'game-state',
        gameId: 'game-123',
        tick: 1,
        snakes: { p1: [{ x: 3, y: 3 }], p2: [{ x: 16, y: 16 }] },
        food: { x: 10, y: 10 },
        scores: { p1: 0, p2: 0 },
        directions: { p1: 'right', p2: 'left' },
      };

      // State from wrong peer should be ignored
      controller.handleGamePayload(stateUpdate, 'peer-WRONG', 'Hacker');
      // No crash, state ignored
    });
  });

  describe('input handling', () => {
    it('should handle local input for P1', async () => {
      await controller.sendInvitation('peer-123', 'Alice');

      // Accept and start
      const accept: GameAcceptPayload = { type: 'game-accept', gameId: controller.getSession()!.gameId };
      controller.handleGamePayload(accept, 'peer-123', 'Alice');
      const ready: GameReadyPayload = { type: 'game-ready', gameId: controller.getSession()!.gameId };
      controller.handleGamePayload(ready, 'peer-123', 'Alice');

      // Advance countdown
      vi.advanceTimersByTime(4000);

      // P1 input is local, should not send payload
      const sendCallsBefore = (mockClient.sendPayload as ReturnType<typeof vi.fn>).mock.calls.length;
      controller.handleLocalInput('up');

      // After 100ms tick, state is sent
      vi.advanceTimersByTime(100);
      expect((mockClient.sendPayload as ReturnType<typeof vi.fn>).mock.calls.length).toBeGreaterThan(sendCallsBefore);
    });

    it('should rate limit P2 input', async () => {
      // Set up P2 session
      const invite: GameInvitePayload = {
        type: 'game-invite',
        gameId: 'game-123',
        gameType: 'snake',
        gridSize: 20,
        tickMs: 100,
      };
      controller.handleGamePayload(invite, 'peer-123', 'Alice');
      await controller.acceptInvitation();

      // Force to playing state
      const session = controller.getSession();
      if (session) {
        (session as GameSession).state = 'playing';
      }

      // Reset mock
      (mockClient.sendPayload as ReturnType<typeof vi.fn>).mockClear();

      // Send multiple inputs quickly
      controller.handleLocalInput('up');
      controller.handleLocalInput('left');
      controller.handleLocalInput('down');

      // Only first should go through due to rate limiting
      expect(mockClient.sendPayload).toHaveBeenCalledTimes(1);

      // After rate limit period, another input should work
      vi.advanceTimersByTime(60);
      controller.handleLocalInput('right');
      expect(mockClient.sendPayload).toHaveBeenCalledTimes(2);
    });
  });

  describe('connection quality', () => {
    it('should update connection quality', () => {
      controller.setConnectionQuality('relay');
      expect(events.onConnectionQualityChange).toHaveBeenCalledWith('relay');

      controller.setConnectionQuality('direct');
      expect(events.onConnectionQualityChange).toHaveBeenCalledWith('direct');
    });
  });

  describe('peer disconnect', () => {
    it('should end game on peer disconnect during gameplay', async () => {
      await controller.sendInvitation('peer-123', 'Alice');

      // Accept and start
      const accept: GameAcceptPayload = { type: 'game-accept', gameId: controller.getSession()!.gameId };
      controller.handleGamePayload(accept, 'peer-123', 'Alice');
      const ready: GameReadyPayload = { type: 'game-ready', gameId: controller.getSession()!.gameId };
      controller.handleGamePayload(ready, 'peer-123', 'Alice');

      // Advance to playing
      vi.advanceTimersByTime(4000);

      controller.handlePeerDisconnect();

      expect(controller.getSession()?.state).toBe('ended');
      expect(events.onGameEnd).toHaveBeenCalled();
    });

    it('should cancel session on peer disconnect during pre-game', async () => {
      await controller.sendInvitation('peer-123', 'Alice');

      controller.handlePeerDisconnect();

      expect(controller.getSession()?.state).toBe('ended');
    });
  });

  describe('game end handling', () => {
    it('should handle game end from remote (P2 receives from P1)', async () => {
      const invite: GameInvitePayload = {
        type: 'game-invite',
        gameId: 'game-123',
        gameType: 'snake',
        gridSize: 20,
        tickMs: 100,
      };
      controller.handleGamePayload(invite, 'peer-123', 'Alice');
      await controller.acceptInvitation();

      // Force to playing state
      const session = controller.getSession();
      if (session) {
        (session as GameSession).state = 'playing';
      }

      const gameEnd: GameEndPayload = {
        type: 'game-end',
        gameId: 'game-123',
        winner: 'p1',
        reason: 'collision',
        finalScores: { p1: 5, p2: 3 },
      };

      controller.handleGamePayload(gameEnd, 'peer-123', 'Alice');

      expect(controller.getSession()?.state).toBe('ended');
      expect(events.onGameEnd).toHaveBeenCalled();
    });

    it('should ignore game-end from P2 when P1 is authoritative (GPT-5.2 fix)', async () => {
      // P1 sends invitation
      await controller.sendInvitation('peer-123', 'Alice');

      // Accept and start
      const accept: GameAcceptPayload = { type: 'game-accept', gameId: controller.getSession()!.gameId };
      controller.handleGamePayload(accept, 'peer-123', 'Alice');
      const ready: GameReadyPayload = { type: 'game-ready', gameId: controller.getSession()!.gameId };
      controller.handleGamePayload(ready, 'peer-123', 'Alice');

      // Advance to playing
      vi.advanceTimersByTime(4000);
      expect(controller.getSession()?.state).toBe('playing');

      // P2 tries to send game-end (malicious)
      const gameEnd: GameEndPayload = {
        type: 'game-end',
        gameId: controller.getSession()!.gameId,
        winner: 'p2',
        reason: 'collision',
        finalScores: { p1: 0, p2: 10 },
      };

      controller.handleGamePayload(gameEnd, 'peer-123', 'Alice');

      // P1 should ignore - game still playing
      expect(controller.getSession()?.state).toBe('playing');
      expect(events.onGameEnd).not.toHaveBeenCalled();
    });

    it('should transition P2 to playing on first game-state (GPT-5.2 fix)', async () => {
      const invite: GameInvitePayload = {
        type: 'game-invite',
        gameId: 'game-123',
        gameType: 'snake',
        gridSize: 20,
        tickMs: 100,
      };
      controller.handleGamePayload(invite, 'peer-123', 'Alice');
      await controller.acceptInvitation();

      // P2 is in waiting-game-start
      expect(controller.getSession()?.state).toBe('waiting-game-start');

      // Receive first game-state from P1
      const gameState: GameStatePayload = {
        type: 'game-state',
        gameId: 'game-123',
        tick: 1,
        snakes: { p1: [{ x: 4, y: 3 }], p2: [{ x: 15, y: 16 }] },
        food: { x: 10, y: 10 },
        scores: { p1: 0, p2: 0 },
        directions: { p1: 'right', p2: 'left' },
      };

      controller.handleGamePayload(gameState, 'peer-123', 'Alice');

      // P2 should transition to playing
      expect(controller.getSession()?.state).toBe('playing');
    });
  });

  describe('session cleanup', () => {
    it('should clean up session on endSession', async () => {
      await controller.sendInvitation('peer-123', 'Alice');

      controller.endSession();

      expect(controller.getSession()).toBeNull();
      expect(controller.isInGame()).toBe(false);
      expect(controller.canStartGame()).toBe(true);
    });

    it('should allow new game after ended session', async () => {
      await controller.sendInvitation('peer-123', 'Alice');
      controller.endSession();

      await controller.sendInvitation('peer-456', 'Bob');

      expect(controller.getSession()).not.toBeNull();
      expect(controller.getSession()?.peerId).toBe('peer-456');
    });
  });
});
