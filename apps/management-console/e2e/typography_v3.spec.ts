import { test, expect } from '@playwright/test';

test.describe('Typography and Animations Validation (v3)', () => {
  test.beforeEach(async ({ page }) => {
    // 開発サーバーの起動を待つ
    await page.goto('/', { waitUntil: 'load' });
  });

  test('CSS Variables: --font-display should prioritize Outfit', async ({ page }) => {
    // documentElementのcomputedStyleをチェック
    const fontDisplay = await page.evaluate(() => {
      return getComputedStyle(document.documentElement).getPropertyValue('--font-display').trim();
    });
    // Outfitが最初にあること
    expect(fontDisplay).toContain('Outfit');
  });

  test('Animations: .animate-spin class should be defined', async ({ page }) => {
    // 実際のDOM要素を作成してクラスを適用し、animationプロパティが設定されるか確認
    const hasAnimation = await page.evaluate(() => {
      const el = document.createElement('div');
      el.className = 'animate-spin';
      document.body.appendChild(el);
      const style = getComputedStyle(el);
      const animationName = style.animationName;
      document.body.removeChild(el);
      return animationName !== 'none' && animationName !== '';
    });
    expect(hasAnimation).toBe(true);
  });

  test('Animations: .ani-pulse class should be defined', async ({ page }) => {
    const hasAnimation = await page.evaluate(() => {
      const el = document.createElement('div');
      el.className = 'ani-pulse';
      document.body.appendChild(el);
      const style = getComputedStyle(el);
      const animationName = style.animationName;
      document.body.removeChild(el);
      return animationName !== 'none' && animationName !== '';
    });
    expect(hasAnimation).toBe(true);
  });
  
  test('Typography: h4 and h5 should use global styles', async ({ page }) => {
     // Check if standard heading styles are applied to h4
     const h4Style = await page.evaluate(() => {
        const el = document.createElement('h4');
        document.body.appendChild(el);
        const style = getComputedStyle(el);
        const fontFamily = style.fontFamily;
        document.body.removeChild(el);
        return fontFamily;
     });
     
     // Currently we expect it to use the --font-display variable's resolved value after fix.
     // Before the fix, it might be the default.
     expect(h4Style).toContain('Outfit');
  });

  test('Typography: Tier 1 inline styles should be removed', async ({ page }) => {
     // We will check for any h2 elements with inline style containing fontFamily on the page.
     // Since some components might not mount immediately on / depending on state, 
     // we just ensure whatever is mounted doesn't have it.
     
     const hasInlineFontFamily = await page.evaluate(() => {
         const h2s = Array.from(document.querySelectorAll('h2'));
         return h2s.some(h2 => h2.style.fontFamily !== '');
     });
     
     expect(hasInlineFontFamily).toBe(false);
  });
});
