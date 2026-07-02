<!-- 
  Source: VoltAgent/awesome-design-md (MIT License)
  Usage: Inspiration reference only. Do NOT apply to Aiome Management Console.
  For Aiome UI, use: apps/management-console/DESIGN.md
-->

# Airbnb — Prompt Guide

詳細は `airbnb.design.md` を参照（オンデマンド）。

## Key Colors

| Role | HEX |
|------|-----|
| Pure White (background) | `#ffffff` |
| Near Black (text) | `#222222` |
| Rausch Red (brand/CTA) | `#ff385c` |
| Deep Rausch (pressed) | `#e00b41` |
| Secondary Gray | `#6a6a6a` |
| Disabled | `rgba(0,0,0,0.24)` |
| Legal Blue | `#428bff` |
| Light Surface (controls) | `#f2f2f2` |
| Luxe Purple | `#460479` |
| Plus Magenta | `#92174d` |
| Card Shadow | `rgba(0,0,0,0.02) 0px 0px 0px 1px, rgba(0,0,0,0.04) 0px 2px 6px, rgba(0,0,0,0.1) 0px 4px 8px` |

## Typography Essentials

- **Primary**: `Airbnb Cereal VF` (fallback: Circular, system-ui) — OpenType `"salt"` on badges/captions
- **Weights**: 500 (UI baseline), 600 (emphasis), 700 (headings); no thin weights for headings
- **Tracking**: -0.18px to -0.44px on headings for cozy, intimate feel

## 9. Agent Prompt Guide

### Quick Color Reference
- Background: Pure White (`#ffffff`)
- Text: Near Black (`#222222`)
- Brand accent: Rausch Red (`#ff385c`)
- Secondary text: `#6a6a6a`
- Disabled: `rgba(0,0,0,0.24)`
- Card border: `rgba(0,0,0,0.02) 0px 0px 0px 1px`
- Card shadow: full three-layer stack
- Button surface: `#f2f2f2`

### Example Component Prompts
- "Create a listing card: white background, 20px radius. Three-layer shadow: rgba(0,0,0,0.02) 0px 0px 0px 1px, rgba(0,0,0,0.04) 0px 2px 6px, rgba(0,0,0,0.1) 0px 4px 8px. Photo area on top (16:10 ratio), details below: 16px Airbnb Cereal VF weight 600 title, 14px weight 400 description in #6a6a6a."
- "Design search bar: white background, full card shadow, 32px radius on container. Search text at 14px Cereal VF weight 400. Red search button (#ff385c, 50% radius, white icon)."
- "Build category pill bar: horizontal scrollable row. Each pill: 14px Cereal VF weight 600, #222222 text, bottom border on active. Circular prev/next arrows (#f2f2f2 bg, 50% radius)."
- "Create a CTA button: #222222 background, white text, 8px radius, 16px Cereal VF weight 500, 0px 24px padding. Hover: brand red accent."
- "Design a heart/wishlist button: transparent background, 50% radius, white heart icon with dark shadow outline."

### Iteration Guide
1. Start with white — the photography provides all the color
2. Rausch Red (#ff385c) is the singular accent — use sparingly for CTAs only
3. Near-black (#222222) for text — the warmth matters
4. Three-layer shadows create natural, warm lift — always use all three layers
5. Generous radius: 8px buttons, 20px cards, 50% controls
6. Cereal VF at 500–700 weight — no thin weights for any heading
7. Photography is hero — every listing card is image-first
