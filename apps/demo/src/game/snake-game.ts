/**
 * Snake Game Engine (Story 4.5 - Task 3)
 *
 * Core game logic for multiplayer Snake.
 * P1 (host) runs authoritative game loop.
 * P2 (client) only receives state updates.
 *
 * @see 4-5-demo-snake-multiplayer-p2p-game.md for architecture
 */

import {
  DEFAULT_GRID_SIZE,
  DEFAULT_TICK_MS,
  type Direction,
  type GameEndReason,
  type GameStatePayload,
  type GameWinner,
  INITIAL_SNAKE_LENGTH,
  P1_START,
  P2_START,
  type PlayerId,
  type Point,
} from './game-types';

/** Game state snapshot */
export interface GameState {
  tick: number;
  snakes: {
    p1: Point[];
    p2: Point[];
  };
  directions: {
    p1: Direction;
    p2: Direction;
  };
  food: Point;
  scores: {
    p1: number;
    p2: number;
  };
  isOver: boolean;
  winner: GameWinner | null;
  endReason: GameEndReason | null;
}

/** Game configuration */
export interface GameConfig {
  gridSize: number;
  tickMs: number;
}

/** Game event callbacks */
export interface GameEvents {
  onStateUpdate?: (state: GameState) => void;
  onGameEnd?: (winner: GameWinner, reason: GameEndReason, scores: { p1: number; p2: number }) => void;
}

/**
 * Snake Game Engine
 *
 * Manages game state, collision detection, and game loop.
 * Only runs game loop on P1 (authoritative host).
 */
export class SnakeGame {
  private state: GameState;
  private config: GameConfig;
  private events: GameEvents;
  private gameLoopInterval: ReturnType<typeof setInterval> | null = null;
  private pendingDirections: { p1: Direction | null; p2: Direction | null } = { p1: null, p2: null };

  constructor(config: Partial<GameConfig> = {}, events: GameEvents = {}) {
    this.config = {
      gridSize: config.gridSize ?? DEFAULT_GRID_SIZE,
      tickMs: config.tickMs ?? DEFAULT_TICK_MS,
    };
    this.events = events;
    this.state = this.createInitialState();
  }

  /**
   * Create initial game state with snakes at starting positions
   */
  private createInitialState(): GameState {
    return {
      tick: 0,
      snakes: {
        p1: this.createInitialSnake(P1_START, 'right'),
        p2: this.createInitialSnake(P2_START, 'left'),
      },
      directions: {
        p1: 'right',
        p2: 'left',
      },
      food: this.spawnFood(this.createInitialSnake(P1_START, 'right'), this.createInitialSnake(P2_START, 'left')),
      scores: { p1: 0, p2: 0 },
      isOver: false,
      winner: null,
      endReason: null,
    };
  }

  /**
   * Create initial snake body from head position and direction
   */
  private createInitialSnake(head: Point, direction: Direction): Point[] {
    const snake: Point[] = [head];
    const dx = direction === 'left' ? 1 : direction === 'right' ? -1 : 0;
    const dy = direction === 'up' ? 1 : direction === 'down' ? -1 : 0;

    for (let i = 1; i < INITIAL_SNAKE_LENGTH; i++) {
      snake.push({
        x: head.x + dx * i,
        y: head.y + dy * i,
      });
    }
    return snake;
  }

  /**
   * Spawn food at random empty position
   */
  private spawnFood(snake1: Point[], snake2: Point[]): Point {
    const occupied = new Set<string>();
    for (const p of [...snake1, ...snake2]) {
      occupied.add(`${p.x},${p.y}`);
    }

    const empty: Point[] = [];
    for (let x = 0; x < this.config.gridSize; x++) {
      for (let y = 0; y < this.config.gridSize; y++) {
        if (!occupied.has(`${x},${y}`)) {
          empty.push({ x, y });
        }
      }
    }

    if (empty.length === 0) {
      // Grid is full - very unlikely but handle it
      return { x: Math.floor(this.config.gridSize / 2), y: Math.floor(this.config.gridSize / 2) };
    }

    return empty[Math.floor(Math.random() * empty.length)];
  }

  /**
   * Start the game loop (only call on P1/host)
   */
  start(): void {
    if (this.gameLoopInterval) return;

    this.gameLoopInterval = setInterval(() => {
      this.tick();
    }, this.config.tickMs);
  }

  /**
   * Stop the game loop
   */
  stop(): void {
    if (this.gameLoopInterval) {
      clearInterval(this.gameLoopInterval);
      this.gameLoopInterval = null;
    }
  }

  /**
   * Process one game tick
   */
  tick(): void {
    if (this.state.isOver) {
      this.stop();
      return;
    }

    // Apply pending direction changes
    if (this.pendingDirections.p1 && this.isValidDirectionChange('p1', this.pendingDirections.p1)) {
      this.state.directions.p1 = this.pendingDirections.p1;
    }
    if (this.pendingDirections.p2 && this.isValidDirectionChange('p2', this.pendingDirections.p2)) {
      this.state.directions.p2 = this.pendingDirections.p2;
    }
    this.pendingDirections = { p1: null, p2: null };

    // Move snakes
    const newHead1 = this.getNextHead(this.state.snakes.p1[0], this.state.directions.p1);
    const newHead2 = this.getNextHead(this.state.snakes.p2[0], this.state.directions.p2);

    // Check collisions before moving
    const p1Collision = this.checkCollision(newHead1, 'p1', newHead2);
    const p2Collision = this.checkCollision(newHead2, 'p2', newHead1);

    if (p1Collision && p2Collision) {
      // Both collided - draw
      this.endGame('draw', 'collision');
      return;
    }
    if (p1Collision) {
      this.endGame('p2', 'collision');
      return;
    }
    if (p2Collision) {
      this.endGame('p1', 'collision');
      return;
    }

    // Move snakes (add new head)
    this.state.snakes.p1.unshift(newHead1);
    this.state.snakes.p2.unshift(newHead2);

    // Check food consumption
    const p1AteFood = this.pointsEqual(newHead1, this.state.food);
    const p2AteFood = this.pointsEqual(newHead2, this.state.food);

    if (p1AteFood) {
      this.state.scores.p1++;
    } else {
      this.state.snakes.p1.pop();
    }

    if (p2AteFood) {
      this.state.scores.p2++;
    } else {
      this.state.snakes.p2.pop();
    }

    // Spawn new food if eaten
    if (p1AteFood || p2AteFood) {
      this.state.food = this.spawnFood(this.state.snakes.p1, this.state.snakes.p2);
    }

    this.state.tick++;
    this.events.onStateUpdate?.(this.getState());
  }

  /**
   * Check if direction change is valid (can't reverse)
   */
  private isValidDirectionChange(player: PlayerId, newDir: Direction): boolean {
    const currentDir = this.state.directions[player];
    const opposites: Record<Direction, Direction> = {
      up: 'down',
      down: 'up',
      left: 'right',
      right: 'left',
    };
    return opposites[currentDir] !== newDir;
  }

  /**
   * Get next head position based on direction
   */
  private getNextHead(head: Point, direction: Direction): Point {
    const moves: Record<Direction, Point> = {
      up: { x: head.x, y: head.y - 1 },
      down: { x: head.x, y: head.y + 1 },
      left: { x: head.x - 1, y: head.y },
      right: { x: head.x + 1, y: head.y },
    };
    return moves[direction];
  }

  /**
   * Check if new head position causes collision
   */
  private checkCollision(newHead: Point, player: PlayerId, otherNewHead: Point): boolean {
    // Wall collision
    if (newHead.x < 0 || newHead.x >= this.config.gridSize || newHead.y < 0 || newHead.y >= this.config.gridSize) {
      return true;
    }

    // Self collision (skip head, it's about to move)
    const ownSnake = this.state.snakes[player];
    for (let i = 1; i < ownSnake.length; i++) {
      if (this.pointsEqual(newHead, ownSnake[i])) {
        return true;
      }
    }

    // Opponent collision (check entire body including where new head will be)
    const opponent = player === 'p1' ? 'p2' : 'p1';
    const opponentSnake = this.state.snakes[opponent];
    for (const segment of opponentSnake) {
      if (this.pointsEqual(newHead, segment)) {
        return true;
      }
    }

    // Head-to-head collision (both moving to same spot)
    if (this.pointsEqual(newHead, otherNewHead)) {
      return true;
    }

    return false;
  }

  /**
   * Compare two points for equality
   */
  private pointsEqual(a: Point, b: Point): boolean {
    return a.x === b.x && a.y === b.y;
  }

  /**
   * End the game
   */
  private endGame(winner: GameWinner, reason: GameEndReason): void {
    this.state.isOver = true;
    this.state.winner = winner;
    this.state.endReason = reason;
    this.stop();
    this.events.onGameEnd?.(winner, reason, { ...this.state.scores });
  }

  /**
   * End game due to disconnect
   */
  endByDisconnect(disconnectedPlayer: PlayerId): void {
    const winner = disconnectedPlayer === 'p1' ? 'p2' : 'p1';
    this.endGame(winner, 'disconnect');
  }

  /**
   * Set direction for a player (queued for next tick)
   */
  setDirection(player: PlayerId, direction: Direction): void {
    this.pendingDirections[player] = direction;
  }

  /**
   * Get current game state (immutable copy)
   */
  getState(): GameState {
    return {
      ...this.state,
      snakes: {
        p1: [...this.state.snakes.p1],
        p2: [...this.state.snakes.p2],
      },
      directions: { ...this.state.directions },
      scores: { ...this.state.scores },
    };
  }

  /**
   * Apply state from P1 (for P2 to sync)
   * Includes security validations (Fix #9, #10)
   */
  applyState(payload: GameStatePayload): void {
    // Fix #9: Verify tick monotony (must be > current, prevent replay attacks)
    if (payload.tick <= this.state.tick) {
      console.warn(
        '[SnakeGame] Rejected state: tick not monotonically increasing',
        payload.tick,
        '<=',
        this.state.tick,
      );
      return;
    }

    // Fix #10: Validate coordinates are within grid bounds
    const gridSize = this.config.gridSize;
    const isValidPoint = (p: Point): boolean =>
      typeof p.x === 'number' &&
      typeof p.y === 'number' &&
      Number.isInteger(p.x) &&
      Number.isInteger(p.y) &&
      p.x >= 0 &&
      p.x < gridSize &&
      p.y >= 0 &&
      p.y < gridSize;

    // Validate all snake segments
    for (const segment of payload.snakes.p1) {
      if (!isValidPoint(segment)) {
        console.warn('[SnakeGame] Rejected state: invalid p1 snake coordinates');
        return;
      }
    }
    for (const segment of payload.snakes.p2) {
      if (!isValidPoint(segment)) {
        console.warn('[SnakeGame] Rejected state: invalid p2 snake coordinates');
        return;
      }
    }

    // Validate food position
    if (!isValidPoint(payload.food)) {
      console.warn('[SnakeGame] Rejected state: invalid food coordinates');
      return;
    }

    // Validate scores are non-negative integers
    if (
      !Number.isInteger(payload.scores.p1) ||
      payload.scores.p1 < 0 ||
      !Number.isInteger(payload.scores.p2) ||
      payload.scores.p2 < 0
    ) {
      console.warn('[SnakeGame] Rejected state: invalid scores');
      return;
    }

    // Apply validated state
    this.state.tick = payload.tick;
    this.state.snakes = {
      p1: [...payload.snakes.p1],
      p2: [...payload.snakes.p2],
    };
    this.state.food = { ...payload.food };
    this.state.scores = { ...payload.scores };
    this.state.directions = { ...payload.directions };
    this.events.onStateUpdate?.(this.getState());
  }

  /**
   * Convert current state to payload for network transmission
   */
  toStatePayload(gameId: string): GameStatePayload {
    return {
      type: 'game-state',
      gameId,
      tick: this.state.tick,
      snakes: {
        p1: [...this.state.snakes.p1],
        p2: [...this.state.snakes.p2],
      },
      food: { ...this.state.food },
      scores: { ...this.state.scores },
      directions: { ...this.state.directions },
    };
  }

  /**
   * Get game configuration
   */
  getConfig(): GameConfig {
    return { ...this.config };
  }

  /**
   * Check if game is over
   */
  isGameOver(): boolean {
    return this.state.isOver;
  }

  /**
   * Reset game to initial state
   */
  reset(): void {
    this.stop();
    this.state = this.createInitialState();
    this.pendingDirections = { p1: null, p2: null };
  }
}
