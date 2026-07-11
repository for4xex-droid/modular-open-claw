#!/usr/bin/env node
/**
 * ServeDir (api-server) has no SPA fallback. Copy index.html into
 * deep routes so soft-refresh / Stripe return URLs serve the shell.
 */
import { copyFileSync, mkdirSync, existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const dist = join(root, 'dist');
const index = join(dist, 'index.html');

if (!existsSync(index)) {
  console.error('copy-spa-fallbacks: dist/index.html missing — run vite build first');
  process.exit(1);
}

const targets = ['checkout/success/index.html'];

for (const rel of targets) {
  const dest = join(dist, rel);
  mkdirSync(dirname(dest), { recursive: true });
  copyFileSync(index, dest);
  console.log('spa-fallback:', rel);
}
