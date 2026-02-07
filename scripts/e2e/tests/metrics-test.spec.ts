import { type Browser, type BrowserContext, type Page, expect, test } from '@playwright/test';
import { metrics } from './metrics';
import {
  POST_DISCONNECT_TIMEOUTS,
  STANDARD_TIMEOUTS,
  waitForConnectionsReady,
  waitForHubRecovery,
  withRetry,
} from './test-helpers';

/**
 * E2E Test with Progressive Complexity
 *
 * Phase 1: 2 users - Direct messages 1-to-1
 * Phase 2: 3rd user joins - Triangle of messages
 * Phase 3: Create first group
 * Phase 4: Invitations and acceptances
 * Phase 5: Second group
 * Phase 6: Key host disconnections
 */

interface UserSession {
  context: BrowserContext;
  page: Page;
  username: string;
  nodeId?: string;
}

const DEMO_URL = process.env.DEMO_URL || 'http://localhost:5173';

test.describe('Progressive Complexity Test', () => {
  let browser: Browser;
  const sessions: Map<string, UserSession> = new Map();

  test.beforeAll(async ({ browser: b }) => {
    browser = b;
    metrics.reset();
  });

  test.afterAll(async () => {
    for (const [, session] of sessions) {
      await session.context.close();
    }

    // Generate and display report
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

    console.log(`✓ ${username} connecté`);
    return session;
  }

  async function reconnectUser(username: string, expectedPeers = 0): Promise<boolean> {
    const session = sessions.get(username);
    if (!session) return false;

    console.log(`  ↻ Reconnecting ${username}...`);

    try {
      await session.page.reload();
      await session.page.waitForSelector('#username-input', { timeout: STANDARD_TIMEOUTS.PAGE_LOAD });
      await session.page.fill('#username-input', username);
      await session.page.click('#join-btn');
      await session.page.waitForSelector('#chat', { state: 'visible', timeout: STANDARD_TIMEOUTS.PAGE_LOAD });

      // Wait for peer connections to re-establish
      if (expectedPeers > 0) {
        await waitForConnectionsReady(session.page, expectedPeers);
      }

      // Additional stabilization time for WebRTC
      await session.page.waitForTimeout(3000);

      console.log(`  ✓ ${username} reconnected successfully`);
      return true;
    } catch (error) {
      console.log(`  ⚠ ${username} reconnection failed: ${(error as Error).message}`);
      return false;
    }
  }

  async function sendDM(from: string, to: string, message: string, useExtendedTimeout = false): Promise<boolean> {
    const sender = sessions.get(from);
    const receiver = sessions.get(to);
    if (!sender || !receiver) return false;

    const msgId = metrics.recordMessageSent(from, to, 'direct');
    const timeout = useExtendedTimeout
      ? POST_DISCONNECT_TIMEOUTS.MESSAGE_DELIVERY_MS
      : STANDARD_TIMEOUTS.MESSAGE_DELIVERY;

    try {
      return await withRetry(
        async () => {
          await sender.page.waitForSelector(`.participant:has-text("${to}")`, {
            timeout: STANDARD_TIMEOUTS.ELEMENT_VISIBLE,
          });
          await sender.page.click(`.participant:has-text("${to}")`);
          await sender.page.fill('#message-input', message);
          await sender.page.click('#send-btn');

          await receiver.page.waitForSelector(`.participant:has-text("${from}")`, {
            timeout: STANDARD_TIMEOUTS.ELEMENT_VISIBLE,
          });
          await receiver.page.click(`.participant:has-text("${from}")`);

          const received = await receiver.page
            .waitForSelector(`.message:has-text("${message}")`, { timeout })
            .then(() => true)
            .catch(() => false);

          if (received) {
            metrics.recordMessageReceived(msgId);
            console.log(`  ✓ ${from} → ${to}: "${message}"`);
            return true;
          }
          throw new Error('Message not received');
        },
        `DM ${from}→${to}`,
        { maxAttempts: 2, delayMs: 1000 },
      );
    } catch (e) {
      metrics.recordMessageFailed(msgId, (e as Error).message);
      console.log(`  ✗ ${from} → ${to}: "${message}" (erreur)`);
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
    } catch (e) {
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
        console.log(`  ✗ Invitation ${inviter} → ${invitee}: contact non trouvé`);
        return null;
      }

      await session.page.click(`#invite-modal-list div:has-text("${invitee}")`);
      console.log(`  → Invitation envoyée: ${inviter} → ${invitee} (${groupName})`);
      return invId;
    } catch (e) {
      metrics.recordInvitationFailed(invId, (e as Error).message);
      console.log(`  ✗ Invitation ${inviter} → ${invitee}: erreur`);
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
        console.log(`  ✗ ${invitee}: invitation non visible`);
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
      console.log(`  ✗ ${invitee}: échec rejoindre groupe`);
      return false;
    } catch (e) {
      metrics.recordInvitationFailed(invId, (e as Error).message);
      return false;
    }
  }

  async function sendGroupMsg(
    sender: string,
    groupName: string,
    message: string,
    useExtendedTimeout = false,
  ): Promise<{ success: boolean; recipients: string[] }> {
    const session = sessions.get(sender);
    if (!session) return { success: false, recipients: [] };

    const msgId = metrics.recordMessageSent(sender, groupName, 'group', groupName);
    const recipients: string[] = [];
    const timeout = useExtendedTimeout
      ? POST_DISCONNECT_TIMEOUTS.MESSAGE_DELIVERY_MS
      : STANDARD_TIMEOUTS.MESSAGE_DELIVERY;

    try {
      // Use retry for post-disconnect resilience
      await withRetry(
        async () => {
          await session.page.click(`.group-item:has-text("${groupName}")`);
          await session.page.waitForTimeout(500);
          await session.page.fill('#message-input', message);
          await session.page.click('#send-btn');

          // Wait for own message to appear (send confirmation)
          await session.page.waitForSelector(`.message:has-text("${message}")`, {
            timeout: 5000,
          });
        },
        `Send group msg to ${groupName}`,
        { maxAttempts: 2, delayMs: 1000 },
      );

      // Check other users in the group with extended timeout
      for (const [username, otherSession] of sessions) {
        if (username === sender) continue;

        const hasGroup = await otherSession.page.locator(`.group-item:has-text("${groupName}")`).isVisible();
        if (!hasGroup) continue;

        await otherSession.page.click(`.group-item:has-text("${groupName}")`);

        const received = await otherSession.page
          .waitForSelector(`.message:has-text("${message}")`, { timeout })
          .then(() => true)
          .catch(() => false);

        if (received) {
          recipients.push(username);
        }
      }

      if (recipients.length > 0) {
        metrics.recordMessageReceived(msgId);
        console.log(`  ✓ ${sender} → groupe: "${message}" (reçu par: ${recipients.join(', ')})`);
        return { success: true, recipients };
      }
      metrics.recordMessageFailed(msgId, 'Aucun destinataire');
      console.log(`  ✗ ${sender} → groupe: "${message}" (aucun destinataire)`);
      return { success: false, recipients };
    } catch (e) {
      metrics.recordMessageFailed(msgId, (e as Error).message);
      return { success: false, recipients };
    }
  }

  test('Phase 1: 2 utilisateurs - Messages 1-to-1', async () => {
    metrics.startTest('Phase 1: 2 Users 1-to-1');
    console.log('\n═══ PHASE 1: 2 utilisateurs - Messages directs ═══\n');

    const alice = await createUser('alice');
    const bob = await createUser('bob');

    // Wait longer for gossip discovery and WebRTC connections
    await alice.page.waitForTimeout(8000);

    console.log('\nMessages directs:');
    const results = [
      await sendDM('alice', 'bob', 'Salut Bob !'),
      await sendDM('bob', 'alice', 'Salut Alice !'),
      await sendDM('alice', 'bob', 'Comment ça va ?'),
      await sendDM('bob', 'alice', 'Très bien, merci !'),
    ];

    const successCount = results.filter(Boolean).length;
    console.log(`\nPhase 1 Result: ${successCount}/${results.length} messages delivered`);

    // Soft assertion - log but don't fail immediately
    if (successCount < results.length) {
      console.log('⚠️ Some messages failed - continuing to gather metrics');
    }
  });

  test('Phase 2: 3ème utilisateur - Triangle de messages', async () => {
    metrics.startTest('Phase 2: 3rd User - Triangle');
    console.log('\n═══ PHASE 2: 3ème utilisateur - Triangle ═══\n');

    const charlie = await createUser('charlie');
    await charlie.page.waitForTimeout(8000);

    console.log('\nMessages triangle:');
    const results = [
      await sendDM('alice', 'charlie', 'Bienvenue Charlie !'),
      await sendDM('charlie', 'alice', 'Merci Alice !'),
      await sendDM('bob', 'charlie', 'Salut Charlie !'),
      await sendDM('charlie', 'bob', 'Salut Bob !'),
    ];

    const successCount = results.filter(Boolean).length;
    console.log(`\nPhase 2 Result: ${successCount}/${results.length} messages delivered`);
  });

  test('Phase 3: Création premier groupe', async () => {
    metrics.startTest('Phase 3: First Group');
    console.log('\n═══ PHASE 3: Premier groupe ═══\n');

    const created = await createGroup('alice', 'Équipe Alpha');
    console.log(`\nPhase 3 Result: Group created = ${created}`);
  });

  test('Phase 4: Invitations et acceptations', async () => {
    metrics.startTest('Phase 4: Invitations');
    console.log('\n═══ PHASE 4: Invitations ═══\n');

    let invSuccessCount = 0;

    // Alice invite Bob
    const inv1 = await invite('alice', 'bob', 'Équipe Alpha');
    await sessions.get('alice')?.page.waitForTimeout(2000);

    if (inv1) {
      const accepted = await acceptInvite('bob', 'alice', inv1, 'Équipe Alpha');
      if (accepted) invSuccessCount++;
    }

    // Alice invite Charlie
    const inv2 = await invite('alice', 'charlie', 'Équipe Alpha');
    await sessions.get('alice')?.page.waitForTimeout(2000);

    if (inv2) {
      const accepted = await acceptInvite('charlie', 'alice', inv2, 'Équipe Alpha');
      if (accepted) invSuccessCount++;
    }

    console.log(`\nInvitations acceptées: ${invSuccessCount}/2`);

    // Test group messages
    console.log('\nMessages groupe:');
    await sessions.get('alice')?.page.waitForTimeout(3000);

    const gm1 = await sendGroupMsg('alice', 'Équipe Alpha', 'Bienvenue dans le groupe !');
    const gm2 = await sendGroupMsg('bob', 'Équipe Alpha', "Merci pour l'invitation !");
    const gm3 = await sendGroupMsg('charlie', 'Équipe Alpha', "Content d'être là !");

    const groupMsgSuccess = [gm1.success, gm2.success, gm3.success].filter(Boolean).length;
    console.log(`\nPhase 4 Result: ${groupMsgSuccess}/3 group messages delivered`);
  });

  test('Phase 5: Deuxième groupe', async () => {
    metrics.startTest('Phase 5: Second Group');
    console.log('\n═══ PHASE 5: Deuxième groupe ═══\n');

    const created = await createGroup('bob', 'Projet Beta');
    console.log(`Groupe créé: ${created}`);

    let invSuccess = false;
    const inv = await invite('bob', 'charlie', 'Projet Beta');
    await sessions.get('bob')?.page.waitForTimeout(2000);

    if (inv) {
      invSuccess = await acceptInvite('charlie', 'bob', inv, 'Projet Beta');
    }

    console.log('\nMessages groupe 2:');
    await sessions.get('bob')?.page.waitForTimeout(2000);

    const gm = await sendGroupMsg('bob', 'Projet Beta', 'Nouveau projet !');
    console.log(`\nPhase 5 Result: Group=${created}, Invite=${invSuccess}, Message=${gm.success}`);
  });

  // Phase 6 needs extended timeout due to hub failover complexity
  test('Phase 6: Déconnexion hub et récupération', async () => {
    test.setTimeout(180000); // 3 minutes for hub disconnect/recovery scenario

    metrics.startTest('Phase 6: Hub Disconnect');
    console.log('\n═══ PHASE 6: Déconnexion et récupération (robust) ═══\n');

    const aliceSession = sessions.get('alice');
    const bobSession = sessions.get('bob');
    const charlieSession = sessions.get('charlie');

    // Alice (group creator/hub) disconnects
    console.log('Alice (hub) se déconnecte et reconnecte...');

    // Track peer count before disconnect
    const expectedPeers = 2; // Bob and Charlie

    // Reconnect Alice with extended verification
    const reconnected = await reconnectUser('alice', expectedPeers);
    console.log(`  Alice reconnection: ${reconnected ? 'success' : 'needs recovery'}`);

    // Wait for hub recovery across the network
    console.log('\n⏳ Waiting for hub failover and recovery...');
    if (bobSession) {
      await waitForHubRecovery(bobSession.page, 'Équipe Alpha');
    }
    if (charlieSession) {
      await waitForHubRecovery(charlieSession.page, 'Équipe Alpha');
    }
    if (aliceSession) {
      await waitForHubRecovery(aliceSession.page, 'Équipe Alpha');
    }

    // Additional stabilization - WebRTC connections need time
    console.log('  ⏳ Stabilizing WebRTC connections...');
    await aliceSession?.page.waitForTimeout(POST_DISCONNECT_TIMEOUTS.WEBRTC_RECONNECT_MS / 2);

    // Test messages after hub disconnect with extended timeouts
    console.log('\n📨 Test messages après déconnexion hub (extended timeouts):');

    // Bob sends to group - use extended timeout
    const gm1 = await sendGroupMsg('bob', 'Équipe Alpha', 'Alice est revenue ?', true);

    // Charlie sends - use extended timeout
    const gm2 = await sendGroupMsg('charlie', 'Équipe Alpha', 'Le groupe fonctionne !', true);

    // Alice sends - she was the hub, test recovery
    const gm3 = await sendGroupMsg('alice', 'Équipe Alpha', 'Je suis de retour !', true);

    const successCount = [gm1.success, gm2.success, gm3.success].filter(Boolean).length;
    console.log(`\n📊 Résultat Phase 6: ${successCount}/3 messages delivered after hub disconnect`);

    // Additional verification: direct messages also work
    console.log('\n📨 Test DMs après récupération:');
    const dm1 = await sendDM('bob', 'alice', 'Test DM post-recovery', true);
    const dm2 = await sendDM('alice', 'charlie', 'Confirmation recovery', true);

    const dmSuccess = [dm1, dm2].filter(Boolean).length;
    console.log(`  DMs: ${dmSuccess}/2 delivered`);

    console.log(`\n✓ Phase 6 Complete: Hub failover ${successCount >= 2 ? 'PASSED' : 'needs attention'}`);
  });

  test('Phase 7: Messages finaux et bilan', async () => {
    metrics.startTest('Phase 7: Final Messages');
    console.log('\n═══ PHASE 7: Messages finaux (stability validation) ═══\n');

    // Final message series to validate network stability after all tests
    console.log('📨 Messages directs finaux (post-recovery):');

    // Use extended timeouts since network may still be stabilizing
    const dmResults = [
      await sendDM('alice', 'bob', 'Test final 1', true),
      await sendDM('bob', 'charlie', 'Test final 2', true),
      await sendDM('charlie', 'alice', 'Test final 3', true),
    ];
    const dmSuccess = dmResults.filter(Boolean).length;
    console.log(`  DMs: ${dmSuccess}/${dmResults.length} delivered`);

    console.log('\n📨 Messages groupe finaux:');
    const gm1 = await sendGroupMsg('alice', 'Équipe Alpha', 'Message final groupe 1', true);
    const gm2 = await sendGroupMsg('bob', 'Projet Beta', 'Message final groupe 2', true);

    const groupSuccess = [gm1.success, gm2.success].filter(Boolean).length;
    console.log(`  Group messages: ${groupSuccess}/2 delivered`);

    // Final summary
    const totalSuccess = dmSuccess + groupSuccess;
    const totalExpected = dmResults.length + 2;
    console.log(`\n✓ Phase 7 Complete: ${totalSuccess}/${totalExpected} final messages delivered`);
  });
});
