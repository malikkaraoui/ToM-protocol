/**
 * ToM Protocol MCP Server
 *
 * Enables LLMs to interact with the ToM network via Model Context Protocol.
 * Exposes tools for sending messages, querying participants, and network status.
 *
 * @see https://modelcontextprotocol.io for MCP specification
 */

import type { MessageEnvelope, PeerInfo } from 'tom-protocol';
import { TomClient } from 'tom-sdk';

export interface McpTool {
  name: string;
  description: string;
  inputSchema: {
    type: 'object';
    properties: Record<string, { type: string; description: string }>;
    required: string[];
  };
}

export interface McpToolCall {
  name: string;
  arguments: Record<string, unknown>;
}

export interface McpToolResult {
  content: Array<{ type: 'text'; text: string }>;
  isError?: boolean;
}

/**
 * ToM MCP Server - provides tools for LLM interaction with the network
 */
export class TomMcpServer {
  private client: TomClient | null = null;
  private signalingUrl: string;
  private username: string;
  private messageHistory: Array<{
    id: string;
    from: string;
    to: string;
    text: string;
    timestamp: number;
    status: string;
  }> = [];

  constructor(signalingUrl: string, username: string) {
    this.signalingUrl = signalingUrl;
    this.username = username;
  }

  /**
   * Get available tools for the MCP protocol
   */
  getTools(): McpTool[] {
    return [
      {
        name: 'tom_connect',
        description: 'Connect to the ToM network. Must be called before other operations.',
        inputSchema: {
          type: 'object',
          properties: {},
          required: [],
        },
      },
      {
        name: 'tom_disconnect',
        description: 'Disconnect from the ToM network.',
        inputSchema: {
          type: 'object',
          properties: {},
          required: [],
        },
      },
      {
        name: 'tom_send_message',
        description: 'Send a message to another participant on the ToM network.',
        inputSchema: {
          type: 'object',
          properties: {
            to: {
              type: 'string',
              description: 'The node ID or username of the recipient',
            },
            message: {
              type: 'string',
              description: 'The message text to send',
            },
          },
          required: ['to', 'message'],
        },
      },
      {
        name: 'tom_list_participants',
        description: 'List all currently connected participants on the ToM network.',
        inputSchema: {
          type: 'object',
          properties: {},
          required: [],
        },
      },
      {
        name: 'tom_get_network_status',
        description: 'Get current network status including role, peers, and statistics.',
        inputSchema: {
          type: 'object',
          properties: {},
          required: [],
        },
      },
      {
        name: 'tom_get_message_history',
        description: 'Get recent message history (sent and received).',
        inputSchema: {
          type: 'object',
          properties: {
            limit: {
              type: 'number',
              description: 'Maximum number of messages to return (default: 20)',
            },
          },
          required: [],
        },
      },
      {
        name: 'tom_get_gossip_stats',
        description: 'Get peer discovery statistics from the gossip protocol.',
        inputSchema: {
          type: 'object',
          properties: {},
          required: [],
        },
      },
      {
        name: 'tom_get_subnet_stats',
        description: 'Get ephemeral subnet statistics.',
        inputSchema: {
          type: 'object',
          properties: {},
          required: [],
        },
      },
    ];
  }

  /**
   * Execute a tool call
   */
  async executeTool(call: McpToolCall): Promise<McpToolResult> {
    try {
      switch (call.name) {
        case 'tom_connect':
          return await this.handleConnect();
        case 'tom_disconnect':
          return this.handleDisconnect();
        case 'tom_send_message':
          return await this.handleSendMessage(call.arguments as { to: string; message: string });
        case 'tom_list_participants':
          return this.handleListParticipants();
        case 'tom_get_network_status':
          return this.handleGetNetworkStatus();
        case 'tom_get_message_history':
          return this.handleGetMessageHistory(call.arguments as { limit?: number });
        case 'tom_get_gossip_stats':
          return this.handleGetGossipStats();
        case 'tom_get_subnet_stats':
          return this.handleGetSubnetStats();
        default:
          return this.errorResult(`Unknown tool: ${call.name}`);
      }
    } catch (error) {
      return this.errorResult(error instanceof Error ? error.message : 'Unknown error');
    }
  }

  private async handleConnect(): Promise<McpToolResult> {
    if (this.client) {
      return this.successResult('Already connected to ToM network');
    }

    this.client = new TomClient({
      signalingUrl: this.signalingUrl,
      username: this.username,
    });

    // Set up message handler
    this.client.onMessage((envelope: MessageEnvelope) => {
      const payload = envelope.payload as { text?: string };
      this.messageHistory.push({
        id: envelope.id,
        from: envelope.from,
        to: envelope.to,
        text: payload.text ?? '[encrypted]',
        timestamp: envelope.timestamp,
        status: 'received',
      });
    });

    await this.client.connect();

    return this.successResult(
      JSON.stringify(
        {
          status: 'connected',
          nodeId: this.client.getNodeId(),
          username: this.username,
          signalingUrl: this.signalingUrl,
        },
        null,
        2,
      ),
    );
  }

  private handleDisconnect(): McpToolResult {
    if (!this.client) {
      return this.successResult('Not connected');
    }

    this.client.disconnect();
    this.client = null;

    return this.successResult('Disconnected from ToM network');
  }

  private async handleSendMessage(args: { to: string; message: string }): Promise<McpToolResult> {
    if (!this.client) {
      return this.errorResult('Not connected. Call tom_connect first.');
    }

    const { to, message } = args;

    // Find recipient by username or node ID
    const participants = this.client.getTopology();
    let recipientId = to;

    // If 'to' doesn't look like a node ID, try to find by username
    if (to.length < 32) {
      const participant = participants.find((p: PeerInfo) => p.username.toLowerCase() === to.toLowerCase());
      if (participant) {
        recipientId = participant.nodeId;
      } else {
        return this.errorResult(
          `Participant not found: ${to}. Available: ${participants.map((p: PeerInfo) => p.username).join(', ')}`,
        );
      }
    }

    const envelope = await this.client.sendMessage(recipientId, message);

    if (envelope) {
      this.messageHistory.push({
        id: envelope.id,
        from: this.client.getNodeId(),
        to: recipientId,
        text: message,
        timestamp: Date.now(),
        status: 'sent',
      });

      return this.successResult(
        JSON.stringify(
          {
            status: 'sent',
            messageId: envelope.id,
            to: recipientId,
            message,
          },
          null,
          2,
        ),
      );
    }

    return this.errorResult('Failed to send message');
  }

  private handleListParticipants(): McpToolResult {
    if (!this.client) {
      return this.errorResult('Not connected. Call tom_connect first.');
    }

    const participants = this.client.getTopology();

    return this.successResult(
      JSON.stringify(
        {
          count: participants.length,
          participants: participants.map((p: PeerInfo) => ({
            nodeId: p.nodeId,
            username: p.username,
            roles: p.roles,
            status: 'online',
          })),
        },
        null,
        2,
      ),
    );
  }

  private handleGetNetworkStatus(): McpToolResult {
    if (!this.client) {
      return this.errorResult('Not connected. Call tom_connect first.');
    }

    const participants = this.client.getTopology();
    const roles = this.client.getCurrentRoles();
    const gossipStats = this.client.getGossipStats();
    const subnetStats = this.client.getSubnetStats();

    return this.successResult(
      JSON.stringify(
        {
          nodeId: this.client.getNodeId(),
          username: this.username,
          roles,
          connectedPeers: participants.length,
          gossip: gossipStats,
          subnets: subnetStats,
        },
        null,
        2,
      ),
    );
  }

  private handleGetMessageHistory(args: { limit?: number }): McpToolResult {
    const limit = args.limit ?? 20;
    const messages = this.messageHistory.slice(-limit);

    return this.successResult(
      JSON.stringify(
        {
          count: messages.length,
          messages: messages.map((m) => ({
            id: m.id,
            from: `${m.from.slice(0, 8)}...`,
            to: `${m.to.slice(0, 8)}...`,
            text: m.text,
            timestamp: new Date(m.timestamp).toISOString(),
            status: m.status,
          })),
        },
        null,
        2,
      ),
    );
  }

  private handleGetGossipStats(): McpToolResult {
    if (!this.client) {
      return this.errorResult('Not connected. Call tom_connect first.');
    }

    const stats = this.client.getGossipStats();

    return this.successResult(
      JSON.stringify(
        {
          totalPeers: stats.totalPeers,
          bootstrapPeers: stats.bootstrapPeers,
          gossipPeers: stats.gossipPeers,
          connectedPeers: stats.connectedPeers,
          bootstrapDependency:
            stats.totalPeers > 0 ? `${((stats.bootstrapPeers / stats.totalPeers) * 100).toFixed(1)}%` : 'N/A',
        },
        null,
        2,
      ),
    );
  }

  private handleGetSubnetStats(): McpToolResult {
    if (!this.client) {
      return this.errorResult('Not connected. Call tom_connect first.');
    }

    const stats = this.client.getSubnetStats();
    const subnets = this.client.getSubnets();

    return this.successResult(
      JSON.stringify(
        {
          totalSubnets: stats.totalSubnets,
          totalNodesInSubnets: stats.totalNodesInSubnets,
          averageSubnetSize: stats.averageSubnetSize.toFixed(1),
          communicationEdges: stats.communicationEdges,
          subnets: subnets.map((s) => ({
            id: s.subnetId,
            members: s.members.size,
            formedAt: new Date(s.formedAt).toISOString(),
            lastActivity: new Date(s.lastActivity).toISOString(),
            densityScore: s.densityScore.toFixed(2),
          })),
        },
        null,
        2,
      ),
    );
  }

  private successResult(text: string): McpToolResult {
    return {
      content: [{ type: 'text', text }],
    };
  }

  private errorResult(message: string): McpToolResult {
    return {
      content: [{ type: 'text', text: `Error: ${message}` }],
      isError: true,
    };
  }
}
