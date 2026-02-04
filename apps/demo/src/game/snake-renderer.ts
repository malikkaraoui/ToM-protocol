/**
 * Snake Game Renderer (Story 4.5 - Task 5)
 *
 * Canvas-based rendering for the Snake game.
 * Renders grid, snakes, food, scores, and overlays.
 */

import { COLORS, DEFAULT_GRID_SIZE, type PlayerId, type Point } from './game-types';
import type { GameState } from './snake-game';

/** Renderer configuration */
export interface RendererConfig {
  gridSize: number;
  cellSize: number;
}

/** Connection quality indicator */
export type ConnectionQuality = 'direct' | 'relay' | 'disconnected';

/**
 * Snake Game Renderer
 *
 * Handles all canvas drawing operations for the game.
 */
export class SnakeRenderer {
  private canvas: HTMLCanvasElement;
  private ctx: CanvasRenderingContext2D;
  private config: RendererConfig;
  private connectionQuality: ConnectionQuality = 'direct';

  constructor(canvas: HTMLCanvasElement, config: Partial<RendererConfig> = {}) {
    this.canvas = canvas;
    const ctx = canvas.getContext('2d');
    if (!ctx) throw new Error('Failed to get canvas 2D context');
    this.ctx = ctx;

    this.config = {
      gridSize: config.gridSize ?? DEFAULT_GRID_SIZE,
      cellSize: config.cellSize ?? 20,
    };

    this.resize();
  }

  /**
   * Resize canvas to fit grid
   */
  resize(): void {
    const size = this.config.gridSize * this.config.cellSize;
    this.canvas.width = size;
    this.canvas.height = size;
    // Set CSS size to match for crisp rendering
    this.canvas.style.width = `${size}px`;
    this.canvas.style.height = `${size}px`;
  }

  /**
   * Set connection quality for indicator
   */
  setConnectionQuality(quality: ConnectionQuality): void {
    this.connectionQuality = quality;
  }

  /**
   * Render the current game state
   */
  render(state: GameState, localPlayer: PlayerId): void {
    this.clear();
    this.drawGrid();
    this.drawFood(state.food);
    this.drawSnake(state.snakes.p1, 'p1', localPlayer);
    this.drawSnake(state.snakes.p2, 'p2', localPlayer);
    this.drawScores(state.scores, localPlayer);
    this.drawConnectionIndicator();
  }

  /**
   * Clear the canvas
   */
  private clear(): void {
    this.ctx.fillStyle = COLORS.grid;
    this.ctx.fillRect(0, 0, this.canvas.width, this.canvas.height);
  }

  /**
   * Draw the grid lines
   */
  private drawGrid(): void {
    this.ctx.strokeStyle = COLORS.gridLine;
    this.ctx.lineWidth = 0.5;

    const size = this.config.gridSize * this.config.cellSize;

    for (let i = 0; i <= this.config.gridSize; i++) {
      const pos = i * this.config.cellSize;
      // Vertical line
      this.ctx.beginPath();
      this.ctx.moveTo(pos, 0);
      this.ctx.lineTo(pos, size);
      this.ctx.stroke();
      // Horizontal line
      this.ctx.beginPath();
      this.ctx.moveTo(0, pos);
      this.ctx.lineTo(size, pos);
      this.ctx.stroke();
    }
  }

  /**
   * Draw food item
   */
  private drawFood(food: Point): void {
    const x = food.x * this.config.cellSize;
    const y = food.y * this.config.cellSize;
    const size = this.config.cellSize;

    // Draw as a circle
    this.ctx.fillStyle = COLORS.food;
    this.ctx.beginPath();
    this.ctx.arc(x + size / 2, y + size / 2, size / 2 - 2, 0, Math.PI * 2);
    this.ctx.fill();
  }

  /**
   * Draw a snake
   */
  private drawSnake(snake: Point[], player: PlayerId, localPlayer: PlayerId): void {
    const color = player === 'p1' ? COLORS.p1 : COLORS.p2;
    const isLocal = player === localPlayer;
    const size = this.config.cellSize;

    for (let i = 0; i < snake.length; i++) {
      const segment = snake[i];
      const x = segment.x * size;
      const y = segment.y * size;

      // Head is slightly different
      if (i === 0) {
        this.ctx.fillStyle = color;
        this.ctx.fillRect(x + 1, y + 1, size - 2, size - 2);
        // Draw eyes on head
        this.ctx.fillStyle = '#fff';
        const eyeSize = 3;
        this.ctx.fillRect(x + 4, y + 4, eyeSize, eyeSize);
        this.ctx.fillRect(x + size - 7, y + 4, eyeSize, eyeSize);
      } else {
        // Body segment - slightly smaller and with gap
        this.ctx.fillStyle = color;
        this.ctx.globalAlpha = isLocal ? 1 : 0.8;
        this.ctx.fillRect(x + 2, y + 2, size - 4, size - 4);
        this.ctx.globalAlpha = 1;
      }
    }
  }

  /**
   * Draw scores overlay
   */
  private drawScores(scores: { p1: number; p2: number }, localPlayer: PlayerId): void {
    this.ctx.font = 'bold 14px monospace';
    this.ctx.textAlign = 'left';

    // P1 score (top-left)
    this.ctx.fillStyle = COLORS.p1;
    const p1Label = localPlayer === 'p1' ? 'You' : 'P1';
    this.ctx.fillText(`${p1Label}: ${scores.p1}`, 10, 20);

    // P2 score (top-right)
    this.ctx.fillStyle = COLORS.p2;
    this.ctx.textAlign = 'right';
    const p2Label = localPlayer === 'p2' ? 'You' : 'P2';
    this.ctx.fillText(`${p2Label}: ${scores.p2}`, this.canvas.width - 10, 20);
  }

  /**
   * Draw connection quality indicator
   */
  private drawConnectionIndicator(): void {
    const colors: Record<ConnectionQuality, string> = {
      direct: '#00ff88',
      relay: '#ffaa00',
      disconnected: '#ff4444',
    };

    const labels: Record<ConnectionQuality, string> = {
      direct: '⚡',
      relay: '🔀',
      disconnected: '⚠️',
    };

    this.ctx.fillStyle = colors[this.connectionQuality];
    this.ctx.font = '12px monospace';
    this.ctx.textAlign = 'center';
    this.ctx.fillText(labels[this.connectionQuality], this.canvas.width / 2, 20);
  }

  /**
   * Render "Waiting for opponent" screen
   */
  renderWaiting(): void {
    this.clear();
    this.drawGrid();

    this.ctx.fillStyle = 'rgba(0, 0, 0, 0.7)';
    this.ctx.fillRect(0, 0, this.canvas.width, this.canvas.height);

    this.ctx.fillStyle = '#00d4ff';
    this.ctx.font = 'bold 18px monospace';
    this.ctx.textAlign = 'center';
    this.ctx.fillText('Waiting for opponent...', this.canvas.width / 2, this.canvas.height / 2);
  }

  /**
   * Render countdown
   */
  renderCountdown(count: number): void {
    this.ctx.fillStyle = 'rgba(0, 0, 0, 0.5)';
    this.ctx.fillRect(0, 0, this.canvas.width, this.canvas.height);

    this.ctx.fillStyle = '#00d4ff';
    this.ctx.font = 'bold 48px monospace';
    this.ctx.textAlign = 'center';
    this.ctx.fillText(String(count), this.canvas.width / 2, this.canvas.height / 2 + 15);
  }

  /**
   * Render game over screen
   */
  renderGameOver(winner: 'p1' | 'p2' | 'draw', scores: { p1: number; p2: number }, localPlayer: PlayerId): void {
    // Semi-transparent overlay
    this.ctx.fillStyle = 'rgba(0, 0, 0, 0.8)';
    this.ctx.fillRect(0, 0, this.canvas.width, this.canvas.height);

    this.ctx.textAlign = 'center';
    const centerX = this.canvas.width / 2;
    const centerY = this.canvas.height / 2;

    // Winner text
    let resultText: string;
    let resultColor: string;

    if (winner === 'draw') {
      resultText = "It's a Draw!";
      resultColor = '#ffaa00';
    } else if (winner === localPlayer) {
      resultText = '🏆 You Win!';
      resultColor = '#00ff88';
    } else {
      resultText = 'You Lose';
      resultColor = '#ff4444';
    }

    this.ctx.fillStyle = resultColor;
    this.ctx.font = 'bold 28px monospace';
    this.ctx.fillText(resultText, centerX, centerY - 30);

    // Final scores
    this.ctx.fillStyle = '#e0e0e0';
    this.ctx.font = '16px monospace';
    this.ctx.fillText(`Final Score: ${scores.p1} - ${scores.p2}`, centerX, centerY + 10);

    // Return to chat hint
    this.ctx.fillStyle = '#888';
    this.ctx.font = '14px monospace';
    this.ctx.fillText('Click to return to chat', centerX, centerY + 50);
  }

  /**
   * Get canvas element
   */
  getCanvas(): HTMLCanvasElement {
    return this.canvas;
  }
}
