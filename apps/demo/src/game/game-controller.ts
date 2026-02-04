/**
 * Game Controller (Story 4.5 - Tasks 2, 4, 6, 7)
 *
 * Manages game session lifecycle:
 * - Invitation flow (invite, accept, decline)
 * - Game state synchronization (P1 → P2)
 * - Input handling and transmission
 * - Connection resilience
 * - Game end and result
 */

import type { TomClient } from 'tom-sdk';
import {
  DEFAULT_GRID_SIZE,
  DEFAULT_TICK_MS,
  type Direction,
  type GameAcceptPayload,
  type GameDeclinePayload,
  type GameEndPayload,
  type GameEndReason,
  type GameInputPayload,
  type GameInvitePayload,
  type GamePayload,
  type GameReadyPayload,
  type GameStatePayload,
  type GameWinner,
  type PlayerId,
  isGameAccept,
  isGameDecline,
  isGameEnd,
  isGameInput,
  isGameInvite,
  isGameReady,
  isGameState,
} from './game-types';
import { type GameState, SnakeGame } from './snake-game';
import type { ConnectionQuality, SnakeRenderer } from './snake-renderer';

/** Game session state */
export type GameSessionState =
  | 'idle'
  | 'invited' // P2: received invitation, waiting for decision
  | 'waiting-accept' // P1: sent invitation, waiting for response
  | 'waiting-ready' // P1: accepted, waiting for P2 ready
  | 'countdown' // Both: countdown before game starts
  | 'playing' // Both: game in progress
  | 'ended'; // Both: game over

/** Active game session */
export interface GameSession {
  gameId: string;
  peerId: string;
  peerUsername: string;
  localPlayer: PlayerId;
  state: GameSessionState;
  game: SnakeGame;
  config: { gridSize: number; tickMs: number };
}

/** Game controller events */
export interface GameControllerEvents {
  /** Called when session state changes */
  onSessionStateChange?: (state: GameSessionState, session: GameSession | null) => void;
  /** Called when game state updates (for rendering) */
  onGameStateUpdate?: (state: GameState) => void;
  /** Called when game ends */
  onGameEnd?: (winner: GameWinner, reason: GameEndReason, resultMessage: string) => void;
  /** Called when invitation received */
  onInvitationReceived?: (peerId: string, peerUsername: string, gameId: string) => void;
  /** Called when invitation declined */
  onInvitationDeclined?: (peerId: string) => void;
  /** Called when connection quality changes */
  onConnectionQualityChange?: (quality: ConnectionQuality) => void;
}

/**
 * Game Controller
 *
 * Orchestrates multiplayer Snake game sessions.
 */
/** Minimum interval between input messages in ms (Fix #8: rate limiting) */
const INPUT_RATE_LIMIT_MS = 50;

export class GameController {
  private client: TomClient;
  private events: GameControllerEvents;
  private session: GameSession | null = null;
  private renderer: SnakeRenderer | null = null;
  private connectionQuality: ConnectionQuality = 'direct';
  private countdownInterval: ReturnType<typeof setInterval> | null = null;
  private stateUpdateInterval: ReturnType<typeof setInterval> | null = null;
  private lastInputTime = 0; // Fix #8: Rate limiting (local)
  private lastRemoteInputTime = 0; // Fix #8: Rate limiting (remote)

  constructor(client: TomClient, events: GameControllerEvents = {}) {
    this.client = client;
    this.events = events;
  }

  /**
   * Set the renderer for the game
   */
  setRenderer(renderer: SnakeRenderer): void {
    this.renderer = renderer;
  }

  /**
   * Handle incoming game payload
   */
  handleGamePayload(payload: GamePayload, fromPeerId: string, peerUsername: string): void {
    if (isGameInvite(payload)) {
      this.handleInvite(payload, fromPeerId, peerUsername);
    } else if (isGameAccept(payload)) {
      this.handleAccept(payload, fromPeerId);
    } else if (isGameDecline(payload)) {
      this.handleDecline(payload, fromPeerId);
    } else if (isGameReady(payload)) {
      this.handleReady(payload, fromPeerId);
    } else if (isGameState(payload)) {
      this.handleState(payload, fromPeerId);
    } else if (isGameInput(payload)) {
      this.handleInput(payload, fromPeerId);
    } else if (isGameEnd(payload)) {
      this.handleEnd(payload, fromPeerId);
    }
  }

  /**
   * Send game invitation to peer
   */
  async sendInvitation(peerId: string, peerUsername: string): Promise<void> {
    if (!this.canStartGame()) {
      console.warn('[GameController] Already in an active game session');
      return;
    }

    // Clean up any previous ended session
    if (this.session) {
      this.endSession();
    }

    // Reset rate limiters for new game
    this.lastInputTime = 0;
    this.lastRemoteInputTime = 0;

    const gameId = this.generateGameId();
    const config = { gridSize: DEFAULT_GRID_SIZE, tickMs: DEFAULT_TICK_MS };

    const payload: GameInvitePayload = {
      type: 'game-invite',
      gameId,
      gameType: 'snake',
      gridSize: config.gridSize,
      tickMs: config.tickMs,
    };

    await this.client.sendPayload(peerId, payload);

    // Create session as P1 (host)
    this.session = {
      gameId,
      peerId,
      peerUsername,
      localPlayer: 'p1',
      state: 'waiting-accept',
      game: new SnakeGame(config, {
        onStateUpdate: (state) => this.onGameStateUpdate(state),
        onGameEnd: (winner, reason, scores) => this.onGameEnd(winner, reason, scores),
      }),
      config,
    };

    this.setSessionState('waiting-accept');
  }

  /**
   * Accept pending invitation
   */
  async acceptInvitation(): Promise<void> {
    if (!this.session || this.session.state !== 'invited') {
      console.warn('[GameController] No pending invitation to accept');
      return;
    }

    const payload: GameAcceptPayload = {
      type: 'game-accept',
      gameId: this.session.gameId,
    };

    await this.client.sendPayload(this.session.peerId, payload);
    this.setSessionState('countdown');
    this.startCountdown();
  }

  /**
   * Decline pending invitation
   */
  async declineInvitation(): Promise<void> {
    if (!this.session || this.session.state !== 'invited') {
      console.warn('[GameController] No pending invitation to decline');
      return;
    }

    const payload: GameDeclinePayload = {
      type: 'game-decline',
      gameId: this.session.gameId,
    };

    await this.client.sendPayload(this.session.peerId, payload);
    this.endSession();
  }

  /**
   * Handle local player input
   */
  handleLocalInput(direction: Direction): void {
    if (!this.session || this.session.state !== 'playing') return;

    if (this.session.localPlayer === 'p1') {
      // P1: apply locally (no rate limit needed for local)
      this.session.game.setDirection('p1', direction);
    } else {
      // P2: send to P1 with rate limiting (Fix #8)
      const now = Date.now();
      if (now - this.lastInputTime < INPUT_RATE_LIMIT_MS) {
        return; // Rate limited, skip this input
      }
      this.lastInputTime = now;

      const payload: GameInputPayload = {
        type: 'game-input',
        gameId: this.session.gameId,
        direction,
      };
      this.client.sendPayload(this.session.peerId, payload);
    }
  }

  /**
   * Handle connection quality change (from TomClient events)
   */
  setConnectionQuality(quality: ConnectionQuality): void {
    this.connectionQuality = quality;
    this.renderer?.setConnectionQuality(quality);
    this.events.onConnectionQualityChange?.(quality);
  }

  /**
   * Handle peer disconnect during any game phase
   */
  handlePeerDisconnect(): void {
    if (!this.session) return;

    const state = this.session.state;

    // Already ended, nothing to do
    if (state === 'idle' || state === 'ended') return;

    // During active gameplay, end with disconnect reason
    if (state === 'playing') {
      this.session.game.endByDisconnect(this.session.localPlayer === 'p1' ? 'p2' : 'p1');
      return;
    }

    // During any pre-game phase (invited, waiting-accept, waiting-ready, countdown)
    // Just clean up the session
    this.stopCountdown();
    this.stopStateUpdates();
    this.session.game.stop();

    // Notify about disconnect
    const disconnectedPlayer = this.session.localPlayer === 'p1' ? 'p2' : 'p1';
    const resultMessage = `🎮 Game cancelled (${this.session.peerUsername} disconnected)`;
    this.events.onGameEnd?.(disconnectedPlayer, 'disconnect', resultMessage);

    this.setSessionState('ended');
  }

  /**
   * End current session (e.g., user clicks "return to chat")
   */
  endSession(): void {
    this.stopCountdown();
    this.stopStateUpdates();

    if (this.session) {
      this.session.game.stop();
    }

    this.session = null;
    this.setSessionState('idle');
  }

  /**
   * Get current session
   */
  getSession(): GameSession | null {
    return this.session;
  }

  /**
   * Check if currently in an active game (not ended)
   */
  isInGame(): boolean {
    if (!this.session) return false;
    // 'ended' state allows starting a new game
    return this.session.state !== 'idle' && this.session.state !== 'ended';
  }

  /**
   * Check if can start a new game
   */
  canStartGame(): boolean {
    return !this.session || this.session.state === 'idle' || this.session.state === 'ended';
  }

  // ============================================
  // Private: Invitation Handlers
  // ============================================

  private handleInvite(payload: GameInvitePayload, fromPeerId: string, peerUsername: string): void {
    // Can accept if no session or session is ended
    if (!this.canStartGame()) {
      // Already in an active session, auto-decline
      const declinePayload: GameDeclinePayload = {
        type: 'game-decline',
        gameId: payload.gameId,
      };
      this.client.sendPayload(fromPeerId, declinePayload);
      return;
    }

    // Clean up any previous ended session
    if (this.session) {
      this.endSession();
    }

    // Reset rate limiters for new game
    this.lastInputTime = 0;
    this.lastRemoteInputTime = 0;

    // Create session as P2 (client)
    const config = { gridSize: payload.gridSize, tickMs: payload.tickMs };

    this.session = {
      gameId: payload.gameId,
      peerId: fromPeerId,
      peerUsername,
      localPlayer: 'p2',
      state: 'invited',
      game: new SnakeGame(config, {
        onStateUpdate: (state) => this.onGameStateUpdate(state),
        onGameEnd: (winner, reason, scores) => this.onGameEnd(winner, reason, scores),
      }),
      config,
    };

    this.setSessionState('invited');
    this.events.onInvitationReceived?.(fromPeerId, peerUsername, payload.gameId);
  }

  private handleAccept(payload: GameAcceptPayload, fromPeerId: string): void {
    if (!this.session || this.session.gameId !== payload.gameId) return;
    if (fromPeerId !== this.session.peerId) return; // Security: verify sender
    if (this.session.state !== 'waiting-accept') return;

    this.setSessionState('waiting-ready');
    // Wait for P2 ready signal
  }

  private handleDecline(_payload: GameDeclinePayload, fromPeerId: string): void {
    if (!this.session) return;
    if (fromPeerId !== this.session.peerId) return; // Security: verify sender

    this.events.onInvitationDeclined?.(fromPeerId);
    this.endSession();
  }

  private handleReady(payload: GameReadyPayload, fromPeerId: string): void {
    if (!this.session || this.session.gameId !== payload.gameId) return;
    if (fromPeerId !== this.session.peerId) return; // Security: verify sender
    if (this.session.localPlayer !== 'p1') return;
    if (this.session.state !== 'waiting-ready') return; // Fix #3: Check state

    // P1 received ready from P2 - start countdown
    this.setSessionState('countdown');
    this.startCountdown();
  }

  // ============================================
  // Private: Game State Handlers
  // ============================================

  private handleState(payload: GameStatePayload, fromPeerId: string): void {
    if (!this.session || this.session.gameId !== payload.gameId) return;
    if (fromPeerId !== this.session.peerId) return; // Security: verify sender
    if (this.session.localPlayer !== 'p2') return;
    if (this.session.state !== 'playing') return; // Only accept state during gameplay

    // P2: apply state from P1
    this.session.game.applyState(payload);
  }

  private handleInput(payload: GameInputPayload, fromPeerId: string): void {
    if (!this.session || this.session.gameId !== payload.gameId) return;
    if (fromPeerId !== this.session.peerId) return; // Security: verify sender
    if (this.session.localPlayer !== 'p1') return;
    if (this.session.state !== 'playing') return; // Only accept input during gameplay

    // Fix #8: Rate limit incoming input (prevent DoS)
    const now = Date.now();
    if (now - this.lastRemoteInputTime < INPUT_RATE_LIMIT_MS) {
      return; // Rate limited, ignore
    }
    this.lastRemoteInputTime = now;

    // P1: apply P2's input
    this.session.game.setDirection('p2', payload.direction);
  }

  private handleEnd(payload: GameEndPayload, fromPeerId: string): void {
    if (!this.session || this.session.gameId !== payload.gameId) return;
    if (fromPeerId !== this.session.peerId) return; // Security: verify sender

    // Game ended by remote - clean up intervals (Fix #5)
    this.stopCountdown();
    this.stopStateUpdates();
    this.session.game.stop();
    this.setSessionState('ended');

    const resultMessage = this.formatResultMessage(payload.winner, payload.reason);
    this.events.onGameEnd?.(payload.winner, payload.reason, resultMessage);
  }

  // ============================================
  // Private: Game Loop
  // ============================================

  private startCountdown(): void {
    let count = 3;

    // Render initial countdown
    this.renderer?.renderCountdown(count);

    this.countdownInterval = setInterval(() => {
      count--;
      if (count > 0) {
        this.renderer?.renderCountdown(count);
      } else {
        this.stopCountdown();
        this.startGame();
      }
    }, 1000);

    // P2: send ready signal after accepting
    if (this.session?.localPlayer === 'p2') {
      const payload: GameReadyPayload = {
        type: 'game-ready',
        gameId: this.session.gameId,
      };
      this.client.sendPayload(this.session.peerId, payload);
    }
  }

  private stopCountdown(): void {
    if (this.countdownInterval) {
      clearInterval(this.countdownInterval);
      this.countdownInterval = null;
    }
  }

  private startGame(): void {
    if (!this.session) return;

    this.setSessionState('playing');

    if (this.session.localPlayer === 'p1') {
      // P1: start game loop and state broadcasting
      this.session.game.start();
      this.startStateUpdates();
    }
    // P2 just waits for state updates
  }

  private startStateUpdates(): void {
    if (!this.session || this.session.localPlayer !== 'p1') return;

    // Send state updates at tick rate
    this.stateUpdateInterval = setInterval(() => {
      if (!this.session || this.session.state !== 'playing') {
        this.stopStateUpdates();
        return;
      }

      const payload = this.session.game.toStatePayload(this.session.gameId);
      this.client.sendPayload(this.session.peerId, payload);
    }, this.session.config.tickMs);
  }

  private stopStateUpdates(): void {
    if (this.stateUpdateInterval) {
      clearInterval(this.stateUpdateInterval);
      this.stateUpdateInterval = null;
    }
  }

  // ============================================
  // Private: Event Handlers
  // ============================================

  private onGameStateUpdate(state: GameState): void {
    this.events.onGameStateUpdate?.(state);

    // Render if we have a renderer
    if (this.renderer && this.session) {
      this.renderer.render(state, this.session.localPlayer);
    }
  }

  private onGameEnd(winner: GameWinner, reason: GameEndReason, scores: { p1: number; p2: number }): void {
    if (!this.session) return;

    this.stopStateUpdates();
    this.setSessionState('ended');

    // P1: send game end to P2
    if (this.session.localPlayer === 'p1') {
      const payload: GameEndPayload = {
        type: 'game-end',
        gameId: this.session.gameId,
        winner,
        reason,
        finalScores: scores,
      };
      this.client.sendPayload(this.session.peerId, payload);
    }

    const resultMessage = this.formatResultMessage(winner, reason);
    this.events.onGameEnd?.(winner, reason, resultMessage);

    // Render game over screen
    if (this.renderer) {
      this.renderer.renderGameOver(winner, scores, this.session.localPlayer);
    }
  }

  // ============================================
  // Private: Helpers
  // ============================================

  private setSessionState(state: GameSessionState): void {
    if (this.session) {
      this.session.state = state;
    }
    this.events.onSessionStateChange?.(state, this.session);
  }

  private generateGameId(): string {
    // Use crypto.randomUUID if available, fallback for HTTP contexts (Fix #7)
    if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
      return `game-${crypto.randomUUID()}`;
    }
    // Fallback: generate pseudo-random ID
    const hex = () => Math.floor(Math.random() * 16).toString(16);
    const segment = (len: number) => Array.from({ length: len }, hex).join('');
    return `game-${segment(8)}-${segment(4)}-${segment(4)}-${segment(4)}-${segment(12)}`;
  }

  private formatResultMessage(winner: GameWinner, reason: GameEndReason): string {
    if (!this.session) return '';

    const winnerName =
      winner === 'draw' ? null : winner === this.session.localPlayer ? 'You' : this.session.peerUsername;

    if (winner === 'draw') {
      return "🎮 It's a draw!";
    }

    if (reason === 'disconnect') {
      return `🎮 ${winnerName} won (opponent disconnected)`;
    }

    return `🏆 ${winnerName} won the Snake game!`;
  }
}
