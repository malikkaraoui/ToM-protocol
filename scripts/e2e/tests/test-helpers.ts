/**
 * E2E Test Helpers - Robust Testing Infrastructure
 *
 * Provides retry mechanisms, extended timeouts, and reliable
 * state synchronization for E2E tests.
 */

import type { Page } from '@playwright/test';

/** Retry configuration */
export interface RetryConfig {
  maxAttempts: number;
  delayMs: number;
  backoffMultiplier: number;
}

/** Default retry configuration */
export const DEFAULT_RETRY_CONFIG: RetryConfig = {
  maxAttempts: 3,
  delayMs: 1000,
  backoffMultiplier: 1.5,
};

/** Extended timeouts for operations after hub disconnect */
export const POST_DISCONNECT_TIMEOUTS = {
  /** Wait for hub failover to complete */
  HUB_RECOVERY_MS: 10000,
  /** Wait for group state sync */
  GROUP_SYNC_MS: 8000,
  /** Wait for message delivery after disconnect */
  MESSAGE_DELIVERY_MS: 15000,
  /** Wait for participant discovery */
  PARTICIPANT_DISCOVERY_MS: 12000,
  /** Wait for WebRTC reconnection */
  WEBRTC_RECONNECT_MS: 10000,
};

/** Standard timeouts for normal operations */
export const STANDARD_TIMEOUTS = {
  ELEMENT_VISIBLE: 10000,
  MESSAGE_DELIVERY: 10000,
  INVITATION_RECEIVE: 15000,
  GROUP_JOIN: 10000,
  PAGE_LOAD: 15000,
};

/**
 * Retry an async operation with exponential backoff
 */
export async function withRetry<T>(
  operation: () => Promise<T>,
  operationName: string,
  config: Partial<RetryConfig> = {},
): Promise<T> {
  const { maxAttempts, delayMs, backoffMultiplier } = { ...DEFAULT_RETRY_CONFIG, ...config };

  let lastError: Error | null = null;
  let currentDelay = delayMs;

  for (let attempt = 1; attempt <= maxAttempts; attempt++) {
    try {
      return await operation();
    } catch (error) {
      lastError = error as Error;
      console.log(`  ⟳ ${operationName}: attempt ${attempt}/${maxAttempts} failed`);

      if (attempt < maxAttempts) {
        await new Promise((r) => setTimeout(r, currentDelay));
        currentDelay *= backoffMultiplier;
      }
    }
  }

  throw new Error(`${operationName} failed after ${maxAttempts} attempts: ${lastError?.message}`);
}

/**
 * Wait for hub failover to complete after a disconnect
 * Detects when group messages can be sent/received again
 */
export async function waitForHubRecovery(page: Page, groupName: string): Promise<boolean> {
  console.log(`  ⏳ Waiting for hub recovery (${groupName})...`);

  try {
    // Wait for the group to be visible and selectable
    await page.waitForSelector(`.group-item:has-text("${groupName}")`, {
      timeout: POST_DISCONNECT_TIMEOUTS.HUB_RECOVERY_MS,
    });

    // Select the group to trigger state sync
    await page.click(`.group-item:has-text("${groupName}")`);

    // Wait a bit for any pending state updates
    await page.waitForTimeout(2000);

    // Check if we can see messages container (indicates group is functional)
    const messagesVisible = await page.locator('#messages').isVisible();

    if (messagesVisible) {
      console.log(`  ✓ Hub recovery complete for ${groupName}`);
      return true;
    }
    return false;
  } catch (error) {
    console.log(`  ⚠ Hub recovery timeout for ${groupName}`);
    return false;
  }
}

/**
 * Wait for WebRTC connections to be re-established after reconnect
 */
export async function waitForConnectionsReady(page: Page, expectedPeers: number): Promise<boolean> {
  console.log(`  ⏳ Waiting for ${expectedPeers} peer connections...`);

  const startTime = Date.now();
  const timeout = POST_DISCONNECT_TIMEOUTS.WEBRTC_RECONNECT_MS;

  while (Date.now() - startTime < timeout) {
    const onlinePeers = await page.locator('.participant:not(.offline)').count();

    if (onlinePeers >= expectedPeers) {
      console.log(`  ✓ ${onlinePeers} peers connected`);
      return true;
    }

    await page.waitForTimeout(500);
  }

  const finalCount = await page.locator('.participant:not(.offline)').count();
  console.log(`  ⚠ Only ${finalCount}/${expectedPeers} peers connected after timeout`);
  return false;
}

/**
 * Send a message with retry on failure
 */
export async function sendMessageWithRetry(
  senderPage: Page,
  recipientSelector: string,
  message: string,
  config: Partial<RetryConfig> = {},
): Promise<boolean> {
  return withRetry(
    async () => {
      await senderPage.click(recipientSelector);
      await senderPage.fill('#message-input', message);
      await senderPage.click('#send-btn');

      // Wait for message to appear in sender's view (confirmation)
      await senderPage.waitForSelector(`.message.sent:has-text("${message}")`, {
        timeout: 5000,
      });

      return true;
    },
    `Send message "${message.slice(0, 20)}..."`,
    config,
  );
}

/**
 * Wait for a message to be received
 */
export async function waitForMessageReceived(
  receiverPage: Page,
  senderName: string,
  message: string,
  timeout: number = STANDARD_TIMEOUTS.MESSAGE_DELIVERY,
): Promise<boolean> {
  try {
    // First ensure we're viewing the right conversation
    await receiverPage.waitForSelector(`.participant:has-text("${senderName}")`, { timeout: 5000 });
    await receiverPage.click(`.participant:has-text("${senderName}")`);

    // Wait for the message
    await receiverPage.waitForSelector(`.message:has-text("${message}")`, { timeout });
    return true;
  } catch {
    return false;
  }
}

/**
 * Verify group state is synchronized across multiple pages
 */
export async function verifyGroupSync(pages: Page[], groupName: string): Promise<boolean> {
  const results = await Promise.all(
    pages.map(async (page) => {
      const hasGroup = await page.locator(`.group-item:has-text("${groupName}")`).isVisible();
      return hasGroup;
    }),
  );

  const allSynced = results.every(Boolean);
  console.log(`  Group "${groupName}" sync: ${results.filter(Boolean).length}/${pages.length} pages`);
  return allSynced;
}

/**
 * Capture screenshot on failure for debugging
 */
export async function captureDebugInfo(page: Page, testName: string): Promise<void> {
  try {
    const timestamp = Date.now();
    await page.screenshot({
      path: `./scripts/e2e/reports/artifacts/debug-${testName}-${timestamp}.png`,
      fullPage: true,
    });
    console.log(`  📸 Debug screenshot saved: debug-${testName}-${timestamp}.png`);
  } catch {
    // Ignore screenshot errors
  }
}

/**
 * Wait for status bar to show a specific status
 * Useful for waiting for connection/reconnection events
 */
export async function waitForStatus(page: Page, statusContains: string, timeout = 10000): Promise<boolean> {
  try {
    await page.waitForFunction(
      (text) => {
        const statusBar = document.getElementById('status-bar');
        return statusBar?.textContent?.toLowerCase().includes(text.toLowerCase());
      },
      statusContains,
      { timeout },
    );
    return true;
  } catch {
    return false;
  }
}

/**
 * Robust reconnection with verification
 */
export async function reconnectWithVerification(page: Page, username: string, expectedPeers = 0): Promise<boolean> {
  console.log(`  ↻ Reconnecting ${username}...`);

  try {
    await page.reload();
    await page.waitForSelector('#username-input', { timeout: 10000 });
    await page.fill('#username-input', username);
    await page.click('#join-btn');
    await page.waitForSelector('#chat', { state: 'visible', timeout: 15000 });

    // Wait for connections if expected
    if (expectedPeers > 0) {
      await waitForConnectionsReady(page, expectedPeers);
    }

    // Additional stabilization time
    await page.waitForTimeout(3000);

    console.log(`  ✓ ${username} reconnected`);
    return true;
  } catch (error) {
    console.log(`  ✗ ${username} reconnection failed: ${(error as Error).message}`);
    return false;
  }
}
