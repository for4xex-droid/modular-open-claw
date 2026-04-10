import '@testing-library/jest-dom';

// Fix for React / Framer Motion usage in JSDOM which doesn't implement window.scrollTo
window.scrollTo = jest.fn();
