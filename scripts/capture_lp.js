import puppeteer from 'puppeteer';
import path from 'path';

(async () => {
  const browser = await puppeteer.launch({ headless: 'new' });
  const page = await browser.newPage();
  await page.setViewport({ width: 1440, height: 900 });
  await page.goto('http://localhost:5174', { waitUntil: 'networkidle0' });
  
  // Wait for animations
  await new Promise(r => setTimeout(r, 2000));
  
  const screenshotPath = '/Users/motista/.gemini/antigravity/brain/320e5eb9-5322-4920-82b3-56c86c9a5b75/artifacts/lp_screenshot.png';
  await page.screenshot({ path: screenshotPath, fullPage: true });
  
  await browser.close();
  console.log('Screenshot saved to ' + screenshotPath);
})();
