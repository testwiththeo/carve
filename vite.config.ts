/// <reference types="vitest" />
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';
import path from 'path';

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: [
      { find: /^konva$/, replacement: path.resolve(__dirname, 'node_modules/konva/lib/index.js') },
    ],
  },
  test: {
    server: {
      deps: {
        inline: ['konva', 'react-konva'],
      },
    },
  },
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
  },
});
