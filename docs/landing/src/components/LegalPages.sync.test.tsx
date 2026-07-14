/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const legalDir = path.resolve(__dirname, '../../../legal');
const legalPagesPath = path.resolve(__dirname, 'LegalPages.tsx');

const PAGE_MAPPINGS = [
  { component: 'PrivacyPage', markdown: 'PRIVACY_POLICY.md' },
  { component: 'TermsPage', markdown: 'TERMS_OF_SERVICE.md' },
  { component: 'TokushohoPage', markdown: 'TOKUSHOHO.md' },
  { component: 'CancellationPage', markdown: 'CANCELLATION_POLICY.md' },
] as const;

function extractLastUpdatedFromMarkdown(filePath: string): string {
  const content = readFileSync(filePath, 'utf-8');
  const match = content.match(/\*\*最終更新日\*\*:\s*(\d{4}-\d{2}-\d{2})/);
  if (!match) {
    throw new Error(`Could not extract lastUpdated from ${filePath}`);
  }
  return match[1];
}

function extractLastUpdatedFromComponent(componentName: string, source: string): string {
  const pattern = new RegExp(
    `export function ${componentName}\\(\\)[\\s\\S]*?lastUpdated="(\\d{4}-\\d{2}-\\d{2})"`
  );
  const match = source.match(pattern);
  if (!match) {
    throw new Error(`Could not extract lastUpdated for ${componentName} in LegalPages.tsx`);
  }
  return match[1];
}

describe('LegalPages sync with docs/legal canonical sources', () => {
  const legalPagesSource = readFileSync(legalPagesPath, 'utf-8');

  it.each(PAGE_MAPPINGS)(
    'lastUpdated in $component matches canonical $markdown',
    ({ component, markdown }) => {
      const markdownPath = path.join(legalDir, markdown);
      const canonicalDate = extractLastUpdatedFromMarkdown(markdownPath);
      const componentDate = extractLastUpdatedFromComponent(component, legalPagesSource);

      expect(componentDate).toBe(canonicalDate);
    }
  );

  it('does not contain sealed legacy refund/charge language', () => {
    expect(legalPagesSource).not.toContain('チャージ代金');
    expect(legalPagesSource).not.toContain('理由の如何を問わず一切の返金');
  });
});
