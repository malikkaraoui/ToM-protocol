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
const SIGNALING_URL = process.env.SIGNALING_URL || 'ws://localhost:3000';

test.describe('Group Creation Flow', () => {
  let browser: Browser;
  const users: UserSession[] = [];

  test.beforeAll(async ({ browser: b }) => {
    browser = b;
  });

  test.afterAll(async () => {
    // Clean up all user sessions
    for (const user of users) {
      await user.context.close();
    }
  });

  async function createUserSession(username: string): Promise<UserSession> {
    const context = await browser.newContext();
    const page = await context.newPage();

    // Navigate to demo
    await page.goto(DEMO_URL);

    // Wait for the app to load
    await page.waitForSelector('#username', { timeout: 10000 });

    // Enter username and connect
    await page.fill('#username', username);
    await page.click('#connect-btn');

    // Wait for connection
    await page.waitForSelector('.connection-status:has-text("Connected")', {
      timeout: 15000,
    });

    // Get the node ID from the UI
    const nodeIdElement = await page.waitForSelector('.node-id', {
      timeout: 5000,
    });
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

  test('should create a group with 3 users', async () => {
    // Step 1: Create 3 user sessions
    const alice = await createUserSession('alice-e2e');
    const bob = await createUserSession('bob-e2e');
    const charlie = await createUserSession('charlie-e2e');

    // Verify all users are connected
    expect(alice.nodeId).toBeTruthy();
    expect(bob.nodeId).toBeTruthy();
    expect(charlie.nodeId).toBeTruthy();

    // Step 2: Alice creates a group
    await alice.page.click('#create-group-btn');
    await alice.page.fill('#group-name-input', 'Test Group E2E');
    await alice.page.click('#confirm-create-group');

    // Wait for group to appear in Alice's list
    await alice.page.waitForSelector('.group-item:has-text("Test Group E2E")', {
      timeout: 10000,
    });

    // Step 3: Alice invites Bob
    await alice.page.click('.group-item:has-text("Test Group E2E")');
    await alice.page.click('#invite-member-btn');

    // Wait for peer list to populate (gossip discovery)
    await alice.page.waitForSelector('.peer-list .peer-item', {
      timeout: 15000,
    });

    // Select Bob from peer list
    await alice.page.click(`.peer-item:has-text("bob-e2e")`);
    await alice.page.click('#send-invite-btn');

    // Step 4: Bob receives and accepts invitation
    await bob.page.waitForSelector('.invitation-item:has-text("Test Group E2E")', {
      timeout: 15000,
    });
    await bob.page.click('.invitation-item:has-text("Test Group E2E") .accept-btn');

    // Wait for Bob to join
    await bob.page.waitForSelector('.group-item:has-text("Test Group E2E")', {
      timeout: 10000,
    });

    // Step 5: Alice invites Charlie
    await alice.page.click('#invite-member-btn');
    await alice.page.click(`.peer-item:has-text("charlie-e2e")`);
    await alice.page.click('#send-invite-btn');

    // Step 6: Charlie accepts
    await charlie.page.waitForSelector('.invitation-item:has-text("Test Group E2E")', {
      timeout: 15000,
    });
    await charlie.page.click('.invitation-item:has-text("Test Group E2E") .accept-btn');

    // Wait for Charlie to join
    await charlie.page.waitForSelector('.group-item:has-text("Test Group E2E")', {
      timeout: 10000,
    });

    // Step 7: Verify all users see each other in the group
    // Select the group on all pages
    await alice.page.click('.group-item:has-text("Test Group E2E")');
    await bob.page.click('.group-item:has-text("Test Group E2E")');
    await charlie.page.click('.group-item:has-text("Test Group E2E")');

    // Check member count
    const aliceMembers = await alice.page.locator('.group-member').count();
    const bobMembers = await bob.page.locator('.group-member').count();
    const charlieMembers = await charlie.page.locator('.group-member').count();

    expect(aliceMembers).toBe(3);
    expect(bobMembers).toBe(3);
    expect(charlieMembers).toBe(3);

    // Step 8: Alice sends a message
    await alice.page.fill('#group-message-input', 'Hello from Alice!');
    await alice.page.click('#send-group-message-btn');

    // Step 9: Bob and Charlie receive the message
    await bob.page.waitForSelector('.group-message:has-text("Hello from Alice!")', {
      timeout: 10000,
    });
    await charlie.page.waitForSelector('.group-message:has-text("Hello from Alice!")', {
      timeout: 10000,
    });

    // Verify message appears on all screens
    const bobMessage = await bob.page.locator('.group-message:has-text("Hello from Alice!")').isVisible();
    const charlieMessage = await charlie.page.locator('.group-message:has-text("Hello from Alice!")').isVisible();

    expect(bobMessage).toBe(true);
    expect(charlieMessage).toBe(true);
  });

  test('should handle rapid group member additions', async () => {
    // Create hub user
    const hub = await createUserSession('hub-e2e');

    // Create group
    await hub.page.click('#create-group-btn');
    await hub.page.fill('#group-name-input', 'Rapid Test Group');
    await hub.page.click('#confirm-create-group');

    await hub.page.waitForSelector('.group-item:has-text("Rapid Test Group")', {
      timeout: 10000,
    });

    // Create 3 members rapidly
    const members: UserSession[] = [];
    for (let i = 1; i <= 3; i++) {
      members.push(await createUserSession(`member${i}-e2e`));
    }

    // Wait for gossip discovery
    await hub.page.waitForTimeout(5000);

    // Select group and invite all members rapidly
    await hub.page.click('.group-item:has-text("Rapid Test Group")');

    for (const member of members) {
      await hub.page.click('#invite-member-btn');
      await hub.page.waitForSelector('.peer-list .peer-item', { timeout: 10000 });
      await hub.page.click(`.peer-item:has-text("${member.username}")`);
      await hub.page.click('#send-invite-btn');
    }

    // All members accept
    for (const member of members) {
      await member.page.waitForSelector('.invitation-item:has-text("Rapid Test Group")', {
        timeout: 15000,
      });
      await member.page.click('.invitation-item:has-text("Rapid Test Group") .accept-btn');
    }

    // Verify all joined
    await hub.page.waitForTimeout(3000); // Allow state to sync

    await hub.page.click('.group-item:has-text("Rapid Test Group")');
    const memberCount = await hub.page.locator('.group-member').count();

    expect(memberCount).toBe(4); // hub + 3 members
  });
});
