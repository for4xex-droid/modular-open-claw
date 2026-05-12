const puppeteer = require('puppeteer');

(async () => {
  try {
    console.log('Starting browser...');
    const browser = await puppeteer.launch({
      defaultViewport: { width: 1440, height: 900 }
    });
    const page = await browser.newPage();
    console.log('Navigating to localhost:1420...');
    await page.goto('http://localhost:1420', { waitUntil: 'networkidle0' });
    
    // Wait an additional second for any final animations
    await new Promise(r => setTimeout(r, 1000));
    
    console.log('Capturing screenshot...');
    await page.screenshot({ path: '../docs/landing/console-preview.png' });
    console.log('Screenshot saved to docs/landing/console-preview.png');
    
    await browser.close();
  } catch (err) {
    console.error('Error capturing screenshot:', err);
    process.exit(1);
  }
})();
