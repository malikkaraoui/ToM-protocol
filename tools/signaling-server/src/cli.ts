// TEMPORARY: Bootstrap signaling server CLI (ADR-002)
import { createSignalingServer } from './server.js';

const PORT = Number(process.env.PORT) || 3001;
const server = createSignalingServer(PORT);

server.listening
  .then(() => {
    console.log(`ToM signaling server running on ws://localhost:${PORT}`);
    console.log(`Healthcheck: http://localhost:${PORT}/health`);
  })
  .catch((err: unknown) => {
    console.error('Failed to start signaling server:', err);
    process.exit(1);
  });

process.on('SIGINT', () => {
  server.close();
  process.exit(0);
});
