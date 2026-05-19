import puppeteer from 'puppeteer';
import path from 'path';

(async () => {
  const browser = await puppeteer.launch({ headless: 'new' });
  const page = await browser.newPage();
  await page.setViewport({ width: 1440, height: 900 });
  
  // Capture Aiome UI
  await page.goto('http://localhost:1420', { waitUntil: 'networkidle0' });
  await new Promise(r => setTimeout(r, 2000));
  await page.screenshot({ path: '/Users/motista/.gemini/antigravity/brain/320e5eb9-5322-4920-82b3-56c86c9a5b75/artifacts/aiome_ui_screenshot.png' });
  console.log('Aiome UI captured');
  
  // Capture Nurture UI
  await page.goto('http://localhost:5175', { waitUntil: 'networkidle0' });
  await new Promise(r => setTimeout(r, 2000));
  await page.screenshot({ path: '/Users/motista/.gemini/antigravity/brain/320e5eb9-5322-4920-82b3-56c86c9a5b75/artifacts/nurture_ui_screenshot.png' });
  console.log('Nurture UI captured');

  await browser.close();
})();
