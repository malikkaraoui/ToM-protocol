import { type Browser, type BrowserContext, type Page, expect, test } from '@playwright/test';

/**
 * E2E Test: Group Creation with 3 Users
 *
 * Scenario:
 * 1. User A connects and creates a group
 * 2. User A invites User B and User C
 * 3. Users B and C accept invitations
 * 4. All 3 users can see each other in the group
 * 5. User A sends a message, B and C receive it
 */

interface UserSession {
  context: BrowserContext;
  page: Page;
  username: string;
  nodeId?: string;
}

const DEMO_URL = process.env.DEMO_URL || 'http://localhost:5173';

test.describe('Group Creation Flow', () => {
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

    // Wait for chat view to be visible (connection established)
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

  test('should create a group with 3 users', async () => {
    const alice = await createUserSession('alice-e2e');
    const bob = await createUserSession('bob-e2e');
    const charlie = await createUserSession('charlie-e2e');

    expect(alice.nodeId).toBeTruthy();
    expect(bob.nodeId).toBeTruthy();
    expect(charlie.nodeId).toBeTruthy();

    // Wait for gossip discovery
    await alice.page.waitForTimeout(3000);

    // Alice creates a group
    await alice.page.click('#create-group-btn');
    await alice.page.waitForSelector('#create-group-modal.active', { timeout: 5000 });
    await alice.page.fill('#group-name-input', 'Test Group E2E');
    await alice.page.click('#create-group-confirm-btn');

    await alice.page.waitForSelector('.group-item:has-text("Test Group E2E")', { timeout: 10000 });

    // Alice selects the group and invites Bob
    await alice.page.click('.group-item:has-text("Test Group E2E")');
    await alice.page.waitForSelector('#chat-header:has-text("Test Group E2E")', { timeout: 5000 });
    await alice.page.click('button:has-text("Inviter")');
    await alice.page.waitForSelector('#invite-modal.active', { timeout: 5000 });
    await alice.page.click('#invite-modal-list div:has-text("bob-e2e")');
    await alice.page.waitForTimeout(1000);

    // Bob sees Alice in participants, clicks to view chat, and accepts invite
    await bob.page.waitForSelector('.participant:has-text("alice-e2e")', { timeout: 10000 });
    await bob.page.click('.participant:has-text("alice-e2e")');
    await bob.page.waitForSelector('.group-invite-message', { timeout: 15000 });
    await bob.page.click('.group-accept-btn');
    await bob.page.waitForSelector('.group-item:has-text("Test Group E2E")', { timeout: 10000 });

    // Alice invites Charlie
    await alice.page.click('.group-item:has-text("Test Group E2E")');
    await alice.page.click('button:has-text("Inviter")');
    await alice.page.waitForSelector('#invite-modal.active', { timeout: 5000 });
    await alice.page.click('#invite-modal-list div:has-text("charlie-e2e")');
    await alice.page.waitForTimeout(1000);

    // Charlie accepts
    await charlie.page.waitForSelector('.participant:has-text("alice-e2e")', { timeout: 10000 });
    await charlie.page.click('.participant:has-text("alice-e2e")');
    await charlie.page.waitForSelector('.group-invite-message', { timeout: 15000 });
    await charlie.page.click('.group-accept-btn');
    await charlie.page.waitForSelector('.group-item:has-text("Test Group E2E")', { timeout: 10000 });

    // All select the group
    await alice.page.click('.group-item:has-text("Test Group E2E")');
    await bob.page.click('.group-item:has-text("Test Group E2E")');
    await charlie.page.click('.group-item:has-text("Test Group E2E")');

    // Verify 3 members
    await alice.page.waitForSelector('#chat-header:has-text("3 membres")', { timeout: 5000 });

    // Alice sends a message
    await alice.page.fill('#message-input', 'Hello from Alice!');
    await alice.page.click('#send-btn');

    // Bob and Charlie receive it
    await bob.page.waitForSelector('.message:has-text("Hello from Alice!")', { timeout: 10000 });
    await charlie.page.waitForSelector('.message:has-text("Hello from Alice!")', { timeout: 10000 });

    expect(await bob.page.locator('.message:has-text("Hello from Alice!")').isVisible()).toBe(true);
    expect(await charlie.page.locator('.message:has-text("Hello from Alice!")').isVisible()).toBe(true);
  });

  test('should handle rapid group member additions', async () => {
    const hub = await createUserSession('hub-e2e');

    await hub.page.click('#create-group-btn');
    await hub.page.waitForSelector('#create-group-modal.active', { timeout: 5000 });
    await hub.page.fill('#group-name-input', 'Rapid Test Group');
    await hub.page.click('#create-group-confirm-btn');
    await hub.page.waitForSelector('.group-item:has-text("Rapid Test Group")', { timeout: 10000 });

    const members: UserSession[] = [];
    for (let i = 1; i <= 3; i++) {
      members.push(await createUserSession(`member${i}-e2e`));
    }

    await hub.page.waitForTimeout(5000);
    await hub.page.click('.group-item:has-text("Rapid Test Group")');

    for (const member of members) {
      await hub.page.click('button:has-text("Inviter")');
      await hub.page.waitForSelector('#invite-modal.active', { timeout: 5000 });
      await hub.page.click(`#invite-modal-list div:has-text("${member.username}")`);
      await hub.page.waitForTimeout(500);
    }

    for (const member of members) {
      await member.page.waitForSelector('.participant:has-text("hub-e2e")', { timeout: 10000 });
      await member.page.click('.participant:has-text("hub-e2e")');
      await member.page.waitForSelector('.group-invite-message', { timeout: 15000 });
      await member.page.click('.group-accept-btn');
    }

    await hub.page.waitForTimeout(3000);
    await hub.page.click('.group-item:has-text("Rapid Test Group")');
    await hub.page.waitForSelector('#chat-header:has-text("4 membres")', { timeout: 10000 });
  });
});
