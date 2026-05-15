const fs = require('fs');
const assert = require('assert');
const path = require('path');

const castPath = path.join(__dirname, '../assets/quickstart.cast');
const svgPath = path.join(__dirname, '../quickstart-demo.svg');
const htmlPath = path.join(__dirname, '../index.html');

console.log("🚀 Starting Phase B TDD Verification...\n");

try {
  // 1. Check if .cast file exists and is valid JSONL
  console.log("Checking for quickstart.cast...");
  assert.ok(fs.existsSync(castPath), "quickstart.cast is missing.");
  const castContent = fs.readFileSync(castPath, 'utf8').trim().split('\n');
  const header = JSON.parse(castContent[0]);
  assert.strictEqual(header.version, 2, "cast file must be version 2");
  console.log("✅ quickstart.cast exists and is valid.");

  // 2. Check if .svg file exists
  console.log("Checking for quickstart-demo.svg...");
  assert.ok(fs.existsSync(svgPath), "quickstart-demo.svg is missing.");
  const svgContent = fs.readFileSync(svgPath, 'utf8');
  assert.ok(svgContent.includes('<svg'), "File must be a valid SVG.");
  console.log("✅ quickstart-demo.svg exists and is valid.");

  // 3. Check HTML integration
  console.log("Checking index.html for SVG integration...");
  const htmlContent = fs.readFileSync(htmlPath, 'utf8');
  assert.ok(htmlContent.includes('src="quickstart-demo.svg"'), "HTML must reference quickstart-demo.svg");
  assert.ok(!htmlContent.includes('class="terminal-window"').valueOf() || !htmlContent.includes('<span class="token command">'), "Old terminal HTML should be removed.");
  console.log("✅ index.html is correctly updated.");

  console.log("\n🎉 All tests passed! (GREEN)");
} catch (error) {
  console.error("\n❌ Test Failed! (RED)");
  console.error(error.message);
  process.exit(1);
}
