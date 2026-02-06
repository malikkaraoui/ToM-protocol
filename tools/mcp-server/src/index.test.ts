import { describe, expect, it } from 'vitest';
import { TomMcpServer } from './index.js';

describe('TomMcpServer', () => {
  it('should expose tools list', () => {
    const server = new TomMcpServer('ws://localhost:3001', 'test-bot');
    const tools = server.getTools();

    expect(tools.length).toBeGreaterThan(0);
    expect(tools.map((t) => t.name)).toContain('tom_connect');
    expect(tools.map((t) => t.name)).toContain('tom_send_message');
    expect(tools.map((t) => t.name)).toContain('tom_list_participants');
    expect(tools.map((t) => t.name)).toContain('tom_get_network_status');
  });

  it('should have valid tool schemas', () => {
    const server = new TomMcpServer('ws://localhost:3001', 'test-bot');
    const tools = server.getTools();

    for (const tool of tools) {
      expect(tool.name).toMatch(/^tom_/);
      expect(tool.description).toBeTruthy();
      expect(tool.inputSchema.type).toBe('object');
      expect(tool.inputSchema.properties).toBeDefined();
      expect(tool.inputSchema.required).toBeDefined();
    }
  });

  it('should require connection for most operations', async () => {
    const server = new TomMcpServer('ws://localhost:3001', 'test-bot');

    // These should fail without connection
    const result1 = await server.executeTool({ name: 'tom_send_message', arguments: { to: 'test', message: 'hi' } });
    expect(result1.isError).toBe(true);
    expect(result1.content[0].text).toContain('Not connected');

    const result2 = await server.executeTool({ name: 'tom_list_participants', arguments: {} });
    expect(result2.isError).toBe(true);

    const result3 = await server.executeTool({ name: 'tom_get_network_status', arguments: {} });
    expect(result3.isError).toBe(true);
  });

  it('should handle unknown tool gracefully', async () => {
    const server = new TomMcpServer('ws://localhost:3001', 'test-bot');
    const result = await server.executeTool({ name: 'unknown_tool', arguments: {} });

    expect(result.isError).toBe(true);
    expect(result.content[0].text).toContain('Unknown tool');
  });

  it('should return message history', async () => {
    const server = new TomMcpServer('ws://localhost:3001', 'test-bot');
    const result = await server.executeTool({ name: 'tom_get_message_history', arguments: { limit: 10 } });

    // History is empty but should not error
    expect(result.isError).toBeUndefined();
    const data = JSON.parse(result.content[0].text);
    expect(data.count).toBe(0);
    expect(data.messages).toEqual([]);
  });
});
