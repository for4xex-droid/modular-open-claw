import { generateTokensCss } from './syncDesignTokens';

describe('generateTokensCss', () => {
  it('should generate exact CSS variables without prefixing category names', () => {
    const markdown = `---
colors:
  bg-primary: "#0b0b0f"
  accent-cyan-glass: "rgba(0, 242, 255, 0.08)"
spacing:
  space-sm: "1rem"
  layout-sidebar-width: "280px"
motion:
  timing-soft: "cubic-bezier(0.16, 1, 0.3, 1)"
  speed-fast: "0.15s"
---
# Aiome Design System
`;

    const expectedCss = `/* 
 * Aiome Design System - Tokens
 * Auto-generated from DESIGN.md. DO NOT EDIT DIRECTLY. 
 */
:root {
  --bg-primary: #0b0b0f;
  --accent-cyan-glass: rgba(0, 242, 255, 0.08);
  --space-sm: 1rem;
  --layout-sidebar-width: 280px;
  --timing-soft: cubic-bezier(0.16, 1, 0.3, 1);
  --speed-fast: 0.15s;
}
`;
    expect(generateTokensCss(markdown)).toBe(expectedCss);
  });

  it('should handle CRLF, inline comments, and flexible indentation robustly', () => {
    const markdown = `---\r\ncolors:\r\n    bg-primary: "#0b0b0f" # The primary background\r\n    accent-cyan-glass: 'rgba(0, 242, 255, 0.08)'\r\n---\r\n# Body`;

    const expectedCss = `/* 
 * Aiome Design System - Tokens
 * Auto-generated from DESIGN.md. DO NOT EDIT DIRECTLY. 
 */
:root {
  --bg-primary: #0b0b0f;
  --accent-cyan-glass: rgba(0, 242, 255, 0.08);
}
`;
    expect(generateTokensCss(markdown)).toBe(expectedCss);
  });

  it('should throw error if frontmatter is missing', () => {
    const markdown = `# Just text`;
    expect(() => generateTokensCss(markdown)).toThrow('No YAML frontmatter found in DESIGN.md');
  });
});
