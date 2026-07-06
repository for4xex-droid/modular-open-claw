/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import fs from 'fs';
import path from 'path';
import {
  diffTokenMaps,
  generateTokensCss,
  normalizeTokenValue,
  parseDesignTokenMap,
} from './syncDesignTokens';

const MINIMAL_TEMPLATE = `/* 
 * Aiome Design System - Tokens
 * Auto-generated from DESIGN.md. DO NOT EDIT DIRECTLY. 
 */
:root {
  --bg-primary: #0b0b0f;
  --accent-cyan-glass: rgba(0, 242, 255, 0.08);
  --space-sm: 1rem;
  --layout-sidebar-width: 280px;
  --timing-soft: cubic-bezier(0.16, 1, 0.3, 1);
  --speed-fast: 0.15s;
}
`;

describe('syncDesignTokens', () => {
  const fixtureMarkdown = `---
colors:
  bg-primary: "#0b0b0f"
  accent-cyan-glass: "rgba(0, 242, 255, 0.08)"
spacing:
  space-sm: "1rem"
  layout-sidebar-width: "280px"
motion:
  timing-soft: "cubic-bezier(0.16, 1, 0.3, 1)"
  speed-fast: "0.15s"
---
# Aiome Design System
`;

  it('should preserve template structure and apply DESIGN.md values', () => {
    const expectedCss = `/* 
 * Aiome Design System - Tokens
 * Auto-generated from DESIGN.md. DO NOT EDIT DIRECTLY. 
 */
:root {
  --bg-primary: #0b0b0f;
  --accent-cyan-glass: rgba(0, 242, 255, 0.08);
  --space-sm: 1rem;
  --layout-sidebar-width: 280px;
  --timing-soft: cubic-bezier(0.16, 1, 0.3, 1);
  --speed-fast: 0.15s;
}
`;
    expect(generateTokensCss(fixtureMarkdown, MINIMAL_TEMPLATE)).toBe(expectedCss);
  });

  it('should strip YAML quotes via normalizeTokenValue', () => {
    expect(normalizeTokenValue('"#0b0b0f"')).toBe('#0b0b0f');
    expect(normalizeTokenValue("'Inter', system-ui")).toBe("'Inter', system-ui");
  });

  it('should throw if frontmatter is missing', () => {
    expect(() => generateTokensCss('# Just text', MINIMAL_TEMPLATE)).toThrow(
      'No YAML frontmatter found in DESIGN.md',
    );
  });

  it('should throw if DESIGN.md is missing a template token', () => {
    const incomplete = `---
colors:
  bg-primary: "#0b0b0f"
---
# x
`;
    expect(() => generateTokensCss(incomplete, MINIMAL_TEMPLATE)).toThrow(
      'DESIGN.md is missing token "accent-cyan-glass"',
    );
  });

  it('should detect value mismatches between DESIGN.md and tokens.css', () => {
    const design = `---
colors:
  bg-primary: "#ffffff"
---
# x
`;
    const css = MINIMAL_TEMPLATE;
    const diff = diffTokenMaps(design, css);
    expect(diff.mismatches.some((m) => m.startsWith('bg-primary:'))).toBe(true);
  });

  it('should round-trip the on-disk DESIGN.md and tokens.css (idempotent sync)', () => {
    const root = path.resolve(__dirname, '..');
    const design = fs.readFileSync(path.join(root, 'DESIGN.md'), 'utf-8');
    const tokensPath = path.join(root, 'src/styles/tokens.css');
    const template = fs.readFileSync(tokensPath, 'utf-8');

    const parity = diffTokenMaps(design, template);
    expect(parity.missingInDesign).toEqual([]);
    expect(parity.missingInCss).toEqual([]);
    expect(parity.mismatches).toEqual([]);

    const regenerated = generateTokensCss(design, template);
    expect(regenerated).toBe(template);
  });

  it('Negative: changing a DESIGN.md value alters output but preserves structure', () => {
    const mutated = fixtureMarkdown.replace('#0b0b0f', '#ff0000');
    const out = generateTokensCss(mutated, MINIMAL_TEMPLATE);
    expect(out).toContain('--bg-primary: #ff0000;');
    expect(out).not.toBe(generateTokensCss(fixtureMarkdown, MINIMAL_TEMPLATE));
    // structure lines unchanged
    expect(out.split('\n')[0]).toBe('/* ');
  });

  it('should append DESIGN.md-only tokens before the closing brace', () => {
    const designWithExtra = fixtureMarkdown.replace(
      'speed-fast: "0.15s"',
      'speed-fast: "0.15s"\n  brand-new-token: "#aabbcc"',
    );
    const out = generateTokensCss(designWithExtra, MINIMAL_TEMPLATE);
    expect(out).toContain('--brand-new-token: #aabbcc;');
    expect(out).toContain('Added from DESIGN.md');
    expect(out.indexOf('--brand-new-token')).toBeLessThan(out.lastIndexOf('}'));
  });

  it('diffTokenMaps reports keys present in DESIGN.md but absent from template', () => {
    const design = `---
colors:
  bg-primary: "#0b0b0f"
  orphan-token: "#123456"
---
# x
`;
    const diff = diffTokenMaps(design, MINIMAL_TEMPLATE);
    expect(diff.missingInCss).toContain('orphan-token');
  });

  it('parseDesignTokenMap flattens all YAML categories', () => {
    const map = parseDesignTokenMap(fixtureMarkdown);
    expect(map.get('bg-primary')).toBe('#0b0b0f');
    expect(map.get('speed-fast')).toBe('0.15s');
    expect(map.size).toBe(6);
  });
});
