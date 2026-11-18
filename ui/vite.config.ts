import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';

// Relative Basis, damit die gebaute Oberfläche auch in der Tauri-Hülle läuft.
export default defineConfig({
  base: './',
  plugins: [react()],
  build: { target: 'es2022', sourcemap: false },
  test: { environment: 'node', include: ['src/**/*.test.ts'] },
});
