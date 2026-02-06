#!/usr/bin/env node
/**
 * ToM Protocol MCP Server CLI
 *
 * Runs the MCP server over stdio for LLM integration.
 *
 * Usage:
 *   tom-mcp --url wss://signaling.example.com --username bot
 *
 * Or via environment variables:
 *   TOM_SIGNALING_URL=wss://... TOM_USERNAME=bot tom-mcp
 */

import * as readline from 'node:readline';
import { TomMcpServer } from './index.js';

interface JsonRpcRequest {
  jsonrpc: '2.0';
  id: number | string;
  method: string;
  params?: Record<string, unknown>;
}

interface JsonRpcResponse {
  jsonrpc: '2.0';
  id: number | string | null;
  result?: unknown;
  error?: { code: number; message: string; data?: unknown };
}

function parseArgs(): { signalingUrl: string; username: string } {
  const args = process.argv.slice(2);
  let signalingUrl = process.env.TOM_SIGNALING_URL ?? 'ws://localhost:3001';
  let username = process.env.TOM_USERNAME ?? 'mcp-bot';

  for (let i = 0; i < args.length; i++) {
    if (args[i] === '--url' && args[i + 1]) {
      signalingUrl = args[i + 1];
      i++;
    } else if (args[i] === '--username' && args[i + 1]) {
      username = args[i + 1];
      i++;
    } else if (args[i] === '--help' || args[i] === '-h') {
      console.error(`
ToM Protocol MCP Server

Usage: tom-mcp [options]

Options:
  --url <url>       Signaling server URL (default: ws://localhost:3001)
  --username <name> Bot username (default: mcp-bot)
  -h, --help        Show this help message

Environment variables:
  TOM_SIGNALING_URL   Signaling server URL
  TOM_USERNAME        Bot username

Example:
  tom-mcp --url wss://signaling.example.com --username assistant
`);
      process.exit(0);
    }
  }

  return { signalingUrl, username };
}

async function main(): Promise<void> {
  const { signalingUrl, username } = parseArgs();
  const server = new TomMcpServer(signalingUrl, username);

  // MCP server info
  const serverInfo = {
    name: 'tom-mcp-server',
    version: '1.0.0',
    description: 'ToM Protocol MCP Server - enables LLM interaction with P2P messaging network',
  };

  // Set up readline for stdio communication
  const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout,
    terminal: false,
  });

  function sendResponse(response: JsonRpcResponse): void {
    process.stdout.write(`${JSON.stringify(response)}\n`);
  }

  function sendError(id: number | string | null, code: number, message: string): void {
    sendResponse({
      jsonrpc: '2.0',
      id,
      error: { code, message },
    });
  }

  rl.on('line', async (line) => {
    let request: JsonRpcRequest;

    try {
      request = JSON.parse(line);
    } catch {
      sendError(null, -32700, 'Parse error');
      return;
    }

    if (request.jsonrpc !== '2.0') {
      sendError(request.id, -32600, 'Invalid Request');
      return;
    }

    try {
      switch (request.method) {
        case 'initialize': {
          sendResponse({
            jsonrpc: '2.0',
            id: request.id,
            result: {
              protocolVersion: '2024-11-05',
              capabilities: {
                tools: {},
              },
              serverInfo,
            },
          });
          break;
        }

        case 'initialized': {
          // No response needed for notification
          break;
        }

        case 'tools/list': {
          const tools = server.getTools();
          sendResponse({
            jsonrpc: '2.0',
            id: request.id,
            result: { tools },
          });
          break;
        }

        case 'tools/call': {
          const params = request.params as { name: string; arguments?: Record<string, unknown> };
          const result = await server.executeTool({
            name: params.name,
            arguments: params.arguments ?? {},
          });
          sendResponse({
            jsonrpc: '2.0',
            id: request.id,
            result,
          });
          break;
        }

        case 'ping': {
          sendResponse({
            jsonrpc: '2.0',
            id: request.id,
            result: {},
          });
          break;
        }

        default: {
          sendError(request.id, -32601, `Method not found: ${request.method}`);
        }
      }
    } catch (error) {
      sendError(request.id, -32603, error instanceof Error ? error.message : 'Internal error');
    }
  });

  rl.on('close', () => {
    process.exit(0);
  });

  // Log to stderr (not stdout, which is for MCP protocol)
  console.error('ToM MCP Server started');
  console.error(`  Signaling URL: ${signalingUrl}`);
  console.error(`  Username: ${username}`);
  console.error('  Waiting for MCP client...');
}

main().catch((error) => {
  console.error('Fatal error:', error);
  process.exit(1);
});
