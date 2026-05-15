import { test, expect } from '@playwright/test';

test.describe('Cortex View UI', () => {
  test('has a navigation tab, loads wiki articles and sanitizes markdown', async ({ page }) => {
    // 1. Arrange: Intercept API calls to provide mock Wiki data
    // IMPORTANT: Register the more specific route first to avoid glob collision
    await page.route('**/api/v1/cortex/wiki/article_1', async route => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          id: 'article_1',
          title: 'Test Article 1',
          concepts: ['test', 'concept'],
          backlinks: ['related_concept'],
          source_refs: [],
          version: 1,
          content_hash: 'abc',
          content_md: '# Hello XSS\nThis is a *test* snippet with <script>alert(1)</script>.'
        })
      });
    });

    await page.route('**/api/v1/cortex/wiki', async route => {
      // Only match the exact list endpoint, not sub-paths
      const url = new URL(route.request().url());
      if (url.pathname !== '/api/v1/cortex/wiki') {
        return route.fallback();
      }
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([
          { id: 'article_1', title: 'Test Article 1', concepts: ['test', 'concept'], version: 1, updated_at: new Date().toISOString() },
          { id: 'article_2', title: 'Test Article 2', concepts: ['mock'], version: 1, updated_at: new Date().toISOString() }
        ])
      });
    });

    // 2. Act: Go to dashboard — bypass auth overlay (sessionStorage) and enable advanced view mode (localStorage)
    await page.addInitScript(() => {
      window.sessionStorage.setItem('aiome_secret', 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJhZ2VudC0wMDEiLCJla3ljX3ZlcmlmaWVkIjp0cnVlfQ.mock_signature');
      window.localStorage.setItem('aiome_view_mode', 'advanced');
      window.localStorage.setItem('aiome_onboarding_done', 'true');
      window.localStorage.setItem('aiome_birth_shown', 'true');
    });
    await page.goto('/');

    // The nav item should be visible in the sidebar
    const cortexTab = page.locator('.nav-item').filter({ hasText: /Knowledge Base/i });
    await expect(cortexTab).toBeVisible({ timeout: 5000 });
    await cortexTab.click();

    // 3. Assert List View: The mocked articles should appear after lazy load + fetch
    await expect(page.locator('text=Test Article 1')).toBeVisible({ timeout: 10000 });
    await expect(page.locator('text=Test Article 2')).toBeVisible();

    // 4. Assert Detail View & Sanitization: Click an article to view its markdown
    await page.locator('text=Test Article 1').first().click();

    // The markdown h1 should render correctly
    await expect(page.locator('h1', { hasText: 'Hello XSS' })).toBeVisible({ timeout: 10000 });

    // The raw script tag MUST NOT exist in the DOM (it should be sanitized out by rehype-sanitize)
    const content = await page.content();
    expect(content).not.toContain('<script>alert(1)</script>');
  });
});
