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

  test('should complete invite → accept → chat flow', async () => {
    const inviter = await createUserSession('inviter-flow');
    const invitee = await createUserSession('invitee-flow');

    await inviter.page.waitForTimeout(5000);

    // Create group
    await inviter.page.click('#create-group-btn');
    await inviter.page.waitForSelector('#create-group-modal.active', { timeout: 5000 });
    await inviter.page.fill('#group-name-input', 'Invite Flow Test');
    await inviter.page.click('#create-group-confirm-btn');
    await inviter.page.waitForSelector('.group-item:has-text("Invite Flow Test")', { timeout: 10000 });

    // Send invitation
    await inviter.page.click('.group-item:has-text("Invite Flow Test")');
    await inviter.page.click('button:has-text("Inviter")');
    await inviter.page.waitForSelector('#invite-modal.active', { timeout: 5000 });
    await inviter.page.click('#invite-modal-list div:has-text("invitee-flow")');
    await inviter.page.waitForTimeout(1000);

    // Invitee receives invitation
    await invitee.page.waitForSelector('.participant:has-text("inviter-flow")', { timeout: 10000 });
    await invitee.page.click('.participant:has-text("inviter-flow")');
    await invitee.page.waitForSelector('.group-invite-message', { timeout: 15000 });

    // Accept
    await invitee.page.click('.group-accept-btn');
    await invitee.page.waitForSelector('.group-item:has-text("Invite Flow Test")', { timeout: 10000 });

    // Verify inviter sees member count update
    await inviter.page.waitForTimeout(3000);
    await inviter.page.click('.group-item:has-text("Invite Flow Test")');
    await inviter.page.waitForSelector('#chat-header:has-text("2 membres")', { timeout: 5000 });

    // Test chat works
    await invitee.page.click('.group-item:has-text("Invite Flow Test")');
    await invitee.page.fill('#message-input', 'Thanks for the invite!');
    await invitee.page.click('#send-btn');

    await inviter.page.waitForSelector('.message:has-text("Thanks for the invite")', { timeout: 10000 });
  });

  test('should handle invite → decline correctly', async () => {
    const inviter = await createUserSession('inviter-decline');
    const decliner = await createUserSession('decliner-test');

    await inviter.page.waitForTimeout(5000);

    // Create group
    await inviter.page.click('#create-group-btn');
    await inviter.page.waitForSelector('#create-group-modal.active', { timeout: 5000 });
    await inviter.page.fill('#group-name-input', 'Decline Test Group');
    await inviter.page.click('#create-group-confirm-btn');
    await inviter.page.waitForSelector('.group-item:has-text("Decline Test Group")', { timeout: 10000 });

    // Send invitation
    await inviter.page.click('.group-item:has-text("Decline Test Group")');
    await inviter.page.click('button:has-text("Inviter")');
    await inviter.page.waitForSelector('#invite-modal.active', { timeout: 5000 });
    await inviter.page.click('#invite-modal-list div:has-text("decliner-test")');
    await inviter.page.waitForTimeout(1000);

    // Decliner receives and declines
    await decliner.page.waitForSelector('.participant:has-text("inviter-decline")', { timeout: 10000 });
    await decliner.page.click('.participant:has-text("inviter-decline")');
    await decliner.page.waitForSelector('.group-invite-message', { timeout: 15000 });
    await decliner.page.click('.group-decline-btn');

    await decliner.page.waitForTimeout(2000);

    // Verify decliner is NOT in the group
    const notInGroup = await decliner.page.locator('.group-item:has-text("Decline Test Group")').isHidden();
    expect(notInGroup).toBe(true);

    // Verify inviter still has group with only self
    await inviter.page.click('.group-item:has-text("Decline Test Group")');
    await inviter.page.waitForSelector('#chat-header:has-text("1 membre")', { timeout: 5000 });
  });

  test('should handle multiple pending invitations', async () => {
    const inviter = await createUserSession('multi-inviter');
    const recipient = await createUserSession('multi-recipient');

    await inviter.page.waitForTimeout(5000);

    const groupNames = ['Group Alpha', 'Group Beta', 'Group Gamma'];

    // Create 3 groups
    for (const name of groupNames) {
      await inviter.page.click('#create-group-btn');
      await inviter.page.waitForSelector('#create-group-modal.active', { timeout: 5000 });
      await inviter.page.fill('#group-name-input', name);
      await inviter.page.click('#create-group-confirm-btn');
      await inviter.page.waitForSelector(`.group-item:has-text("${name}")`, { timeout: 10000 });
    }

    // Send invitations to all 3 groups
    for (const name of groupNames) {
      await inviter.page.click(`.group-item:has-text("${name}")`);
      await inviter.page.click('button:has-text("Inviter")');
      await inviter.page.waitForSelector('#invite-modal.active', { timeout: 5000 });
      await inviter.page.click('#invite-modal-list div:has-text("multi-recipient")');
      await inviter.page.waitForTimeout(500);
    }

    // Recipient should see invites in chat with inviter
    await recipient.page.waitForSelector('.participant:has-text("multi-inviter")', { timeout: 10000 });
    await recipient.page.click('.participant:has-text("multi-inviter")');
    await recipient.page.waitForTimeout(3000);

    const invitationCount = await recipient.page.locator('.group-invite-message').count();
    console.log('Pending invitations count:', invitationCount);

    expect(invitationCount).toBeGreaterThanOrEqual(3);

    // Accept all invitations
    const acceptButtons = recipient.page.locator('.group-accept-btn');
    const count = await acceptButtons.count();
    for (let i = 0; i < count; i++) {
      await acceptButtons.first().click();
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
    await inviter.page.waitForSelector('#create-group-modal.active', { timeout: 5000 });
    await inviter.page.fill('#group-name-input', 'Status Test Group');
    await inviter.page.click('#create-group-confirm-btn');
    await inviter.page.waitForSelector('.group-item:has-text("Status Test Group")', { timeout: 10000 });

    // Send invitation
    await inviter.page.click('.group-item:has-text("Status Test Group")');
    await inviter.page.click('button:has-text("Inviter")');
    await inviter.page.waitForSelector('#invite-modal.active', { timeout: 5000 });
    await inviter.page.click('#invite-modal-list div:has-text("status-invitee")');
    await inviter.page.waitForTimeout(1000);

    // Check status bar for feedback
    const statusText = await inviter.page.locator('#status-bar').textContent();
    console.log('Status after invite:', statusText);

    // Invitee accepts
    await invitee.page.waitForSelector('.participant:has-text("status-inviter")', { timeout: 10000 });
    await invitee.page.click('.participant:has-text("status-inviter")');
    await invitee.page.waitForSelector('.group-invite-message', { timeout: 15000 });
    await invitee.page.click('.group-accept-btn');

    // Check member count update
    await inviter.page.waitForTimeout(3000);
    await inviter.page.click('.group-item:has-text("Status Test Group")');
    await inviter.page.waitForSelector('#chat-header:has-text("2 membres")', { timeout: 10000 });
  });

  test('should handle ACK delivery for invitations', async () => {
    const inviter = await createUserSession('ack-inviter');
    const invitee = await createUserSession('ack-invitee');

    await inviter.page.waitForTimeout(5000);

    // Create group
    await inviter.page.click('#create-group-btn');
    await inviter.page.waitForSelector('#create-group-modal.active', { timeout: 5000 });
    await inviter.page.fill('#group-name-input', 'ACK Test Group');
    await inviter.page.click('#create-group-confirm-btn');
    await inviter.page.waitForSelector('.group-item:has-text("ACK Test Group")', { timeout: 10000 });

    // Send invitation
    await inviter.page.click('.group-item:has-text("ACK Test Group")');
    await inviter.page.click('button:has-text("Inviter")');
    await inviter.page.waitForSelector('#invite-modal.active', { timeout: 5000 });
    await inviter.page.click('#invite-modal-list div:has-text("ack-invitee")');
    await inviter.page.waitForTimeout(1000);

    // Invitee accepts
    await invitee.page.waitForSelector('.participant:has-text("ack-inviter")', { timeout: 10000 });
    await invitee.page.click('.participant:has-text("ack-inviter")');
    await invitee.page.waitForSelector('.group-invite-message', { timeout: 15000 });
    await invitee.page.click('.group-accept-btn');

    // Verify invitee is in group
    await invitee.page.waitForSelector('.group-item:has-text("ACK Test Group")', { timeout: 10000 });

    // Critical: Inviter should see member count update (ACK delivered)
    await inviter.page.waitForTimeout(5000);
    await inviter.page.click('.group-item:has-text("ACK Test Group")');

    const headerText = await inviter.page.locator('#chat-header').textContent();
    console.log('Header after ACK:', headerText);

    const memberCount = headerText?.includes('2 membres');

    if (!memberCount) {
      console.log('⚠️ ACK not delivered - inviter does not see new member');
      console.log('This validates the need for Action 2: Robust Invitations');
    }

    expect(memberCount).toBe(true);
  });
});
