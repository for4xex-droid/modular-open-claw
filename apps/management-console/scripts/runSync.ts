/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { diffTokenMaps, generateTokensCss } from './syncDesignTokens';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const rootDir = path.resolve(__dirname, '..');
const designMdPath = path.resolve(rootDir, 'DESIGN.md');
const aiomeTokensPath = path.resolve(rootDir, 'src/styles/tokens.css');

// Nurture UI sync path (cross-repository)
const nurtureFallbackPath = path.resolve(rootDir, '../../../Project-Nurture/apps/nurture-ui/src');
const nurtureUiDir = process.env.NURTURE_UI_DIR || nurtureFallbackPath;
const nurtureTokensPath = path.join(nurtureUiDir, 'tokens.css');

function writeIfChanged(filePath: string, content: string): boolean {
  if (fs.existsSync(filePath)) {
    const existing = fs.readFileSync(filePath, 'utf-8');
    if (existing === content) {
      console.log(`[OK] ${filePath} already up to date — skipped.`);
      return false;
    }
  }
  fs.writeFileSync(filePath, content);
  console.log(`Wrote tokens to ${filePath}`);
  return true;
}

console.log('Reading DESIGN.md and tokens.css template...');
const markdown = fs.readFileSync(designMdPath, 'utf-8');
const templateCss = fs.readFileSync(aiomeTokensPath, 'utf-8');

const parity = diffTokenMaps(markdown, templateCss);
if (parity.missingInDesign.length > 0) {
  console.error('[ERROR] tokens.css keys missing from DESIGN.md:', parity.missingInDesign.join(', '));
  process.exit(1);
}
if (parity.missingInCss.length > 0) {
  console.warn(
    `[WARN] ${parity.missingInCss.length} DESIGN.md token(s) missing from template — will append before closing brace:`,
  );
  parity.missingInCss.slice(0, 5).forEach((key) => console.warn(`  ${key}`));
}
if (parity.mismatches.length > 0) {
  console.warn(`[WARN] ${parity.mismatches.length} value mismatch(es) — sync will apply DESIGN.md values.`);
  parity.mismatches.slice(0, 5).forEach((m) => console.warn(`  ${m}`));
}

console.log('Generating tokens.css (structure preserved from template)...');
const css = generateTokensCss(markdown, templateCss);

writeIfChanged(aiomeTokensPath, css);

if (fs.existsSync(nurtureUiDir)) {
  writeIfChanged(nurtureTokensPath, css);
} else {
  console.log(`\n[INFO] Skipped Nurture UI sync.`);
  console.log(`       Path not found: ${nurtureUiDir}`);
  console.log(`       Set NURTURE_UI_DIR to override the destination.\n`);
}
