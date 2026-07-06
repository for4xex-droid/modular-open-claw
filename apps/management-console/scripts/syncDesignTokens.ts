/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { parse } from 'yaml';

const TOKEN_LINE_RE = /^\s*--([a-z0-9-]+):\s*.+;\s*$/;

/** Strip YAML string quotes so CSS values are emitted correctly. */
export function normalizeTokenValue(raw: unknown): string {
  const val = String(raw ?? '').trim();
  if (
    (val.startsWith('"') && val.endsWith('"')) ||
    (val.startsWith("'") && val.endsWith("'"))
  ) {
    return val.slice(1, -1);
  }
  return val;
}

/** Flatten all YAML frontmatter categories into a single key → value map. */
export function parseDesignTokenMap(markdown: string): Map<string, string> {
  const frontmatterMatch = markdown.match(/^---\r?\n([\s\S]*?)\r?\n---/);
  if (!frontmatterMatch) {
    throw new Error('No YAML frontmatter found in DESIGN.md');
  }

  let parsed: Record<string, Record<string, unknown>>;
  try {
    parsed = parse(frontmatterMatch[1]) as Record<string, Record<string, unknown>>;
  } catch (err: unknown) {
    const message = err instanceof Error ? err.message : String(err);
    throw new Error(`Failed to parse YAML: ${message}`);
  }

  const map = new Map<string, string>();
  if (parsed && typeof parsed === 'object') {
    for (const tokens of Object.values(parsed)) {
      if (tokens && typeof tokens === 'object') {
        for (const [key, val] of Object.entries(tokens)) {
          map.set(key, normalizeTokenValue(val));
        }
      }
    }
  }
  return map;
}

/**
 * Regenerate tokens.css by applying DESIGN.md values onto an existing template.
 * Preserves section comments, ordering, and header — only `--key: value` lines change.
 *
 * U6-8(5) follow-up: fixes flat-output regression from the original generator.
 */
export function generateTokensCss(markdown: string, templateCss: string): string {
  const tokenMap = parseDesignTokenMap(markdown);
  const seen = new Set<string>();
  const outLines: string[] = [];

  for (const line of templateCss.split('\n')) {
    const match = line.match(TOKEN_LINE_RE);
    if (!match) {
      outLines.push(line);
      continue;
    }

    const key = match[1];
    seen.add(key);
    const value = tokenMap.get(key);
    if (value === undefined) {
      throw new Error(
        `DESIGN.md is missing token "${key}" required by tokens.css template`,
      );
    }
    outLines.push(`  --${key}: ${value};`);
  }

  const extras = [...tokenMap.entries()].filter(([key]) => !seen.has(key));
  if (extras.length > 0) {
    const closeIdx = outLines.findIndex((l) => l.trim() === '}');
    const extraBlock = [
      '',
      '  /* ── Added from DESIGN.md (not yet in template sections) ── */',
      ...extras.map(([key, value]) => `  --${key}: ${value};`),
    ];
    if (closeIdx >= 0) {
      outLines.splice(closeIdx, 0, ...extraBlock);
    } else {
      outLines.push(...extraBlock, '}');
    }
  }

  return outLines.join('\n');
}

/** Compare token values between DESIGN.md and an existing tokens.css file. */
export function diffTokenMaps(
  markdown: string,
  tokensCss: string,
): { missingInDesign: string[]; missingInCss: string[]; mismatches: string[] } {
  const designMap = parseDesignTokenMap(markdown);
  const cssMap = new Map<string, string>();

  for (const line of tokensCss.split('\n')) {
    const m = line.match(/^\s*--([a-z0-9-]+):\s*(.+);\s*$/);
    if (m) cssMap.set(m[1], m[2].trim());
  }

  const missingInDesign: string[] = [];
  const missingInCss: string[] = [];
  const mismatches: string[] = [];

  for (const key of cssMap.keys()) {
    if (!designMap.has(key)) missingInDesign.push(key);
    else if (designMap.get(key) !== cssMap.get(key)) {
      mismatches.push(`${key}: css=${cssMap.get(key)} design=${designMap.get(key)}`);
    }
  }
  for (const key of designMap.keys()) {
    if (!cssMap.has(key)) missingInCss.push(key);
  }

  return { missingInDesign, missingInCss, mismatches };
}
