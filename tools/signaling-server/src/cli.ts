// TEMPORARY: Bootstrap signaling server CLI (ADR-002)
import { createSignalingServer } from './server.js';

const PORT = Number(process.env.PORT) || 3001;
const server = createSignalingServer(PORT);

console.log(`ToM signaling server running on ws://localhost:${PORT}`);

process.on('SIGINT', () => {
  server.close();
  process.exit(0);
});
