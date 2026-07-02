<!-- 
  Source: VoltAgent/awesome-design-md (MIT License)
  Usage: Inspiration reference only. Do NOT apply to Aiome Management Console.
  For Aiome UI, use: apps/management-console/DESIGN.md
-->

# Vercel — Prompt Guide

詳細は `vercel.design.md` を参照（オンデマンド）。

## Key Colors

| Role | HEX |
|------|-----|
| Vercel Black (text/CTA) | `#171717` |
| Pure White (background) | `#ffffff` |
| Gray 600 (body) | `#4d4d4d` |
| Gray 100 (borders) | `#ebebeb` |
| Gray 50 (inner ring) | `#fafafa` |
| Link Blue | `#0072f5` |
| Focus Blue | `hsla(212, 100%, 48%, 1)` |
| Ship Red (workflow) | `#ff5b4f` |
| Preview Pink (workflow) | `#de1d8d` |
| Develop Blue (workflow) | `#0a72ef` |
| Badge Blue Bg / Text | `#ebf5ff` / `#0068d6` |
| Shadow Border | `rgba(0,0,0,0.08) 0px 0px 0px 1px` |

## Typography Essentials

- **Primary**: `Geist` (fallback: Arial, system emoji fonts) — OpenType `"liga"` globally; `"tnum"` for tabular captions
- **Monospace**: `Geist Mono` (fallback: ui-monospace, SFMono-Regular, Roboto Mono)
- **Weights**: 400 (body), 500 (UI/interactive), 600 (headings); 700 only for micro-badges
- **Display tracking**: -2.4px to -2.88px at 48px; -1.28px at 32px, -0.96px at 24px

## 9. Agent Prompt Guide

### Quick Color Reference
- Primary CTA: Vercel Black (`#171717`)
- Background: Pure White (`#ffffff`)
- Heading text: Vercel Black (`#171717`)
- Body text: Gray 600 (`#4d4d4d`)
- Border (shadow): `rgba(0, 0, 0, 0.08) 0px 0px 0px 1px`
- Link: Link Blue (`#0072f5`)
- Focus ring: Focus Blue (`hsla(212, 100%, 48%, 1)`)

### Example Component Prompts
- "Create a hero section on white background. Headline at 48px Geist weight 600, line-height 1.00, letter-spacing -2.4px, color #171717. Subtitle at 20px Geist weight 400, line-height 1.80, color #4d4d4d. Dark CTA button (#171717, 6px radius, 8px 16px padding) and ghost button (white, shadow-border rgba(0,0,0,0.08) 0px 0px 0px 1px, 6px radius)."
- "Design a card: white background, no CSS border. Use shadow stack: rgba(0,0,0,0.08) 0px 0px 0px 1px, rgba(0,0,0,0.04) 0px 2px 2px, #fafafa 0px 0px 0px 1px. Radius 8px. Title at 24px Geist weight 600, letter-spacing -0.96px. Body at 16px weight 400, #4d4d4d."
- "Build a pill badge: #ebf5ff background, #0068d6 text, 9999px radius, 0px 10px padding, 12px Geist weight 500."
- "Create navigation: white sticky header. Geist 14px weight 500 for links, #171717 text. Dark pill CTA 'Start Deploying' right-aligned. Shadow-border on bottom: rgba(0,0,0,0.08) 0px 0px 0px 1px."
- "Design a workflow section showing three steps: Develop (text color #0a72ef), Preview (#de1d8d), Ship (#ff5b4f). Each step: 14px Geist Mono uppercase label + 24px Geist weight 600 title + 16px weight 400 description in #4d4d4d."

### Iteration Guide
1. Always use shadow-as-border instead of CSS border — `0px 0px 0px 1px rgba(0,0,0,0.08)` is the foundation
2. Letter-spacing scales with font size: -2.4px at 48px, -1.28px at 32px, -0.96px at 24px, normal at 14px
3. Three weights only: 400 (read), 500 (interact), 600 (announce)
4. Color is functional, never decorative — workflow colors (Red/Pink/Blue) mark pipeline stages only
5. The inner `#fafafa` ring in card shadows is what gives Vercel cards their subtle inner glow
6. Geist Mono uppercase for technical labels, Geist Sans for everything else
