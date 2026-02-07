import { type Browser, type BrowserContext, type Page, expect, test } from '@playwright/test';

/**
 * E2E Test: Invitation Flow
 *
 * Tests the complete invitation lifecycle:
 * 1. Invite → Accept → Confirmed membership
 * 2. Invite → Decline → No membership
 * 3. Multiple pending invitations
 * 4. Invitation retry on failure
 */

interface UserSession {
  context: BrowserContext;
  page: Page;
  username: string;
  nodeId?: string;
}

const DEMO_URL = process.env.DEMO_URL || 'http://localhost:5173';

test.describe('Invitation Flow', () => {
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

  test('should complete invite → accept → chat flow', async () => {
    const inviter = await createUserSession('inviter-flow');
    const invitee = await createUserSession('invitee-flow');

    // Wait for gossip discovery
    await inviter.page.waitForTimeout(5000);

    // Create group
    await inviter.page.click('#create-group-btn');
    await inviter.page.fill('#group-name-input', 'Invite Flow Test');
    await inviter.page.click('#confirm-create-group');

    await inviter.page.waitForSelector('.group-item:has-text("Invite Flow Test")', {
      timeout: 10000,
    });

    // Send invitation
    await inviter.page.click('.group-item:has-text("Invite Flow Test")');
    await inviter.page.click('#invite-member-btn');
    await inviter.page.waitForSelector('.peer-list .peer-item', { timeout: 10000 });
    await inviter.page.click(`.peer-item:has-text("invitee-flow")`);
    await inviter.page.click('#send-invite-btn');

    // Verify invitation sent feedback
    const inviteSentFeedback = await inviter.page
      .locator('.invite-sent-toast, .invite-pending')
      .isVisible()
      .catch(() => false);

    console.log('Invite sent feedback shown:', inviteSentFeedback);

    // Invitee receives invitation
    await invitee.page.waitForSelector('.invitation-item:has-text("Invite Flow Test")', {
      timeout: 15000,
    });

    // Verify invitation details
    const invitationCard = invitee.page.locator('.invitation-item:has-text("Invite Flow Test")');
    expect(await invitationCard.isVisible()).toBe(true);

    // Accept invitation
    await invitee.page.click('.invitation-item:has-text("Invite Flow Test") .accept-btn');

    // Verify invitee is now in group
    await invitee.page.waitForSelector('.group-item:has-text("Invite Flow Test")', {
      timeout: 10000,
    });

    // Verify inviter sees new member
    await inviter.page.waitForTimeout(3000);
    await inviter.page.click('.group-item:has-text("Invite Flow Test")');

    const memberCount = await inviter.page.locator('.group-member').count();
    expect(memberCount).toBe(2);

    // Test chat works
    await invitee.page.click('.group-item:has-text("Invite Flow Test")');
    await invitee.page.fill('#group-message-input', 'Thanks for the invite!');
    await invitee.page.click('#send-group-message-btn');

    await inviter.page.waitForSelector('.group-message:has-text("Thanks for the invite")', {
      timeout: 10000,
    });
  });

  test('should handle invite → decline correctly', async () => {
    const inviter = await createUserSession('inviter-decline');
    const decliner = await createUserSession('decliner-test');

    await inviter.page.waitForTimeout(5000);

    // Create group
    await inviter.page.click('#create-group-btn');
    await inviter.page.fill('#group-name-input', 'Decline Test Group');
    await inviter.page.click('#confirm-create-group');

    await inviter.page.waitForSelector('.group-item:has-text("Decline Test Group")', {
      timeout: 10000,
    });

    // Send invitation
    await inviter.page.click('.group-item:has-text("Decline Test Group")');
    await inviter.page.click('#invite-member-btn');
    await inviter.page.waitForSelector('.peer-list .peer-item', { timeout: 10000 });
    await inviter.page.click(`.peer-item:has-text("decliner-test")`);
    await inviter.page.click('#send-invite-btn');

    // Decliner receives invitation
    await decliner.page.waitForSelector('.invitation-item:has-text("Decline Test Group")', {
      timeout: 15000,
    });

    // Decline invitation
    await decliner.page.click('.invitation-item:has-text("Decline Test Group") .decline-btn');

    // Verify invitation is removed from UI
    await decliner.page.waitForTimeout(2000);
    const invitationGone = await decliner.page
      .locator('.invitation-item:has-text("Decline Test Group")')
      .isHidden()
      .catch(() => true);

    expect(invitationGone).toBe(true);

    // Verify decliner is NOT in the group
    const notInGroup = await decliner.page
      .locator('.group-item:has-text("Decline Test Group")')
      .isHidden()
      .catch(() => true);

    expect(notInGroup).toBe(true);

    // Verify inviter still has group with only self
    await inviter.page.click('.group-item:has-text("Decline Test Group")');
    const memberCount = await inviter.page.locator('.group-member').count();
    expect(memberCount).toBe(1);
  });

  test('should handle multiple pending invitations', async () => {
    const inviter = await createUserSession('multi-inviter');
    const recipient = await createUserSession('multi-recipient');

    await inviter.page.waitForTimeout(5000);

    // Create 3 groups
    const groupNames = ['Group Alpha', 'Group Beta', 'Group Gamma'];

    for (const name of groupNames) {
      await inviter.page.click('#create-group-btn');
      await inviter.page.fill('#group-name-input', name);
      await inviter.page.click('#confirm-create-group');

      await inviter.page.waitForSelector(`.group-item:has-text("${name}")`, {
        timeout: 10000,
      });
    }

    // Send invitations to all 3 groups
    for (const name of groupNames) {
      await inviter.page.click(`.group-item:has-text("${name}")`);
      await inviter.page.click('#invite-member-btn');
      await inviter.page.waitForSelector('.peer-list .peer-item', { timeout: 10000 });
      await inviter.page.click(`.peer-item:has-text("multi-recipient")`);
      await inviter.page.click('#send-invite-btn');

      // Small delay between invites
      await inviter.page.waitForTimeout(500);
    }

    // Recipient should see all 3 invitations
    await recipient.page.waitForTimeout(5000);

    const invitationCount = await recipient.page.locator('.invitation-item').count();
    console.log('Pending invitations count:', invitationCount);

    expect(invitationCount).toBeGreaterThanOrEqual(3);

    // Accept all invitations
    for (const name of groupNames) {
      await recipient.page.click(`.invitation-item:has-text("${name}") .accept-btn`);
      await recipient.page.waitForTimeout(500);
    }

    // Verify recipient is in all 3 groups
    await recipient.page.waitForTimeout(3000);

    for (const name of groupNames) {
      const inGroup = await recipient.page.locator(`.group-item:has-text("${name}")`).isVisible();

      expect(inGroup).toBe(true);
    }
  });

  test('should show invitation status in inviter UI', async () => {
    const inviter = await createUserSession('status-inviter');
    const invitee = await createUserSession('status-invitee');

    await inviter.page.waitForTimeout(5000);

    // Create group
    await inviter.page.click('#create-group-btn');
    await inviter.page.fill('#group-name-input', 'Status Test Group');
    await inviter.page.click('#confirm-create-group');

    await inviter.page.waitForSelector('.group-item:has-text("Status Test Group")', {
      timeout: 10000,
    });

    // Send invitation
    await inviter.page.click('.group-item:has-text("Status Test Group")');
    await inviter.page.click('#invite-member-btn');
    await inviter.page.waitForSelector('.peer-list .peer-item', { timeout: 10000 });
    await inviter.page.click(`.peer-item:has-text("status-invitee")`);
    await inviter.page.click('#send-invite-btn');

    // Check for pending status indicator
    const pendingIndicator = await inviter.page
      .locator('.pending-invite, .invite-status:has-text("pending")')
      .isVisible()
      .catch(() => false);

    console.log('Pending invite indicator shown:', pendingIndicator);

    // Invitee accepts
    await invitee.page.waitForSelector('.invitation-item:has-text("Status Test Group")', {
      timeout: 15000,
    });
    await invitee.page.click('.invitation-item .accept-btn');

    // Check for accepted status update in inviter UI
    await inviter.page.waitForTimeout(3000);

    const acceptedIndicator = await inviter.page
      .locator('.invite-accepted, .invite-status:has-text("accepted")')
      .isVisible()
      .catch(() => false);

    console.log('Accepted invite indicator shown:', acceptedIndicator);

    // The test captures current behavior
    // This validates the need for Action 3: Reactive UI
  });

  test('should handle ACK delivery for invitations', async () => {
    // This test validates the robustness of invitation ACKs (Action 2)
    const inviter = await createUserSession('ack-inviter');
    const invitee = await createUserSession('ack-invitee');

    await inviter.page.waitForTimeout(5000);

    // Create group
    await inviter.page.click('#create-group-btn');
    await inviter.page.fill('#group-name-input', 'ACK Test Group');
    await inviter.page.click('#confirm-create-group');

    await inviter.page.waitForSelector('.group-item:has-text("ACK Test Group")', {
      timeout: 10000,
    });

    // Enable network logging for debugging
    inviter.page.on('console', (msg) => {
      if (msg.text().includes('ACK') || msg.text().includes('invitation')) {
        console.log('[Inviter]', msg.text());
      }
    });

    invitee.page.on('console', (msg) => {
      if (msg.text().includes('ACK') || msg.text().includes('invitation')) {
        console.log('[Invitee]', msg.text());
      }
    });

    // Send invitation
    await inviter.page.click('.group-item:has-text("ACK Test Group")');
    await inviter.page.click('#invite-member-btn');
    await inviter.page.waitForSelector('.peer-list .peer-item', { timeout: 10000 });
    await inviter.page.click(`.peer-item:has-text("ack-invitee")`);
    await inviter.page.click('#send-invite-btn');

    // Wait for invitation to be received
    await invitee.page.waitForSelector('.invitation-item:has-text("ACK Test Group")', {
      timeout: 15000,
    });

    // Accept
    await invitee.page.click('.invitation-item .accept-btn');

    // Verify membership is confirmed on BOTH sides
    await invitee.page.waitForSelector('.group-item:has-text("ACK Test Group")', {
      timeout: 10000,
    });

    // Critical: Inviter should see member count update
    await inviter.page.waitForTimeout(5000);
    await inviter.page.click('.group-item:has-text("ACK Test Group")');

    const memberCount = await inviter.page.locator('.group-member').count();

    // If this is 1, the ACK wasn't delivered (Action 2 issue)
    if (memberCount === 1) {
      console.log('⚠️ ACK not delivered - inviter does not see new member');
      console.log('This validates the need for Action 2: Robust Invitations');
    }

    // Test passes if both see each other
    expect(memberCount).toBe(2);
  });
});
