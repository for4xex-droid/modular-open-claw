/**
 * Aiome Landing Page - Interactive Terminal
 * Progressive enhancement: 
 * - If JS is disabled, static content is displayed.
 * - If JS is enabled, the static content is hidden (via CSS .js-enabled) and this script draws the animation.
 */

(function() {
  const terminalContent = document.getElementById('terminal-content');
  const terminalCta = document.getElementById('terminal-cta');
  
  if (!terminalContent) return;

  const animationLines = [
    { type: 'cmd', text: 'git clone https://github.com/motivationstudio-llc/aiome' },
    { type: 'cmd', text: 'cd aiome' },
    { type: 'cmd', text: 'docker compose -f docker-compose.quickstart.yml up -d' },
    { type: 'success', text: '✓ All services started successfully' },
    { type: 'output', text: '→ Open http://localhost:1420' }
  ];

  let isAnimated = false;
  
  // Creates a new line container
  function createLine() {
    const line = document.createElement('div');
    line.className = 'term-line';
    return line;
  }

  function typeText(element, text, speed, callback) {
    let i = 0;
    // Add cursor
    const cursor = document.createElement('span');
    cursor.className = 'term-cursor';
    element.appendChild(cursor);

    function type() {
      if (i < text.length) {
        // Insert text before cursor
        const textNode = document.createTextNode(text.charAt(i));
        element.insertBefore(textNode, cursor);
        i++;
        setTimeout(type, speed + (Math.random() * 20)); // slight randomness
      } else {
        cursor.remove();
        if (callback) callback();
      }
    }
    type();
  }

  function runTerminalAnimation() {
    if (isAnimated) return;
    isAnimated = true;

    // Clear static fallback (which is already hidden by CSS, but good to clean up DOM)
    // Wait, let's keep the DOM clean. We will append new animated lines.
    const staticLines = terminalContent.querySelectorAll('.term-static');
    staticLines.forEach(l => l.style.display = 'none');

    let currentLineIndex = 0;

    function processNextLine() {
      if (currentLineIndex >= animationLines.length) {
        // Animation complete
        if (terminalCta) terminalCta.classList.add('visible');
        return;
      }

      const lineData = animationLines[currentLineIndex];
      const lineEl = createLine();
      terminalContent.appendChild(lineEl);

      if (lineData.type === 'cmd') {
        const prompt = document.createElement('span');
        prompt.className = 'term-prompt';
        prompt.textContent = '$';
        lineEl.appendChild(prompt);

        const cmd = document.createElement('span');
        cmd.className = 'term-cmd';
        lineEl.appendChild(cmd);

        typeText(cmd, lineData.text, 30, () => {
          setTimeout(() => {
            currentLineIndex++;
            processNextLine();
          }, 400); // Wait before next command
        });
      } else if (lineData.type === 'success') {
        const span = document.createElement('span');
        span.className = 'term-success';
        span.textContent = lineData.text;
        lineEl.appendChild(span);
        
        setTimeout(() => {
          currentLineIndex++;
          processNextLine();
        }, 300);
      } else if (lineData.type === 'output') {
        const span = document.createElement('span');
        span.className = 'term-output';
        span.textContent = lineData.text;
        lineEl.appendChild(span);

        setTimeout(() => {
          currentLineIndex++;
          processNextLine();
        }, 100);
      }
    }

    // Start after short delay
    setTimeout(processNextLine, 500);
  }

  // Intersection Observer to trigger when visible
  const observer = new IntersectionObserver((entries) => {
    entries.forEach(entry => {
      if (entry.isIntersecting) {
        runTerminalAnimation();
        observer.unobserve(entry.target);
      }
    });
  }, { threshold: 0.5 });

  observer.observe(terminalContent);

})();
