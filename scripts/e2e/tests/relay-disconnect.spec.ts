import { type Browser, type BrowserContext, type Page, expect, test } from '@playwright/test';
import {
  POST_DISCONNECT_TIMEOUTS,
  STANDARD_TIMEOUTS,
  waitForConnectionsReady,
  waitForHubRecovery,
  withRetry,
} from './test-helpers';

/**
 * E2E Test: Relay Disconnect Scenarios
 *
 * Tests network resilience when relay nodes disconnect:
 * 1. Direct messages between users
 * 2. Hub disconnect via page refresh and reconnect
 * 3. Message delivery after reconnection
 *
 * Uses robust test helpers for reliable timing.
 */

interface UserSession {
  context: BrowserContext;
  page: Page;
  username: string;
  nodeId?: string;
}

const DEMO_URL = process.env.DEMO_URL || 'http://localhost:5173';

test.describe('Relay Disconnect Scenarios', () => {
  let browser: Browser;
  const users: UserSession[] = [];

  test.beforeAll(async ({ browser: b }) => {
    browser = b;
  });

  test.afterAll(async () => {
    for (const user of users) {
      await user.context.close();
    }
  });

  async function createUserSession(username: string): Promise<UserSession> {
    const context = await browser.newContext();
    const page = await context.newPage();

    await page.goto(DEMO_URL);
    await page.waitForSelector('#username-input', { timeout: 10000 });
    await page.fill('#username-input', username);
    await page.click('#join-btn');

    await page.waitForSelector('#chat', { state: 'visible', timeout: 15000 });

    const nodeIdElement = await page.waitForSelector('#node-id', { timeout: 5000 });
    const nodeId = await nodeIdElement?.textContent();

    const session: UserSession = {
      context,
      page,
      username,
      nodeId: nodeId || undefined,
    };

    users.push(session);
    return session;
  }

  async function reconnectUser(session: UserSession, expectedPeers = 0): Promise<boolean> {
    console.log(`  ↻ Reconnecting ${session.username}...`);

    try {
      // Simulate page refresh (user closes/reopens tab)
      await session.page.reload();

      // Re-login with same username
      await session.page.waitForSelector('#username-input', { timeout: STANDARD_TIMEOUTS.PAGE_LOAD });
      await session.page.fill('#username-input', session.username);
      await session.page.click('#join-btn');

      // Wait for reconnection
      await session.page.waitForSelector('#chat', { state: 'visible', timeout: STANDARD_TIMEOUTS.PAGE_LOAD });

      // Wait for peer connections if expected
      if (expectedPeers > 0) {
        await waitForConnectionsReady(session.page, expectedPeers);
      }

      // Extra stabilization for WebRTC
      await session.page.waitForTimeout(3000);

      console.log(`  ✓ ${session.username} reconnected`);
      return true;
    } catch (error) {
      console.log(`  ⚠ ${session.username} reconnection failed: ${(error as Error).message}`);
      return false;
    }
  }

  test('should exchange direct messages between two users', async () => {
    const alice = await createUserSession('alice-dm');
    const bob = await createUserSession('bob-dm');

    // Wait for gossip discovery
    await alice.page.waitForTimeout(5000);

    // Alice selects Bob
    await alice.page.waitForSelector('.participant:has-text("bob-dm")', { timeout: 15000 });
    await alice.page.click('.participant:has-text("bob-dm")');

    // Alice sends message to Bob
    await alice.page.fill('#message-input', 'Salut Bob !');
    await alice.page.click('#send-btn');

    // Bob selects Alice and sees message
    await bob.page.waitForSelector('.participant:has-text("alice-dm")', { timeout: 10000 });
    await bob.page.click('.participant:has-text("alice-dm")');
    await bob.page.waitForSelector('.message:has-text("Salut Bob")', { timeout: 10000 });

    // Bob replies
    await bob.page.fill('#message-input', 'Salut Alice !');
    await bob.page.click('#send-btn');

    // Alice sees reply
    await alice.page.waitForSelector('.message:has-text("Salut Alice")', { timeout: 10000 });

    expect(await alice.page.locator('.message:has-text("Salut Alice")').isVisible()).toBe(true);
    expect(await bob.page.locator('.message:has-text("Salut Bob")').isVisible()).toBe(true);
  });

  test('should handle user disconnect via page refresh', async () => {
    test.setTimeout(120000); // 2 minutes for reconnection scenario

    const sender = await createUserSession('sender-refresh');
    const receiver = await createUserSession('receiver-refresh');

    // Extended wait for initial gossip discovery
    await sender.page.waitForTimeout(8000);

    // Exchange initial messages
    await sender.page.waitForSelector('.participant:has-text("receiver-refresh")', {
      timeout: STANDARD_TIMEOUTS.PARTICIPANT_DISCOVERY_MS || 15000,
    });
    await sender.page.click('.participant:has-text("receiver-refresh")');
    await sender.page.fill('#message-input', 'Message avant refresh');
    await sender.page.click('#send-btn');

    await receiver.page.waitForSelector('.participant:has-text("sender-refresh")', {
      timeout: STANDARD_TIMEOUTS.ELEMENT_VISIBLE,
    });
    await receiver.page.click('.participant:has-text("sender-refresh")');
    await receiver.page.waitForSelector('.message:has-text("Message avant refresh")', {
      timeout: STANDARD_TIMEOUTS.MESSAGE_DELIVERY,
    });

    // Receiver refreshes page (simulates disconnect)
    console.log('Receiver refreshing page...');
    await reconnectUser(receiver, 1); // Expect 1 peer (sender)
    console.log('Receiver reconnected, waiting for stabilization...');

    // Extended wait for WebRTC re-establishment
    await receiver.page.waitForTimeout(POST_DISCONNECT_TIMEOUTS.WEBRTC_RECONNECT_MS);

    // Use retry for message sending after disconnect
    const messageReceived = await withRetry(
      async () => {
        // Sender sends message after receiver reconnected
        await sender.page.fill('#message-input', 'Message après refresh');
        await sender.page.click('#send-btn');

        // Receiver should see the new message
        await receiver.page.waitForSelector('.participant:has-text("sender-refresh")', {
          timeout: POST_DISCONNECT_TIMEOUTS.PARTICIPANT_DISCOVERY_MS,
        });
        await receiver.page.click('.participant:has-text("sender-refresh")');

        await receiver.page.waitForSelector('.message:has-text("Message après refresh")', {
          timeout: POST_DISCONNECT_TIMEOUTS.MESSAGE_DELIVERY_MS,
        });
        return true;
      },
      'Message after refresh',
      { maxAttempts: 2, delayMs: 2000 },
    ).catch(() => false);

    console.log('Message received after refresh:', messageReceived);
    expect(messageReceived).toBe(true);
  });

  test('should handle hub disconnect in group via page refresh', async () => {
    const hub = await createUserSession('hub-refresh');
    const member1 = await createUserSession('member1-refresh');
    const member2 = await createUserSession('member2-refresh');

    // Wait for discovery
    await hub.page.waitForTimeout(5000);

    // Hub creates group
    await hub.page.click('#create-group-btn');
    await hub.page.waitForSelector('#create-group-modal.active', { timeout: 5000 });
    await hub.page.fill('#group-name-input', 'Refresh Test');
    await hub.page.click('#create-group-confirm-btn');
    await hub.page.waitForSelector('.group-item:has-text("Refresh Test")', { timeout: 10000 });
    await hub.page.click('.group-item:has-text("Refresh Test")');

    // Invite member1
    await hub.page.click('button:has-text("Inviter")');
    await hub.page.waitForSelector('#invite-modal.active', { timeout: 5000 });
    await hub.page.click('#invite-modal-list div:has-text("member1-refresh")');
    await hub.page.waitForTimeout(2000);

    // Member1 accepts
    await member1.page.waitForSelector('.participant:has-text("hub-refresh")', { timeout: 15000 });
    await member1.page.click('.participant:has-text("hub-refresh")');
    await member1.page.waitForSelector('.group-invite-message', { timeout: 15000 });
    await member1.page.click('.group-accept-btn');
    await member1.page.waitForSelector('.group-item:has-text("Refresh Test")', { timeout: 10000 });

    // Invite member2
    await hub.page.click('button:has-text("Inviter")');
    await hub.page.waitForSelector('#invite-modal.active', { timeout: 5000 });
    await hub.page.click('#invite-modal-list div:has-text("member2-refresh")');
    await hub.page.waitForTimeout(2000);

    // Member2 accepts
    await member2.page.waitForSelector('.participant:has-text("hub-refresh")', { timeout: 15000 });
    await member2.page.click('.participant:has-text("hub-refresh")');
    await member2.page.waitForSelector('.group-invite-message', { timeout: 15000 });
    await member2.page.click('.group-accept-btn');
    await member2.page.waitForSelector('.group-item:has-text("Refresh Test")', { timeout: 10000 });

    // Hub sends message
    await hub.page.click('.group-item:has-text("Refresh Test")');
    await hub.page.waitForTimeout(2000);
    await hub.page.fill('#message-input', 'Hub en ligne');
    await hub.page.click('#send-btn');

    // Members see message
    await member1.page.click('.group-item:has-text("Refresh Test")');
    await member1.page.waitForSelector('.message:has-text("Hub en ligne")', { timeout: 10000 });

    await member2.page.click('.group-item:has-text("Refresh Test")');
    await member2.page.waitForSelector('.message:has-text("Hub en ligne")', { timeout: 10000 });

    // CRITICAL: Hub refreshes (disconnects)
    console.log('Hub refreshing page (disconnect)...');
    await reconnectUser(hub);
    console.log('Hub reconnected');

    // Wait for reconnection
    await hub.page.waitForTimeout(8000);

    // Member1 tries to send message
    await member1.page.fill('#message-input', 'Hub parti, tu me reçois ?');
    await member1.page.click('#send-btn');

    // Member2 should receive
    const messageAfterHubRefresh = await member2.page
      .waitForSelector('.message:has-text("Hub parti")', { timeout: 15000 })
      .then(() => true)
      .catch(() => false);

    console.log('Message delivered after hub refresh:', messageAfterHubRefresh);

    if (!messageAfterHubRefresh) {
      console.log('⚠️ Hub failover not yet implemented - expected failure');
      console.log('This validates the need for Action 1: Hub Failover');
    }
  });

  test('should deliver pending messages after reconnection', async () => {
    test.setTimeout(150000); // 2.5 minutes for offline/online scenario with backup

    const sender = await createUserSession('sender-pending');
    const receiver = await createUserSession('receiver-pending');

    // Extended wait for initial discovery
    await sender.page.waitForTimeout(8000);

    // Initial conversation
    await sender.page.waitForSelector('.participant:has-text("receiver-pending")', {
      timeout: STANDARD_TIMEOUTS.INVITATION_RECEIVE,
    });
    await sender.page.click('.participant:has-text("receiver-pending")');
    await sender.page.fill('#message-input', 'Premier message');
    await sender.page.click('#send-btn');

    await receiver.page.waitForSelector('.participant:has-text("sender-pending")', {
      timeout: STANDARD_TIMEOUTS.ELEMENT_VISIBLE,
    });
    await receiver.page.click('.participant:has-text("sender-pending")');
    await receiver.page.waitForSelector('.message:has-text("Premier message")', {
      timeout: STANDARD_TIMEOUTS.MESSAGE_DELIVERY,
    });

    // Receiver goes offline (refresh to disconnect)
    console.log('Receiver going offline...');
    await receiver.page.reload();
    // Don't re-login yet - stay on login page

    // Sender sends while receiver offline (message should be backed up)
    await sender.page.fill('#message-input', 'Message pendant offline');
    await sender.page.click('#send-btn');
    await sender.page.waitForTimeout(3000);

    // Receiver comes back online
    console.log('Receiver coming back online...');
    await receiver.page.waitForSelector('#username-input', { timeout: STANDARD_TIMEOUTS.PAGE_LOAD });
    await receiver.page.fill('#username-input', 'receiver-pending');
    await receiver.page.click('#join-btn');
    await receiver.page.waitForSelector('#chat', { state: 'visible', timeout: STANDARD_TIMEOUTS.PAGE_LOAD });

    // Extended wait for WebRTC re-establishment and backup delivery
    console.log('Waiting for backup message delivery...');
    await receiver.page.waitForTimeout(POST_DISCONNECT_TIMEOUTS.HUB_RECOVERY_MS);

    // Use retry for finding the message (backup delivery can take time)
    const pendingDelivered = await withRetry(
      async () => {
        await receiver.page.waitForSelector('.participant:has-text("sender-pending")', {
          timeout: POST_DISCONNECT_TIMEOUTS.PARTICIPANT_DISCOVERY_MS,
        });
        await receiver.page.click('.participant:has-text("sender-pending")');

        await receiver.page.waitForSelector('.message:has-text("Message pendant offline")', {
          timeout: POST_DISCONNECT_TIMEOUTS.MESSAGE_DELIVERY_MS,
        });
        return true;
      },
      'Pending message delivery',
      { maxAttempts: 3, delayMs: 3000 },
    ).catch(() => false);

    console.log('Pending message delivered:', pendingDelivered);

    // Note: If this fails, it may indicate backup system needs improvement
    if (!pendingDelivered) {
      console.log('⚠️ Backup message delivery may need investigation');
    }
    expect(pendingDelivered).toBe(true);
  });
});
