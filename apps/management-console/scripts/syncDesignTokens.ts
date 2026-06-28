/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { parse } from 'yaml';

export function generateTokensCss(markdown: string): string {
  const frontmatterMatch = markdown.match(/^---\r?\n([\s\S]*?)\r?\n---/);
  if (!frontmatterMatch) {
    throw new Error('No YAML frontmatter found in DESIGN.md');
  }

  let parsed: any;
  try {
    parsed = parse(frontmatterMatch[1]);
  } catch (err: any) {
    throw new Error(`Failed to parse YAML: ${err.message}`);
  }
  
  let cssVars = '';
  
  if (parsed && typeof parsed === 'object') {
    for (const [category, tokens] of Object.entries(parsed)) {
      if (typeof tokens === 'object' && tokens !== null) {
        for (const [key, val] of Object.entries(tokens)) {
          cssVars += `  --${key}: ${val};\n`;
        }
      }
    }
  }

  return `/* 
 * Aiome Design System - Tokens
 * Auto-generated from DESIGN.md. DO NOT EDIT DIRECTLY. 
 */
:root {
${cssVars}}
`;
}
