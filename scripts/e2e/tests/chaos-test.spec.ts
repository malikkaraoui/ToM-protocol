import { type Browser, type BrowserContext, type Page, expect, test } from '@playwright/test';
import { metrics } from './metrics';

/**
 * Chaos E2E Tests - Aggressive and Unpredictable Scenarios
 *
 * Tests the protocol's resilience under adverse conditions:
 * - Random disconnections during operations
 * - Network throttling (slow 3G simulation)
 * - Rapid reconnection cycles
 * - Concurrent stress operations
 * - Hub failures during message delivery
 */

interface UserSession {
  context: BrowserContext;
  page: Page;
  username: string;
  nodeId?: string;
}

const DEMO_URL = process.env.DEMO_URL || 'http://localhost:5173';

// Chaos configuration
const CHAOS_CONFIG = {
  DISCONNECT_PROBABILITY: 0.15, // 15% chance of random disconnect
  RECONNECT_DELAY_MS: 1000,
  THROTTLE_LATENCY_MS: 500,
  THROTTLE_DOWNLOAD_KBPS: 50,
  THROTTLE_UPLOAD_KBPS: 25,
  MAX_CHAOS_EVENTS: 5,
};

test.describe('Chaos Tests - Network Resilience', () => {
  let browser: Browser;
  const sessions: Map<string, UserSession> = new Map();
  let chaosEventCount = 0;

  test.beforeAll(async ({ browser: b }) => {
    browser = b;
    metrics.reset();
  });

  test.afterAll(async () => {
    for (const [, session] of sessions) {
      await session.context.close();
    }
    console.log(`\n${metrics.generateReport()}`);
  });

  test.afterEach(async () => {
    metrics.endTest();
  });

  async function createUser(username: string): Promise<UserSession> {
    const context = await browser.newContext();
    const page = await context.newPage();

    await page.goto(DEMO_URL);
    await page.waitForSelector('#username-input', { timeout: 10000 });
    await page.fill('#username-input', username);
    await page.click('#join-btn');
    await page.waitForSelector('#chat', { state: 'visible', timeout: 15000 });

    const nodeIdElement = await page.waitForSelector('#node-id', { timeout: 5000 });
    const nodeId = await nodeIdElement?.textContent();

    const session: UserSession = { context, page, username, nodeId: nodeId || undefined };
    sessions.set(username, session);
    metrics.addUser(username);

    console.log(`✓ ${username} connecté (${nodeId?.slice(0, 8)}...)`);
    return session;
  }

  async function disconnectUser(username: string): Promise<void> {
    const session = sessions.get(username);
    if (!session) return;

    // Simulate disconnect by closing WebSocket connections
    await session.page.evaluate(() => {
      // Close any WebSocket connections
      if ((window as unknown as { ws?: WebSocket }).ws) {
        (window as unknown as { ws?: WebSocket }).ws?.close();
      }
    });
    console.log(`⚡ ${username} déconnecté (chaos)`);
  }

  async function reconnectUser(username: string): Promise<void> {
    const session = sessions.get(username);
    if (!session) return;

    await session.page.reload();
    await session.page.waitForSelector('#username-input', { timeout: 10000 });
    await session.page.fill('#username-input', username);
    await session.page.click('#join-btn');
    await session.page.waitForSelector('#chat', { state: 'visible', timeout: 15000 });

    console.log(`↻ ${username} reconnecté`);
  }

  async function applyNetworkThrottling(page: Page): Promise<void> {
    const client = await page.context().newCDPSession(page);
    await client.send('Network.emulateNetworkConditions', {
      offline: false,
      latency: CHAOS_CONFIG.THROTTLE_LATENCY_MS,
      downloadThroughput: (CHAOS_CONFIG.THROTTLE_DOWNLOAD_KBPS * 1024) / 8,
      uploadThroughput: (CHAOS_CONFIG.THROTTLE_UPLOAD_KBPS * 1024) / 8,
    });
    console.log(`🐢 Throttling appliqué (latence: ${CHAOS_CONFIG.THROTTLE_LATENCY_MS}ms)`);
  }

  async function removeNetworkThrottling(page: Page): Promise<void> {
    const client = await page.context().newCDPSession(page);
    await client.send('Network.emulateNetworkConditions', {
      offline: false,
      latency: 0,
      downloadThroughput: -1,
      uploadThroughput: -1,
    });
    console.log('🚀 Throttling retiré');
  }

  async function maybeInjectChaos(username: string): Promise<boolean> {
    if (chaosEventCount >= CHAOS_CONFIG.MAX_CHAOS_EVENTS) return false;
    if (Math.random() > CHAOS_CONFIG.DISCONNECT_PROBABILITY) return false;

    chaosEventCount++;
    console.log(`\n🔥 CHAOS EVENT #${chaosEventCount} for ${username}`);

    await disconnectUser(username);
    await new Promise((r) => setTimeout(r, CHAOS_CONFIG.RECONNECT_DELAY_MS));
    await reconnectUser(username);

    return true;
  }

  async function sendDM(from: string, to: string, message: string, withChaos = false): Promise<boolean> {
    const sender = sessions.get(from);
    const receiver = sessions.get(to);
    if (!sender || !receiver) return false;

    const msgId = metrics.recordMessageSent(from, to, 'direct');

    try {
      // Maybe inject chaos before sending
      if (withChaos) {
        await maybeInjectChaos(from);
      }

      await sender.page.waitForSelector(`.participant:has-text("${to}")`, { timeout: 10000 });
      await sender.page.click(`.participant:has-text("${to}")`);
      await sender.page.fill('#message-input', message);
      await sender.page.click('#send-btn');

      // Maybe inject chaos during delivery
      if (withChaos) {
        await maybeInjectChaos(to);
      }

      await receiver.page.waitForSelector(`.participant:has-text("${from}")`, { timeout: 10000 });
      await receiver.page.click(`.participant:has-text("${from}")`);

      const received = await receiver.page
        .waitForSelector(`.message:has-text("${message}")`, { timeout: 15000 })
        .then(() => true)
        .catch(() => false);

      if (received) {
        metrics.recordMessageReceived(msgId);
        console.log(`  ✓ ${from} → ${to}: "${message}"`);
        return true;
      }
      metrics.recordMessageFailed(msgId, 'Non reçu après chaos');
      console.log(`  ✗ ${from} → ${to}: "${message}" (non reçu)`);
      return false;
    } catch (e) {
      metrics.recordMessageFailed(msgId, (e as Error).message);
      console.log(`  ✗ ${from} → ${to}: "${message}" (erreur: ${(e as Error).message})`);
      return false;
    }
  }

  async function createGroup(creator: string, groupName: string): Promise<boolean> {
    const session = sessions.get(creator);
    if (!session) return false;

    try {
      await session.page.click('#create-group-btn');
      await session.page.waitForSelector('#create-group-modal.active', { timeout: 5000 });
      await session.page.fill('#group-name-input', groupName);
      await session.page.click('#create-group-confirm-btn');
      await session.page.waitForSelector(`.group-item:has-text("${groupName}")`, { timeout: 10000 });

      metrics.recordGroupCreated(groupName, groupName, creator);
      console.log(`  ✓ Groupe "${groupName}" créé par ${creator}`);
      return true;
    } catch {
      console.log(`  ✗ Création groupe "${groupName}" échouée`);
      return false;
    }
  }

  async function invite(inviter: string, invitee: string, groupName: string): Promise<string | null> {
    const session = sessions.get(inviter);
    if (!session) return null;

    const invId = metrics.recordInvitationSent(groupName, groupName, inviter, invitee);

    try {
      await session.page.click(`.group-item:has-text("${groupName}")`);
      await session.page.click('button:has-text("Inviter")');
      await session.page.waitForSelector('#invite-modal.active', { timeout: 5000 });

      const found = await session.page.locator(`#invite-modal-list div:has-text("${invitee}")`).isVisible();
      if (!found) {
        await session.page.click('#invite-modal-close-btn');
        metrics.recordInvitationFailed(invId, 'Contact non trouvé');
        return null;
      }

      await session.page.click(`#invite-modal-list div:has-text("${invitee}")`);
      console.log(`  → Invitation envoyée: ${inviter} → ${invitee} (${groupName})`);
      return invId;
    } catch (e) {
      metrics.recordInvitationFailed(invId, (e as Error).message);
      return null;
    }
  }

  async function acceptInvite(invitee: string, inviter: string, invId: string, groupName: string): Promise<boolean> {
    const session = sessions.get(invitee);
    if (!session) return false;

    try {
      await session.page.waitForSelector(`.participant:has-text("${inviter}")`, { timeout: 15000 });
      await session.page.click(`.participant:has-text("${inviter}")`);

      const inviteVisible = await session.page
        .waitForSelector('.group-invite-message', { timeout: 15000 })
        .then(() => true)
        .catch(() => false);

      if (!inviteVisible) {
        metrics.recordInvitationFailed(invId, 'Invitation non reçue');
        return false;
      }

      metrics.recordInvitationReceived(invId);
      await session.page.click('.group-accept-btn');

      const joined = await session.page
        .waitForSelector(`.group-item:has-text("${groupName}")`, { timeout: 10000 })
        .then(() => true)
        .catch(() => false);

      if (joined) {
        metrics.recordInvitationAccepted(invId);
        console.log(`  ✓ ${invitee} a rejoint "${groupName}"`);
        return true;
      }
      metrics.recordInvitationFailed(invId, 'Échec rejoindre groupe');
      return false;
    } catch (e) {
      metrics.recordInvitationFailed(invId, (e as Error).message);
      return false;
    }
  }

  async function sendGroupMsg(sender: string, groupName: string, message: string): Promise<boolean> {
    const session = sessions.get(sender);
    if (!session) return false;

    const msgId = metrics.recordMessageSent(sender, groupName, 'group', groupName);

    try {
      await session.page.click(`.group-item:has-text("${groupName}")`);
      await session.page.waitForTimeout(500);
      await session.page.fill('#message-input', message);
      await session.page.click('#send-btn');

      // Check at least one other user received it
      for (const [username, otherSession] of sessions) {
        if (username === sender) continue;

        const hasGroup = await otherSession.page.locator(`.group-item:has-text("${groupName}")`).isVisible();
        if (!hasGroup) continue;

        await otherSession.page.click(`.group-item:has-text("${groupName}")`);

        const received = await otherSession.page
          .waitForSelector(`.message:has-text("${message}")`, { timeout: 8000 })
          .then(() => true)
          .catch(() => false);

        if (received) {
          metrics.recordMessageReceived(msgId);
          console.log(`  ✓ ${sender} → groupe: "${message}"`);
          return true;
        }
      }

      metrics.recordMessageFailed(msgId, 'Aucun destinataire');
      return false;
    } catch (e) {
      metrics.recordMessageFailed(msgId, (e as Error).message);
      return false;
    }
  }

  test('Chaos 1: Déconnexions aléatoires pendant messages', async () => {
    metrics.startTest('Chaos 1: Random Disconnects');
    console.log('\n═══ CHAOS 1: Déconnexions aléatoires ═══\n');

    const alice = await createUser('alice');
    const bob = await createUser('bob');
    const charlie = await createUser('charlie');

    await alice.page.waitForTimeout(8000);

    console.log('\nMessages avec chaos (15% de déconnexion):');
    chaosEventCount = 0;

    const results = [];
    for (let i = 0; i < 10; i++) {
      results.push(await sendDM('alice', 'bob', `Chaos msg ${i + 1}`, true));
      results.push(await sendDM('bob', 'charlie', `Chaos msg ${i + 1}`, true));
    }

    const successCount = results.filter(Boolean).length;
    console.log(
      `\nChaos 1 Result: ${successCount}/${results.length} messages (${((successCount / results.length) * 100).toFixed(1)}%)`,
    );
    console.log(`Chaos events: ${chaosEventCount}`);
  });

  test('Chaos 2: Throttling réseau (slow 3G)', async () => {
    metrics.startTest('Chaos 2: Network Throttling');
    console.log('\n═══ CHAOS 2: Throttling réseau ═══\n');

    const alice = sessions.get('alice')!;
    const bob = sessions.get('bob')!;

    // Apply throttling to all users
    console.log('Application du throttling...');
    await applyNetworkThrottling(alice.page);
    await applyNetworkThrottling(bob.page);

    console.log('\nMessages sous throttling:');
    const results = [];
    for (let i = 0; i < 5; i++) {
      results.push(await sendDM('alice', 'bob', `Slow msg ${i + 1}`));
      results.push(await sendDM('bob', 'alice', `Slow reply ${i + 1}`));
    }

    // Remove throttling
    await removeNetworkThrottling(alice.page);
    await removeNetworkThrottling(bob.page);

    const successCount = results.filter(Boolean).length;
    console.log(`\nChaos 2 Result: ${successCount}/${results.length} messages sous throttling`);
  });

  test('Chaos 3: Reconnexions rapides cycliques', async () => {
    metrics.startTest('Chaos 3: Rapid Reconnects');
    console.log('\n═══ CHAOS 3: Reconnexions rapides ═══\n');

    const RECONNECT_CYCLES = 3;
    const MESSAGES_PER_CYCLE = 3;

    let totalSuccess = 0;
    let totalAttempts = 0;

    for (let cycle = 0; cycle < RECONNECT_CYCLES; cycle++) {
      console.log(`\n--- Cycle ${cycle + 1}/${RECONNECT_CYCLES} ---`);

      // Reconnect a random user
      const users = ['alice', 'bob', 'charlie'];
      const targetUser = users[Math.floor(Math.random() * users.length)];
      console.log(`Reconnexion de ${targetUser}...`);
      await reconnectUser(targetUser);
      await sessions.get(targetUser)?.page.waitForTimeout(3000);

      // Send messages immediately after reconnect
      console.log('Messages immédiats après reconnexion:');
      for (let i = 0; i < MESSAGES_PER_CYCLE; i++) {
        totalAttempts++;
        const success = await sendDM(
          targetUser,
          users[(users.indexOf(targetUser) + 1) % 3],
          `Post-reconnect ${cycle}-${i}`,
        );
        if (success) totalSuccess++;
      }
    }

    console.log(`\nChaos 3 Result: ${totalSuccess}/${totalAttempts} messages après reconnexions`);
  });

  test('Chaos 4: Hub disconnect pendant groupe actif', async () => {
    metrics.startTest('Chaos 4: Hub Disconnect');
    console.log('\n═══ CHAOS 4: Hub disconnect pendant activité ═══\n');

    // Create group with Alice as hub
    const groupCreated = await createGroup('alice', 'Chaos Group');
    if (!groupCreated) {
      console.log('Échec création groupe, skip test');
      return;
    }

    // Invite others
    const inv1 = await invite('alice', 'bob', 'Chaos Group');
    await sessions.get('alice')?.page.waitForTimeout(2000);
    if (inv1) await acceptInvite('bob', 'alice', inv1, 'Chaos Group');

    const inv2 = await invite('alice', 'charlie', 'Chaos Group');
    await sessions.get('alice')?.page.waitForTimeout(2000);
    if (inv2) await acceptInvite('charlie', 'alice', inv2, 'Chaos Group');

    await sessions.get('alice')?.page.waitForTimeout(3000);

    // Send initial messages
    console.log('\nMessages avant disconnect hub:');
    await sendGroupMsg('alice', 'Chaos Group', 'Pre-chaos 1');
    await sendGroupMsg('bob', 'Chaos Group', 'Pre-chaos 2');

    // Disconnect hub (Alice)
    console.log('\n⚡ DISCONNECT HUB (Alice)...');
    await reconnectUser('alice');
    await sessions.get('alice')?.page.waitForTimeout(5000);

    // Try to send messages while hub is reconnecting
    console.log('\nMessages pendant/après hub disconnect:');
    const results = [
      await sendGroupMsg('bob', 'Chaos Group', 'During hub chaos 1'),
      await sendGroupMsg('charlie', 'Chaos Group', 'During hub chaos 2'),
      await sendGroupMsg('alice', 'Chaos Group', 'Hub is back'),
    ];

    const successCount = results.filter(Boolean).length;
    console.log(`\nChaos 4 Result: ${successCount}/${results.length} messages groupe après hub disconnect`);
  });

  test('Chaos 5: Stress concurrent - tous envoient en même temps', async () => {
    metrics.startTest('Chaos 5: Concurrent Stress');
    console.log('\n═══ CHAOS 5: Stress concurrent ═══\n');

    const CONCURRENT_MESSAGES = 5;

    console.log(`Envoi concurrent de ${CONCURRENT_MESSAGES} messages par utilisateur...`);

    // All users send messages concurrently
    const promises: Promise<boolean>[] = [];

    for (let i = 0; i < CONCURRENT_MESSAGES; i++) {
      promises.push(sendDM('alice', 'bob', `Concurrent A→B ${i}`));
      promises.push(sendDM('bob', 'charlie', `Concurrent B→C ${i}`));
      promises.push(sendDM('charlie', 'alice', `Concurrent C→A ${i}`));
    }

    const results = await Promise.all(promises);
    const successCount = results.filter(Boolean).length;
    const total = promises.length;

    console.log(
      `\nChaos 5 Result: ${successCount}/${total} messages concurrent (${((successCount / total) * 100).toFixed(1)}%)`,
    );
  });

  test('Chaos 6: Disconnect pendant invitation', async () => {
    metrics.startTest('Chaos 6: Disconnect During Invite');
    console.log('\n═══ CHAOS 6: Disconnect pendant invitation ═══\n');

    // Create new group
    const groupCreated = await createGroup('bob', 'Invite Chaos');
    if (!groupCreated) {
      console.log('Échec création groupe, skip test');
      return;
    }

    // Start invitation
    console.log('Envoi invitation...');
    const inv = await invite('bob', 'alice', 'Invite Chaos');

    if (inv) {
      // Disconnect invitee during invitation reception
      console.log('⚡ Disconnect invitee (Alice) pendant réception...');
      await reconnectUser('alice');
      await sessions.get('alice')?.page.waitForTimeout(5000);

      // Try to accept
      console.log("Tentative d'acceptation après reconnexion...");
      const accepted = await acceptInvite('alice', 'bob', inv, 'Invite Chaos');

      console.log(`\nChaos 6 Result: Invitation ${accepted ? 'acceptée' : 'échouée'} après disconnect`);
    }
  });

  test('Chaos 7: Bilan final sous conditions adverses', async () => {
    metrics.startTest('Chaos 7: Final Summary');
    console.log('\n═══ CHAOS 7: Bilan final ═══\n');

    // Final round of messages to test overall stability
    console.log('Messages finaux de validation:');
    const results = [
      await sendDM('alice', 'bob', 'Final stability check 1'),
      await sendDM('bob', 'charlie', 'Final stability check 2'),
      await sendDM('charlie', 'alice', 'Final stability check 3'),
    ];

    const successCount = results.filter(Boolean).length;
    console.log(`\nChaos 7 Result: ${successCount}/${results.length} messages finaux`);
    console.log(`\n${'='.repeat(60)}`);
    console.log('TOTAL CHAOS EVENTS:', chaosEventCount);
    console.log('='.repeat(60));
  });
});
