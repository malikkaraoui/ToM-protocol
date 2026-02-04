/**
 * Game Types Tests (Story 4.5 - Task 9)
 */

import { describe, expect, it } from 'vitest';
import {
  COLORS,
  DEFAULT_GRID_SIZE,
  DEFAULT_TICK_MS,
  type GameAcceptPayload,
  type GameDeclinePayload,
  type GameEndPayload,
  type GameInputPayload,
  type GameInvitePayload,
  type GameReadyPayload,
  type GameStatePayload,
  INITIAL_SNAKE_LENGTH,
  P1_START,
  P2_START,
  isGameAccept,
  isGameDecline,
  isGameEnd,
  isGameInput,
  isGameInvite,
  isGamePayload,
  isGameReady,
  isGameState,
} from './game-types';

describe('Game Type Guards', () => {
  describe('isGamePayload', () => {
    it('should return true for valid game payloads', () => {
      const payloads = [
        { type: 'game-invite' },
        { type: 'game-accept' },
        { type: 'game-decline' },
        { type: 'game-state' },
        { type: 'game-input' },
        { type: 'game-end' },
        { type: 'game-ready' },
      ];

      for (const payload of payloads) {
        expect(isGamePayload(payload)).toBe(true);
      }
    });

    it('should return false for non-game payloads', () => {
      expect(isGamePayload(null)).toBe(false);
      expect(isGamePayload(undefined)).toBe(false);
      expect(isGamePayload({})).toBe(false);
      expect(isGamePayload({ type: 'chat-message' })).toBe(false);
      expect(isGamePayload({ type: 'text' })).toBe(false);
      expect(isGamePayload({ text: 'hello' })).toBe(false);
      expect(isGamePayload('string')).toBe(false);
      expect(isGamePayload(123)).toBe(false);
    });
  });

  describe('isGameInvite', () => {
    it('should return true for game invite payload', () => {
      const payload: GameInvitePayload = {
        type: 'game-invite',
        gameId: 'game-123',
        gameType: 'snake',
        gridSize: 20,
        tickMs: 100,
      };
      expect(isGameInvite(payload)).toBe(true);
    });

    it('should return false for other payloads', () => {
      expect(isGameInvite({ type: 'game-accept', gameId: '123' })).toBe(false);
      expect(isGameInvite({ type: 'game-state' })).toBe(false);
      expect(isGameInvite(null)).toBe(false);
    });
  });

  describe('isGameAccept', () => {
    it('should return true for game accept payload', () => {
      const payload: GameAcceptPayload = {
        type: 'game-accept',
        gameId: 'game-123',
      };
      expect(isGameAccept(payload)).toBe(true);
    });

    it('should return false for other payloads', () => {
      expect(isGameAccept({ type: 'game-invite' })).toBe(false);
      expect(isGameAccept(null)).toBe(false);
    });
  });

  describe('isGameDecline', () => {
    it('should return true for game decline payload', () => {
      const payload: GameDeclinePayload = {
        type: 'game-decline',
        gameId: 'game-123',
      };
      expect(isGameDecline(payload)).toBe(true);
    });

    it('should return false for other payloads', () => {
      expect(isGameDecline({ type: 'game-accept' })).toBe(false);
      expect(isGameDecline(null)).toBe(false);
    });
  });

  describe('isGameState', () => {
    it('should return true for game state payload', () => {
      const payload: GameStatePayload = {
        type: 'game-state',
        gameId: 'game-123',
        tick: 10,
        snakes: {
          p1: [{ x: 5, y: 5 }],
          p2: [{ x: 15, y: 15 }],
        },
        food: { x: 10, y: 10 },
        scores: { p1: 0, p2: 0 },
        directions: { p1: 'right', p2: 'left' },
      };
      expect(isGameState(payload)).toBe(true);
    });

    it('should return false for other payloads', () => {
      expect(isGameState({ type: 'game-input' })).toBe(false);
      expect(isGameState(null)).toBe(false);
    });
  });

  describe('isGameInput', () => {
    it('should return true for game input payload', () => {
      const payload: GameInputPayload = {
        type: 'game-input',
        gameId: 'game-123',
        direction: 'up',
      };
      expect(isGameInput(payload)).toBe(true);
    });

    it('should return false for other payloads', () => {
      expect(isGameInput({ type: 'game-state' })).toBe(false);
      expect(isGameInput(null)).toBe(false);
    });
  });

  describe('isGameEnd', () => {
    it('should return true for game end payload', () => {
      const payload: GameEndPayload = {
        type: 'game-end',
        gameId: 'game-123',
        winner: 'p1',
        reason: 'collision',
        finalScores: { p1: 5, p2: 3 },
      };
      expect(isGameEnd(payload)).toBe(true);
    });

    it('should return false for other payloads', () => {
      expect(isGameEnd({ type: 'game-state' })).toBe(false);
      expect(isGameEnd(null)).toBe(false);
    });
  });

  describe('isGameReady', () => {
    it('should return true for game ready payload', () => {
      const payload: GameReadyPayload = {
        type: 'game-ready',
        gameId: 'game-123',
      };
      expect(isGameReady(payload)).toBe(true);
    });

    it('should return false for other payloads', () => {
      expect(isGameReady({ type: 'game-accept' })).toBe(false);
      expect(isGameReady(null)).toBe(false);
    });
  });
});

describe('Game Constants', () => {
  it('should have valid default grid size', () => {
    expect(DEFAULT_GRID_SIZE).toBe(20);
    expect(DEFAULT_GRID_SIZE).toBeGreaterThan(0);
  });

  it('should have valid default tick rate', () => {
    expect(DEFAULT_TICK_MS).toBe(100);
    expect(DEFAULT_TICK_MS).toBeGreaterThan(0);
  });

  it('should have valid starting positions', () => {
    expect(P1_START.x).toBeGreaterThanOrEqual(0);
    expect(P1_START.y).toBeGreaterThanOrEqual(0);
    expect(P2_START.x).toBeGreaterThanOrEqual(0);
    expect(P2_START.y).toBeGreaterThanOrEqual(0);

    // P1 and P2 should start at different positions
    expect(P1_START.x !== P2_START.x || P1_START.y !== P2_START.y).toBe(true);
  });

  it('should have valid initial snake length', () => {
    expect(INITIAL_SNAKE_LENGTH).toBe(3);
    expect(INITIAL_SNAKE_LENGTH).toBeGreaterThan(0);
  });

  it('should have all required colors defined', () => {
    expect(COLORS.p1).toBeDefined();
    expect(COLORS.p2).toBeDefined();
    expect(COLORS.food).toBeDefined();
    expect(COLORS.grid).toBeDefined();
    expect(COLORS.gridLine).toBeDefined();
    expect(COLORS.wall).toBeDefined();

    // Colors should be valid hex codes
    const hexRegex = /^#[0-9a-fA-F]{6}$/;
    expect(COLORS.p1).toMatch(hexRegex);
    expect(COLORS.p2).toMatch(hexRegex);
    expect(COLORS.food).toMatch(hexRegex);
  });
});
