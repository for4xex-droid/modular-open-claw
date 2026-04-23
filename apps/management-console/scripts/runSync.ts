import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { generateTokensCss } from './syncDesignTokens';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const rootDir = path.resolve(__dirname, '..');
const designMdPath = path.resolve(rootDir, 'DESIGN.md');
const aiomeTokensPath = path.resolve(rootDir, 'src/styles/tokens.css');

// Nurture UI sync path (cross-repository)
const nurtureFallbackPath = path.resolve(rootDir, '../../../Project-Nurture/apps/nurture-ui/src');
const nurtureUiDir = process.env.NURTURE_UI_DIR || nurtureFallbackPath;
const nurtureTokensPath = path.join(nurtureUiDir, 'tokens.css');

console.log('Reading DESIGN.md...');
const markdown = fs.readFileSync(designMdPath, 'utf-8');

console.log('Generating tokens.css...');
const css = generateTokensCss(markdown);

fs.writeFileSync(aiomeTokensPath, css);
console.log(`Wrote tokens to ${aiomeTokensPath}`);

if (fs.existsSync(nurtureUiDir)) {
  fs.writeFileSync(nurtureTokensPath, css);
  console.log(`Wrote tokens to Nurture UI: ${nurtureTokensPath}`);
} else {
  console.log(`\n[INFO] Skipped Nurture UI sync.`);
  console.log(`       Path not found: ${nurtureUiDir}`);
  console.log(`       Set NURTURE_UI_DIR to override the destination.\n`);
}
