import { defineConfig } from 'vitest/config';
import { fileURLToPath } from 'node:url';

const pages = ['overlay'];

export default defineConfig({
  root: 'src',
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  build: {
    outDir: '../dist',
    emptyOutDir: true,
    target: 'es2022',
    rollupOptions: {
      input: Object.fromEntries(pages.map((p) => [p, fileURLToPath(new URL(`./src/${p}/index.html`, import.meta.url))])),
    },
  },
  test: { include: ['**/*.test.ts'] },
});
