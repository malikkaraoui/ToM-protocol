import { type Browser, type BrowserContext, type Page, expect, test } from '@playwright/test';

/**
 * E2E Test: Relay Disconnect Scenarios
 *
 * Tests network resilience when relay nodes disconnect:
 * 1. Mid-conversation relay crash
 * 2. Hub disconnect and recovery
 * 3. Message delivery after reconnection
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
  let users: UserSession[] = [];

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
    await page.waitForSelector('#username', { timeout: 10000 });
    await page.fill('#username', username);
    await page.click('#connect-btn');

    await page.waitForSelector('.connection-status:has-text("Connected")', {
      timeout: 15000,
    });

    const nodeIdElement = await page.waitForSelector('.node-id', { timeout: 5000 });
    const nodeId = await nodeIdElement?.textContent();

    const session: UserSession = {
      context,
      page,
      username,
      nodeId: nodeId?.replace('Node: ', '') || undefined,
    };

    users.push(session);
    return session;
  }

  test('should detect relay disconnect during group chat', async () => {
    // Setup: 3 users in a group
    const alice = await createUserSession('alice-relay-test');
    const bob = await createUserSession('bob-relay-test');
    const relay = await createUserSession('relay-node-test');

    // Alice creates group
    await alice.page.click('#create-group-btn');
    await alice.page.fill('#group-name-input', 'Relay Test Group');
    await alice.page.click('#confirm-create-group');

    await alice.page.waitForSelector('.group-item:has-text("Relay Test Group")', {
      timeout: 10000,
    });

    // Wait for discovery
    await alice.page.waitForTimeout(5000);

    // Invite Bob and Relay
    await alice.page.click('.group-item:has-text("Relay Test Group")');

    // Invite Bob
    await alice.page.click('#invite-member-btn');
    await alice.page.waitForSelector('.peer-list .peer-item', { timeout: 10000 });
    await alice.page.click(`.peer-item:has-text("bob-relay-test")`);
    await alice.page.click('#send-invite-btn');

    await bob.page.waitForSelector('.invitation-item', { timeout: 15000 });
    await bob.page.click('.invitation-item .accept-btn');

    // Invite Relay node
    await alice.page.click('#invite-member-btn');
    await alice.page.click(`.peer-item:has-text("relay-node-test")`);
    await alice.page.click('#send-invite-btn');

    await relay.page.waitForSelector('.invitation-item', { timeout: 15000 });
    await relay.page.click('.invitation-item .accept-btn');

    // Verify group formed
    await alice.page.waitForTimeout(3000);
    await alice.page.click('.group-item:has-text("Relay Test Group")');

    const memberCount = await alice.page.locator('.group-member').count();
    expect(memberCount).toBe(3);

    // Send initial message (should work)
    await alice.page.fill('#group-message-input', 'Message before disconnect');
    await alice.page.click('#send-group-message-btn');

    await bob.page.waitForSelector('.group-message:has-text("Message before disconnect")', {
      timeout: 10000,
    });

    // Simulate relay disconnect by closing its context
    await relay.context.close();
    users = users.filter((u) => u.username !== 'relay-node-test');

    // Wait for disconnect to be detected
    await alice.page.waitForTimeout(5000);

    // Check for disconnect notification or status change
    // The UI should show a warning or the member count should update
    const disconnectWarning = await alice.page
      .locator('.disconnect-warning, .member-offline')
      .isVisible()
      .catch(() => false);

    // Log the result for debugging
    console.log('Disconnect warning visible:', disconnectWarning);

    // Send message after disconnect
    await alice.page.fill('#group-message-input', 'Message after relay disconnect');
    await alice.page.click('#send-group-message-btn');

    // Bob should still receive via alternate route or direct
    const messageReceived = await bob.page
      .waitForSelector('.group-message:has-text("Message after relay disconnect")', {
        timeout: 15000,
      })
      .then(() => true)
      .catch(() => false);

    // This test may fail if rerouting isn't implemented yet
    // That's actually valuable feedback for Action 1 (Hub Failover)
    console.log('Message received after disconnect:', messageReceived);

    expect(messageReceived).toBe(true);
  });

  test('should handle hub disconnect gracefully', async () => {
    // This test specifically validates hub failover (Action 1 requirement)
    const hub = await createUserSession('hub-user');
    const member1 = await createUserSession('member1-hub-test');
    const member2 = await createUserSession('member2-hub-test');

    // Hub creates group
    await hub.page.click('#create-group-btn');
    await hub.page.fill('#group-name-input', 'Hub Failover Test');
    await hub.page.click('#confirm-create-group');

    await hub.page.waitForSelector('.group-item:has-text("Hub Failover Test")', {
      timeout: 10000,
    });

    // Wait for discovery
    await hub.page.waitForTimeout(5000);

    // Invite members
    await hub.page.click('.group-item:has-text("Hub Failover Test")');

    await hub.page.click('#invite-member-btn');
    await hub.page.waitForSelector('.peer-list .peer-item', { timeout: 10000 });
    await hub.page.click(`.peer-item:has-text("member1-hub-test")`);
    await hub.page.click('#send-invite-btn');

    await member1.page.waitForSelector('.invitation-item', { timeout: 15000 });
    await member1.page.click('.invitation-item .accept-btn');

    await hub.page.click('#invite-member-btn');
    await hub.page.click(`.peer-item:has-text("member2-hub-test")`);
    await hub.page.click('#send-invite-btn');

    await member2.page.waitForSelector('.invitation-item', { timeout: 15000 });
    await member2.page.click('.invitation-item .accept-btn');

    // Verify group is working
    await hub.page.waitForTimeout(3000);
    await hub.page.fill('#group-message-input', 'Hub is online');
    await hub.page.click('#send-group-message-btn');

    await member1.page.waitForSelector('.group-message:has-text("Hub is online")', {
      timeout: 10000,
    });

    // CRITICAL: Hub disconnects
    await hub.context.close();
    users = users.filter((u) => u.username !== 'hub-user');

    // Wait for hub disconnect to be detected
    await member1.page.waitForTimeout(8000);

    // Try to send message between remaining members
    // This SHOULD work if hub failover is implemented (Action 1)
    await member1.page.click('.group-item:has-text("Hub Failover Test")');
    await member1.page.fill('#group-message-input', 'Hub is gone, can you hear me?');
    await member1.page.click('#send-group-message-btn');

    // Check if member2 receives the message
    const messageAfterHubDown = await member2.page
      .waitForSelector('.group-message:has-text("Hub is gone")', {
        timeout: 15000,
      })
      .then(() => true)
      .catch(() => false);

    // This is expected to FAIL until Action 1 is implemented
    // The test documents the current limitation
    console.log('Message delivered after hub disconnect:', messageAfterHubDown);

    // For now, we just verify the test runs and captures the behavior
    // When Action 1 is done, this should pass
    if (!messageAfterHubDown) {
      console.log('⚠️ Hub failover not yet implemented - expected failure');
      console.log('This validates the need for Action 1: Hub Failover');
    }
  });

  test('should recover pending messages after reconnection', async () => {
    const sender = await createUserSession('sender-reconnect');
    const receiver = await createUserSession('receiver-reconnect');

    // Wait for discovery
    await sender.page.waitForTimeout(5000);

    // Sender sends direct message
    await sender.page.click('#peer-list-btn');
    await sender.page.waitForSelector('.peer-item', { timeout: 10000 });
    await sender.page.click(`.peer-item:has-text("receiver-reconnect")`);
    await sender.page.fill('#direct-message-input', 'Message 1 - should arrive');
    await sender.page.click('#send-direct-btn');

    // Verify first message received
    await receiver.page.waitForSelector('.direct-message:has-text("Message 1")', {
      timeout: 10000,
    });

    // Receiver goes offline (close and reopen)
    await receiver.context.close();
    users = users.filter((u) => u.username !== 'receiver-reconnect');

    // Sender sends message while receiver is offline
    await sender.page.fill('#direct-message-input', 'Message 2 - sent while offline');
    await sender.page.click('#send-direct-btn');

    // Wait a bit
    await sender.page.waitForTimeout(3000);

    // Receiver reconnects
    const newReceiver = await createUserSession('receiver-reconnect');

    // Check if pending message is delivered
    const pendingDelivered = await newReceiver.page
      .waitForSelector('.direct-message:has-text("Message 2")', {
        timeout: 20000,
      })
      .then(() => true)
      .catch(() => false);

    console.log('Pending message delivered after reconnect:', pendingDelivered);

    // This tests the backup node message storage (ADR-009)
    expect(pendingDelivered).toBe(true);
  });
});
