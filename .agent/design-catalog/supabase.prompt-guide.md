<!-- 
  Source: VoltAgent/awesome-design-md (MIT License)
  Usage: Inspiration reference only. Do NOT apply to Aiome Management Console.
  For Aiome UI, use: apps/management-console/DESIGN.md
-->

# Supabase — Prompt Guide

詳細は `supabase.design.md` を参照（オンデマンド）。

## Key Colors

| Role | HEX |
|------|-----|
| Page Background | `#171717` |
| Button / Deepest Surface | `#0f0f0f` |
| Primary Text | `#fafafa` |
| Secondary Text | `#b4b4b4` |
| Muted Text | `#898989` |
| Supabase Green (brand) | `#3ecf8e` |
| Green Link | `#00c573` |
| Green Border | `rgba(62, 207, 142, 0.3)` |
| Border Subtle | `#242424` |
| Border Standard | `#2e2e2e` |
| Border Prominent | `#363636` |
| Glass Dark | `rgba(41, 41, 41, 0.84)` |

## Typography Essentials

- **Primary**: `Circular` (fallback: Helvetica Neue, Helvetica, Arial)
- **Monospace**: `Source Code Pro` (fallback: Office Code Pro, Menlo) — uppercase labels, 1.2px letter-spacing
- **Weights**: 400 (default for nearly all text); 500 only for nav links and button labels; no bold 700
- **Hero**: 72px / weight 400 / line-height 1.00 — signature compressed density

## 9. Agent Prompt Guide

### Quick Color Reference
- Background: `#0f0f0f` (button), `#171717` (page)
- Text: `#fafafa` (primary), `#b4b4b4` (secondary), `#898989` (muted)
- Brand green: `#3ecf8e` (brand), `#00c573` (links)
- Borders: `#242424` (subtle), `#2e2e2e` (standard), `#363636` (prominent)
- Green border: `rgba(62, 207, 142, 0.3)` (accent)

### Example Component Prompts
- "Create a hero section on #171717 background. Headline at 72px Circular weight 400, line-height 1.00, #fafafa text. Sub-text at 16px Circular weight 400, line-height 1.50, #b4b4b4. Pill CTA button (#0f0f0f bg, #fafafa text, 9999px radius, 8px 32px padding, 1px solid #fafafa border)."
- "Design a feature card: #171717 background, 1px solid #2e2e2e border, 16px radius. Title at 24px Circular weight 400, letter-spacing -0.16px. Body at 14px weight 400, #898989 text."
- "Build navigation bar: #171717 background. Circular 14px weight 500 for links, #fafafa text. Supabase logo with green icon left-aligned. Green pill CTA 'Start your project' right-aligned."
- "Create a technical label: Source Code Pro 12px, uppercase, letter-spacing 1.2px, #898989 text."
- "Design a framework logo grid: 6-column layout on dark, grayscale logos at 60% opacity, 1px solid #2e2e2e border between sections."

### Iteration Guide
1. Start with #171717 background — everything is dark-mode-native
2. Green is the brand identity marker — use it for links, logo, and accent borders only
3. Depth comes from borders (#242424 → #2e2e2e → #363636), not shadows
4. Weight 400 is the default for everything — 500 only for interactive elements
5. Hero line-height of 1.00 is the signature typographic move
6. Pill (9999px) for primary actions, 6px for secondary, 8-16px for cards
7. HSL with alpha channels creates the sophisticated translucent layering
