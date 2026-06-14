import { defineConfig } from 'vite';

export default defineConfig({
  root: '.',
  // Port dédié : évite la collision avec d'autres projets Vite (ex. 5173).
  server: {
    port: 5180,
    strictPort: true,
  },
  build: {
    outDir: 'dist',
  },
});
