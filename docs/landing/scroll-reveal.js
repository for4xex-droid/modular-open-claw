/**
 * Aiome Landing Page - Scroll Reveal Animations
 * Phase A-β: Adds IntersectionObserver for smooth fade-in/slide-up animations.
 */

(function() {
  // Elements to observe
  const selectors = [
    '.feature-card',
    '.step-item',
    '.console-preview-container',
    '.eco-node',
    '.usecase-card'
  ];

  // Callback for observer
  const handleIntersect = (entries, observer) => {
    entries.forEach(entry => {
      if (entry.isIntersecting) {
        entry.target.classList.add('revealed');
        // Stop observing once revealed
        observer.unobserve(entry.target);
      }
    });
  };

  // Create observer
  const observer = new IntersectionObserver(handleIntersect, {
    root: null,
    rootMargin: '0px',
    threshold: 0.15 // Trigger when 15% visible
  });

  // Start observing on DOMContentLoaded
  document.addEventListener('DOMContentLoaded', () => {
    const elements = document.querySelectorAll(selectors.join(', '));
    elements.forEach(el => {
      // Add base class for CSS transitions
      el.classList.add('reveal-item');
      observer.observe(el);
    });
  });
})();
