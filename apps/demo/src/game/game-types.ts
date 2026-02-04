/**
 * Game Message Protocol Types (Story 4.5)
 *
 * Application-level game payloads transmitted via TomClient.sendMessage().
 * No changes to core protocol — these are just payload shapes.
 *
 * @see architecture.md#ADR-001 for message transport
 */

/** 2D point on the game grid */
export interface Point {
  x: number;
  y: number;
}

/** Direction for snake movement */
export type Direction = 'up' | 'down' | 'left' | 'right';

/** Player identifier (P1 is host/inviter, P2 is client/invitee) */
export type PlayerId = 'p1' | 'p2';

/** Game end reason */
export type GameEndReason = 'collision' | 'disconnect' | 'forfeit';

/** Winner result */
export type GameWinner = 'p1' | 'p2' | 'draw';

// ============================================
// Game Message Payloads
// ============================================

/**
 * Game invitation sent by P1 to P2
 */
export interface GameInvitePayload {
  type: 'game-invite';
  gameId: string;
  gameType: 'snake';
  gridSize: number;
  tickMs: number;
}

/**
 * Game acceptance from P2 to P1
 */
export interface GameAcceptPayload {
  type: 'game-accept';
  gameId: string;
}

/**
 * Game decline from P2 to P1
 */
export interface GameDeclinePayload {
  type: 'game-decline';
  gameId: string;
}

/**
 * Game state broadcast from P1 (authoritative) to P2 every tick
 */
export interface GameStatePayload {
  type: 'game-state';
  gameId: string;
  tick: number;
  snakes: {
    p1: Point[];
    p2: Point[];
  };
  food: Point;
  scores: {
    p1: number;
    p2: number;
  };
  /** Direction each snake is currently moving */
  directions: {
    p1: Direction;
    p2: Direction;
  };
}

/**
 * Player input sent from P2 to P1 (P1 processes locally)
 */
export interface GameInputPayload {
  type: 'game-input';
  gameId: string;
  direction: Direction;
}

/**
 * Game end notification
 */
export interface GameEndPayload {
  type: 'game-end';
  gameId: string;
  winner: GameWinner;
  reason: GameEndReason;
  finalScores: {
    p1: number;
    p2: number;
  };
}

/**
 * Game ready signal (P2 confirms they are ready to start)
 */
export interface GameReadyPayload {
  type: 'game-ready';
  gameId: string;
}

/**
 * Union type for all game payloads
 */
export type GamePayload =
  | GameInvitePayload
  | GameAcceptPayload
  | GameDeclinePayload
  | GameStatePayload
  | GameInputPayload
  | GameEndPayload
  | GameReadyPayload;

/**
 * Type guard to check if a payload is a game payload
 */
export function isGamePayload(payload: unknown): payload is GamePayload {
  if (!payload || typeof payload !== 'object') return false;
  const p = payload as { type?: string };
  return (
    p.type === 'game-invite' ||
    p.type === 'game-accept' ||
    p.type === 'game-decline' ||
    p.type === 'game-state' ||
    p.type === 'game-input' ||
    p.type === 'game-end' ||
    p.type === 'game-ready'
  );
}

/**
 * Type guards for specific game payloads
 */
export function isGameInvite(payload: unknown): payload is GameInvitePayload {
  return isGamePayload(payload) && payload.type === 'game-invite';
}

export function isGameAccept(payload: unknown): payload is GameAcceptPayload {
  return isGamePayload(payload) && payload.type === 'game-accept';
}

export function isGameDecline(payload: unknown): payload is GameDeclinePayload {
  return isGamePayload(payload) && payload.type === 'game-decline';
}

export function isGameState(payload: unknown): payload is GameStatePayload {
  return isGamePayload(payload) && payload.type === 'game-state';
}

export function isGameInput(payload: unknown): payload is GameInputPayload {
  return isGamePayload(payload) && payload.type === 'game-input';
}

export function isGameEnd(payload: unknown): payload is GameEndPayload {
  return isGamePayload(payload) && payload.type === 'game-end';
}

export function isGameReady(payload: unknown): payload is GameReadyPayload {
  return isGamePayload(payload) && payload.type === 'game-ready';
}

// ============================================
// Game Constants
// ============================================

/** Default game configuration */
export const DEFAULT_GRID_SIZE = 20;
export const DEFAULT_TICK_MS = 100;

/** Snake starting positions (opposite corners) */
export const P1_START: Point = { x: 3, y: 3 };
export const P2_START: Point = { x: 16, y: 16 };

/** Initial snake length */
export const INITIAL_SNAKE_LENGTH = 3;

/** Colors for rendering */
export const COLORS = {
  p1: '#00d4ff', // Cyan for P1
  p2: '#ff00ff', // Magenta for P2
  food: '#00ff88', // Green for food
  grid: '#1a1a2e', // Background
  gridLine: '#0f3460', // Grid lines
  wall: '#ff4444', // Wall collision indicator
} as const;
