<!-- 
  Source: VoltAgent/awesome-design-md (MIT License)
  Usage: Inspiration reference only. Do NOT apply to Aiome Management Console.
  For Aiome UI, use: apps/management-console/DESIGN.md
-->

# Framer — Prompt Guide

詳細は `framer.design.md` を参照（オンデマンド）。

## Key Colors

| Role | HEX |
|------|-----|
| Void Black (background) | `#000000` |
| Pure White (text) | `#ffffff` |
| Framer Blue (accent) | `#0099ff` |
| Muted Silver (body) | `#a6a6a6` |
| Near Black (elevation) | `#090909` |
| Frosted White | `rgba(255,255,255,0.1)` |
| Blue Glow (ring) | `rgba(0,153,255,0.15)` |
| Ghost White (tertiary) | `rgba(255,255,255,0.6)` |

## Typography Essentials

- **Display**: `GT Walsheim Framer Medium` / `GT Walsheim Medium` — weight 500 only; extreme tracking (-5.5px at 110px)
- **Body/UI**: `Inter Variable` / `Inter` — OpenType `cv01`, `cv05`, `cv09`, `cv11`, `ss03`, `ss07`
- **Accent**: `Mona Sans` (weight 100 display); **Mono**: `Azeret Mono`; **Micro**: `Open Runde`
- **Display line-height**: as tight as 0.85 at hero sizes

## 9. Agent Prompt Guide

### Quick Color Reference
- Primary Background: Void Black (`#000000`)
- Primary Text: Pure White (`#ffffff`)
- Accent/CTA: Framer Blue (`#0099ff`)
- Secondary Text: Muted Silver (`#a6a6a6`)
- Frosted Surface: Translucent White (`rgba(255, 255, 255, 0.1)`)
- Elevation Ring: Blue Glow (`rgba(0, 153, 255, 0.15)`)

### Example Component Prompts
- "Create a hero section on pure black background with 110px GT Walsheim heading in white, letter-spacing -5.5px, line-height 0.85, and a pill-shaped white CTA button (100px radius) with black text"
- "Design a feature card on black background with a 1px Framer Blue ring shadow border (rgba(0,153,255,0.15)), 12px border-radius, white heading in Inter at 22px weight 700, and muted silver (a6a6a6) body text"
- "Build a navigation bar with black background, white Inter text links at 15px, and a frosted pill button (rgba(255,255,255,0.1) background, 40px radius) as the CTA"
- "Create a product showcase section with a full-width screenshot embedded on black, 10px border-radius, subtle multi-layer shadow (white 0.5px top highlight + rgba(0,0,0,0.25) 30px ambient)"
- "Design a pricing card using pure black surface, Framer Blue (#0099ff) accent for the selected plan border, white text hierarchy (24px Inter bold heading, 14px regular body), and a solid white pill CTA button"

### Iteration Guide
When refining existing screens generated with this design system:
1. Focus on ONE component at a time — the dark canvas makes each element precious
2. Always verify letter-spacing on GT Walsheim headings — the extreme negative tracking is non-negotiable
3. Check that Framer Blue appears ONLY on interactive elements — never as decorative background or text color for non-links
4. Ensure all buttons are pill-shaped — any squared corner immediately breaks the Framer aesthetic
5. Test frosted glass surfaces by checking they have exactly `rgba(255, 255, 255, 0.1)` — too opaque looks like a bug, too transparent disappears
