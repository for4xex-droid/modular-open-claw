<!-- 
  Source: VoltAgent/awesome-design-md (MIT License)
  Usage: Inspiration reference only. Do NOT apply to Aiome Management Console.
  For Aiome UI, use: apps/management-console/DESIGN.md
-->

# Spotify — Prompt Guide

詳細は `spotify.design.md` を参照（オンデマンド）。

## Key Colors

| Role | HEX |
|------|-----|
| Near Black (deepest bg) | `#121212` |
| Dark Surface (cards) | `#181818` |
| Mid Dark (buttons) | `#1f1f1f` |
| Spotify Green (accent) | `#1ed760` |
| White (primary text) | `#ffffff` |
| Silver (secondary) | `#b3b3b3` |
| Border Gray | `#4d4d4d` |
| Light Border | `#7c7c7c` |
| Negative Red | `#f3727f` |
| Warning Orange | `#ffa42b` |
| Announcement Blue | `#539df5` |

## Typography Essentials

- **Title**: `SpotifyMixUITitle` (CircularSp family + global script fallbacks)
- **UI/Body**: `SpotifyMixUI` (same fallback stack)
- **Weights**: 700 (emphasis/nav) and 400 (body) binary; 600 sparingly
- **Buttons**: uppercase + letter-spacing 1.4px–2px; compact range 10px–24px

## 9. Agent Prompt Guide

### Quick Color Reference
- Background: Near Black (`#121212`)
- Surface: Dark Card (`#181818`)
- Text: White (`#ffffff`)
- Secondary text: Silver (`#b3b3b3`)
- Accent: Spotify Green (`#1ed760`)
- Border: `#4d4d4d`
- Error: Negative Red (`#f3727f`)

### Example Component Prompts
- "Create a dark card: #181818 background, 8px radius. Title at 16px SpotifyMixUI weight 700, white text. Subtitle at 14px weight 400, #b3b3b3. Shadow rgba(0,0,0,0.3) 0px 8px 8px on hover."
- "Design a pill button: #1f1f1f background, white text, 9999px radius, 8px 16px padding. 14px SpotifyMixUI weight 700, uppercase, letter-spacing 1.4px."
- "Build a circular play button: Spotify Green (#1ed760) background, #000000 icon, 50% radius, 12px padding."
- "Create search input: #1f1f1f background, white text, 500px radius, 12px 48px padding. Inset border: rgb(124,124,124) 0px 0px 0px 1px inset."
- "Design navigation sidebar: #121212 background. Active items: 14px weight 700, white. Inactive: 14px weight 400, #b3b3b3."

### Iteration Guide
1. Start with #121212 — everything lives in near-black darkness
2. Spotify Green for functional highlights only (play, active, CTA)
3. Pill everything — 500px for large, 9999px for small, 50% for circular
4. Uppercase + wide tracking on buttons — the systematic label voice
5. Heavy shadows (0.3–0.5 opacity) for elevation — light shadows are invisible on dark
6. Album art provides all the color — the UI stays achromatic
