/**
 * E2E Test Metrics Collector
 *
 * Tracks detailed metrics for test analysis:
 * - Messages: sent, received, success rate
 * - Groups: created, members joined
 * - Invitations: sent, accepted, declined, failed
 * - Timing: latencies for each operation
 */

export interface MessageMetric {
  id: string;
  from: string;
  to: string;
  type: 'direct' | 'group';
  groupId?: string;
  sentAt: number;
  receivedAt?: number;
  success: boolean;
  error?: string;
}

export interface InvitationMetric {
  id: string;
  groupId: string;
  groupName: string;
  from: string;
  to: string;
  sentAt: number;
  receivedAt?: number;
  acceptedAt?: number;
  declinedAt?: number;
  status: 'pending' | 'received' | 'accepted' | 'declined' | 'failed';
  error?: string;
}

export interface GroupMetric {
  id: string;
  name: string;
  createdBy: string;
  createdAt: number;
  members: string[];
  messagesCount: number;
  invitationsSent: number;
  invitationsAccepted: number;
}

export interface TestMetrics {
  testName: string;
  startedAt: number;
  endedAt?: number;
  messages: MessageMetric[];
  invitations: InvitationMetric[];
  groups: GroupMetric[];
  users: string[];
}

class MetricsCollector {
  private metrics: TestMetrics[] = [];
  private currentTest: TestMetrics | null = null;

  startTest(testName: string): void {
    this.currentTest = {
      testName,
      startedAt: Date.now(),
      messages: [],
      invitations: [],
      groups: [],
      users: [],
    };
  }

  endTest(): void {
    if (this.currentTest) {
      this.currentTest.endedAt = Date.now();
      this.metrics.push(this.currentTest);
      this.currentTest = null;
    }
  }

  addUser(username: string): void {
    if (this.currentTest && !this.currentTest.users.includes(username)) {
      this.currentTest.users.push(username);
    }
  }

  recordMessageSent(from: string, to: string, type: 'direct' | 'group', groupId?: string): string {
    const id = `msg-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
    if (this.currentTest) {
      this.currentTest.messages.push({
        id,
        from,
        to,
        type,
        groupId,
        sentAt: Date.now(),
        success: false,
      });
    }
    return id;
  }

  recordMessageReceived(id: string): void {
    if (this.currentTest) {
      const msg = this.currentTest.messages.find((m) => m.id === id);
      if (msg) {
        msg.receivedAt = Date.now();
        msg.success = true;
      }
    }
  }

  recordMessageFailed(id: string, error: string): void {
    if (this.currentTest) {
      const msg = this.currentTest.messages.find((m) => m.id === id);
      if (msg) {
        msg.success = false;
        msg.error = error;
      }
    }
  }

  recordGroupCreated(groupId: string, name: string, createdBy: string): void {
    if (this.currentTest) {
      this.currentTest.groups.push({
        id: groupId,
        name,
        createdBy,
        createdAt: Date.now(),
        members: [createdBy],
        messagesCount: 0,
        invitationsSent: 0,
        invitationsAccepted: 0,
      });
    }
  }

  recordInvitationSent(groupId: string, groupName: string, from: string, to: string): string {
    const id = `inv-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
    if (this.currentTest) {
      this.currentTest.invitations.push({
        id,
        groupId,
        groupName,
        from,
        to,
        sentAt: Date.now(),
        status: 'pending',
      });
      const group = this.currentTest.groups.find((g) => g.id === groupId || g.name === groupName);
      if (group) {
        group.invitationsSent++;
      }
    }
    return id;
  }

  recordInvitationReceived(id: string): void {
    if (this.currentTest) {
      const inv = this.currentTest.invitations.find((i) => i.id === id);
      if (inv) {
        inv.receivedAt = Date.now();
        inv.status = 'received';
      }
    }
  }

  recordInvitationAccepted(id: string): void {
    if (this.currentTest) {
      const inv = this.currentTest.invitations.find((i) => i.id === id);
      if (inv) {
        inv.acceptedAt = Date.now();
        inv.status = 'accepted';
        const group = this.currentTest.groups.find((g) => g.id === inv.groupId || g.name === inv.groupName);
        if (group) {
          group.invitationsAccepted++;
          if (!group.members.includes(inv.to)) {
            group.members.push(inv.to);
          }
        }
      }
    }
  }

  recordInvitationDeclined(id: string): void {
    if (this.currentTest) {
      const inv = this.currentTest.invitations.find((i) => i.id === id);
      if (inv) {
        inv.declinedAt = Date.now();
        inv.status = 'declined';
      }
    }
  }

  recordInvitationFailed(id: string, error: string): void {
    if (this.currentTest) {
      const inv = this.currentTest.invitations.find((i) => i.id === id);
      if (inv) {
        inv.status = 'failed';
        inv.error = error;
      }
    }
  }

  generateReport(): string {
    const allMessages = this.metrics.flatMap((t) => t.messages);
    const allInvitations = this.metrics.flatMap((t) => t.invitations);
    const allGroups = this.metrics.flatMap((t) => t.groups);
    const allUsers = [...new Set(this.metrics.flatMap((t) => t.users))];

    // Message stats
    const directMessages = allMessages.filter((m) => m.type === 'direct');
    const groupMessages = allMessages.filter((m) => m.type === 'group');
    const successfulMessages = allMessages.filter((m) => m.success);
    const failedMessages = allMessages.filter((m) => !m.success);

    // Calculate latencies
    const messageLatencies = successfulMessages.filter((m) => m.receivedAt).map((m) => m.receivedAt! - m.sentAt);
    const avgMessageLatency =
      messageLatencies.length > 0 ? messageLatencies.reduce((a, b) => a + b, 0) / messageLatencies.length : 0;

    // Invitation stats
    const invitationsSent = allInvitations.length;
    const invitationsReceived = allInvitations.filter((i) => i.receivedAt).length;
    const invitationsAccepted = allInvitations.filter((i) => i.status === 'accepted').length;
    const invitationsDeclined = allInvitations.filter((i) => i.status === 'declined').length;
    const invitationsFailed = allInvitations.filter((i) => i.status === 'failed' || i.status === 'pending').length;

    // Calculate invitation latencies
    const invReceiveLatencies = allInvitations.filter((i) => i.receivedAt).map((i) => i.receivedAt! - i.sentAt);
    const avgInvReceiveLatency =
      invReceiveLatencies.length > 0 ? invReceiveLatencies.reduce((a, b) => a + b, 0) / invReceiveLatencies.length : 0;

    const invAcceptLatencies = allInvitations
      .filter((i) => i.acceptedAt && i.receivedAt)
      .map((i) => i.acceptedAt! - i.receivedAt!);
    const avgInvAcceptLatency =
      invAcceptLatencies.length > 0 ? invAcceptLatencies.reduce((a, b) => a + b, 0) / invAcceptLatencies.length : 0;

    // Group stats
    const totalMembers = allGroups.reduce((sum, g) => sum + g.members.length, 0);
    const avgMembersPerGroup = allGroups.length > 0 ? totalMembers / allGroups.length : 0;

    const report = `
╔══════════════════════════════════════════════════════════════════════════════╗
║                        ToM Protocol E2E Test Report                          ║
╠══════════════════════════════════════════════════════════════════════════════╣

┌─────────────────────────────────────────────────────────────────────────────┐
│                              GLOBAL SUMMARY                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│  Tests exécutés:        ${this.metrics.length.toString().padStart(6)}                                         │
│  Utilisateurs:          ${allUsers.length.toString().padStart(6)}                                         │
│  Groupes créés:         ${allGroups.length.toString().padStart(6)}                                         │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│                              MESSAGES                                        │
├─────────────────────────────────────────────────────────────────────────────┤
│  Total envoyés:         ${allMessages.length.toString().padStart(6)}                                         │
│  ├── Directs:           ${directMessages.length.toString().padStart(6)}                                         │
│  └── Groupe:            ${groupMessages.length.toString().padStart(6)}                                         │
│                                                                              │
│  ✅ Réussis:            ${successfulMessages.length.toString().padStart(6)}   (${((successfulMessages.length / Math.max(allMessages.length, 1)) * 100).toFixed(1).padStart(5)}%)                        │
│  ❌ Échoués:            ${failedMessages.length.toString().padStart(6)}   (${((failedMessages.length / Math.max(allMessages.length, 1)) * 100).toFixed(1).padStart(5)}%)                        │
│                                                                              │
│  Latence moyenne:       ${avgMessageLatency.toFixed(0).padStart(6)} ms                                     │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│                            INVITATIONS                                       │
├─────────────────────────────────────────────────────────────────────────────┤
│  Total envoyées:        ${invitationsSent.toString().padStart(6)}                                         │
│                                                                              │
│  📤 Envoi → Réception:  ${invitationsReceived.toString().padStart(6)}   (${((invitationsReceived / Math.max(invitationsSent, 1)) * 100).toFixed(1).padStart(5)}%)                        │
│  ✅ Réception → Accept: ${invitationsAccepted.toString().padStart(6)}   (${((invitationsAccepted / Math.max(invitationsReceived, 1)) * 100).toFixed(1).padStart(5)}%)                        │
│  ❌ Refusées:           ${invitationsDeclined.toString().padStart(6)}   (${((invitationsDeclined / Math.max(invitationsReceived, 1)) * 100).toFixed(1).padStart(5)}%)                        │
│  ⚠️  Échouées/Pending:  ${invitationsFailed.toString().padStart(6)}   (${((invitationsFailed / Math.max(invitationsSent, 1)) * 100).toFixed(1).padStart(5)}%)                        │
│                                                                              │
│  Latence envoi→récept:  ${avgInvReceiveLatency.toFixed(0).padStart(6)} ms                                     │
│  Latence récept→accept: ${avgInvAcceptLatency.toFixed(0).padStart(6)} ms                                     │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│                              GROUPES                                         │
├─────────────────────────────────────────────────────────────────────────────┤
│  Total créés:           ${allGroups.length.toString().padStart(6)}                                         │
│  Membres totaux:        ${totalMembers.toString().padStart(6)}                                         │
│  Moyenne membres/grp:   ${avgMembersPerGroup.toFixed(1).padStart(6)}                                         │
└─────────────────────────────────────────────────────────────────────────────┘

${this.generateGroupDetails(allGroups)}

${this.generateFailureDetails(
  failedMessages,
  allInvitations.filter((i) => i.status === 'failed' || i.status === 'pending'),
)}

╚══════════════════════════════════════════════════════════════════════════════╝
    `;

    return report;
  }

  private generateGroupDetails(groups: GroupMetric[]): string {
    if (groups.length === 0) return '';

    let details = '┌─────────────────────────────────────────────────────────────────────────────┐\n';
    details += '│                         DÉTAILS PAR GROUPE                                  │\n';
    details += '├─────────────────────────────────────────────────────────────────────────────┤\n';

    for (const group of groups) {
      const invSuccessRate =
        group.invitationsSent > 0 ? ((group.invitationsAccepted / group.invitationsSent) * 100).toFixed(0) : '0';

      details += `│  📁 ${group.name.padEnd(30)}                                │\n`;
      details += `│     Créé par: ${group.createdBy.padEnd(20)}                             │\n`;
      details += `│     Membres: ${group.members.length} / Invit. acceptées: ${group.invitationsAccepted}/${group.invitationsSent} (${invSuccessRate}%)      │\n`;
      details += `│     Messages: ${group.messagesCount}                                                   │\n`;
      details += '│                                                                              │\n';
    }

    details += '└─────────────────────────────────────────────────────────────────────────────┘';
    return details;
  }

  private generateFailureDetails(failedMessages: MessageMetric[], failedInvitations: InvitationMetric[]): string {
    if (failedMessages.length === 0 && failedInvitations.length === 0) {
      return (
        '┌─────────────────────────────────────────────────────────────────────────────┐\n' +
        '│  ✅ AUCUNE ERREUR DÉTECTÉE                                                  │\n' +
        '└─────────────────────────────────────────────────────────────────────────────┘'
      );
    }

    let details = '┌─────────────────────────────────────────────────────────────────────────────┐\n';
    details += '│  ⚠️  ERREURS DÉTAILLÉES                                                     │\n';
    details += '├─────────────────────────────────────────────────────────────────────────────┤\n';

    if (failedMessages.length > 0) {
      details += '│  Messages échoués:                                                          │\n';
      for (const msg of failedMessages.slice(0, 5)) {
        details += `│    - ${msg.from} → ${msg.to}: ${(msg.error || 'Timeout').slice(0, 40).padEnd(40)}  │\n`;
      }
      if (failedMessages.length > 5) {
        details += `│    ... et ${failedMessages.length - 5} autres                                                    │\n`;
      }
    }

    if (failedInvitations.length > 0) {
      details += '│  Invitations échouées:                                                      │\n';
      for (const inv of failedInvitations.slice(0, 5)) {
        details += `│    - ${inv.from} → ${inv.to} (${inv.groupName}): ${inv.status.padEnd(10)}             │\n`;
      }
      if (failedInvitations.length > 5) {
        details += `│    ... et ${failedInvitations.length - 5} autres                                                    │\n`;
      }
    }

    details += '└─────────────────────────────────────────────────────────────────────────────┘';
    return details;
  }

  getMetrics(): TestMetrics[] {
    return this.metrics;
  }

  reset(): void {
    this.metrics = [];
    this.currentTest = null;
  }
}

// Singleton instance
export const metrics = new MetricsCollector();
