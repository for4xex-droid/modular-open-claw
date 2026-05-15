const fs = require('fs');
const path = require('path');

const castPath = path.join(__dirname, '../assets/quickstart.cast');

const header = {
  version: 2,
  width: 80,
  height: 24,
  timestamp: Math.floor(Date.now() / 1000),
  env: { TERM: "xterm-256color" }
};

let time = 0;
const frames = [];

function addText(text, delay = 0.1) {
  frames.push(`[${time.toFixed(4)}, "o", ${JSON.stringify(text)}]`);
  time += delay;
}

// Sequence
addText("$ ", 0.5);
addText("g", 0.1); addText("i", 0.1); addText("t", 0.1); addText(" ", 0.1);
addText("c", 0.1); addText("l", 0.1); addText("o", 0.1); addText("n", 0.1); addText("e", 0.1); addText(" ", 0.1);
addText("h", 0.05); addText("t", 0.05); addText("t", 0.05); addText("p", 0.05); addText("s", 0.05); addText(":", 0.05);
addText("/", 0.05); addText("/", 0.05); addText("g", 0.05); addText("i", 0.05); addText("t", 0.05); addText("h", 0.05);
addText("u", 0.05); addText("b", 0.05); addText(".", 0.05); addText("c", 0.05); addText("o", 0.05); addText("m", 0.05);
addText("/", 0.05); addText("m", 0.05); addText("o", 0.05); addText("t", 0.05); addText("i", 0.05); addText("v", 0.05);
addText("a", 0.05); addText("t", 0.05); addText("i", 0.05); addText("o", 0.05); addText("n", 0.05); addText("s", 0.05);
addText("t", 0.05); addText("u", 0.05); addText("d", 0.05); addText("i", 0.05); addText("o", 0.05); addText("-", 0.05);
addText("l", 0.05); addText("l", 0.05); addText("c", 0.05); addText("/", 0.05); addText("a", 0.05); addText("i", 0.05);
addText("o", 0.05); addText("m", 0.05); addText("e", 0.05); addText(".", 0.05); addText("g", 0.05); addText("i", 0.05);
addText("t", 0.2); addText("\r\n", 0.5);

addText("\x1b[34mCloning into 'aiome'...\x1b[0m\r\n", 0.8);
addText("remote: Enumerating objects: 1234, done.\r\n", 0.2);
addText("remote: Counting objects: 100% (1234/1234), done.\r\n", 0.2);
addText("Receiving objects: 100% (1234/1234), 2.45 MiB | 3.50 MiB/s, done.\r\n", 0.5);

addText("$ ", 0.5);
addText("c", 0.1); addText("d", 0.1); addText(" ", 0.1); addText("a", 0.1); addText("i", 0.1); addText("o", 0.1); addText("m", 0.1); addText("e", 0.2); addText("\r\n", 0.5);

addText("$ ", 0.5);
addText("d", 0.1); addText("o", 0.1); addText("c", 0.1); addText("k", 0.1); addText("e", 0.1); addText("r", 0.1); addText(" ", 0.1);
addText("c", 0.1); addText("o", 0.1); addText("m", 0.1); addText("p", 0.1); addText("o", 0.1); addText("s", 0.1); addText("e", 0.1); addText(" ", 0.1);
addText("u", 0.1); addText("p", 0.1); addText(" ", 0.1); addText("-", 0.1); addText("d", 0.2); addText("\r\n", 0.8);

addText("[+] Running 4/4\r\n", 0.3);
addText(" \x1b[32m✔\x1b[0m Network aiome_default       \x1b[32mCreated\x1b[0m\r\n", 0.2);
addText(" \x1b[32m✔\x1b[0m Container aiome-db          \x1b[32mStarted\x1b[0m\r\n", 0.2);
addText(" \x1b[32m✔\x1b[0m Container aiome-api         \x1b[32mStarted\x1b[0m\r\n", 0.2);
addText(" \x1b[32m✔\x1b[0m Container aiome-console     \x1b[32mStarted\x1b[0m\r\n", 1.0);

addText("\r\n\x1b[1;32m🚀 Aiome is running at http://localhost:1420\x1b[0m\r\n", 2.0);

const content = JSON.stringify(header) + '\n' + frames.join('\n') + '\n';
fs.writeFileSync(castPath, content);

console.log("Generated: " + castPath);
