# Aiome Management Console — Design System

Aiome's Management Console is a dark-mode-native command center for an autonomous AI operating system. The visual language draws from deep space aesthetics: an obsidian-black canvas (`var(--bg-dark-obsidian)`, #05070a) where information surfaces through layers of frosted glass and cyan-purple luminescence. Every panel breathes with subtle neural-flow animations, holographic sweeps on hover, and floating micro-interactions that make the interface feel alive — like piloting a spacecraft's bridge.

The design is built on three pillars: **Glassmorphism** (translucent panels with backdrop-filter blur), **Neon Accents** (cyan/purple/rose glow effects), and **Typographic Hierarchy** (Outfit for bold geometric display headings, Inter for readable body text, JetBrains Mono for technical data). The result is a premium, futuristic aesthetic that communicates intelligence, precision, and trust.

**Key Characteristics:**
- Dark-mode-native: `var(--bg-dark-obsidian)` (#05070a) as the deepest canvas, glass layers with `backdrop-filter: blur(40px)`
- Glassmorphism everywhere: panels at `var(--bg-glass-heavy)` (rgba(16,20,28,0.8)) with blur and saturate
- Triple-accent system: Cyan (`var(--accent-cyan)`, #00f2ff), Purple (`var(--accent-purple)`, #bc8cff), Rose (`var(--accent-rose)`, #ff4d94)
- Brand gradient: `linear-gradient(135deg, var(--accent-cyan), var(--accent-purple))` on headings
- Artemis Typography: Outfit (display), Inter (body), JetBrains Mono (code)
- Neon glow effects: `var(--glow-cyan)` = `0 0 25px rgba(0, 242, 255, 0.25)`
- Star-field background with radial gradients and pseudo-element particle effects
- Neural-flow animations: slow gradient cycling across surfaces
- All values referenced via CSS custom properties (var()) — never hardcoded

---

## Color Palette

### Background Surfaces
- **Primary Background** — `var(--bg-primary)` (#05070a): Root background color for dynamic panels.
- **Dark Obsidian** — `var(--bg-dark-obsidian)` (#05070a): The deepest canvas. Near-black with a cool blue undertone. Used for the root background.
- **Sidebar** — `var(--bg-dark-sidebar)` (rgba(10, 12, 18, 0.95)): Translucent dark panel for the navigation sidebar. The 0.95 opacity allows subtle bleed-through.
- **Glass Heavy** — `var(--bg-glass-heavy)` (rgba(16, 20, 28, 0.8)): Primary glass surface for cards, panels, and containers. Heavy enough to read over, transparent enough to feel layered.
- **Glass Light** — `var(--bg-glass-light)` (rgba(255, 255, 255, 0.03)): Ultra-subtle hover state background. A whisper of white on dark surfaces.
- **Deep Glass** — `var(--bg-deep-glass)` (rgba(10, 10, 15, 0.9)): Deep, nearly opaque glass for modals and overlays.
- **Inverse Background** — `var(--bg-inverse)` (#fff): Pure white for contrast elements.

### Brand Accents
- **Cyan** — `var(--accent-cyan)` (#00f2ff): Primary interactive accent. CTAs, active navigation, status indicators. The signature Aiome color.
- **Cyan Glass** — `var(--accent-cyan-glass)` (rgba(0, 242, 255, 0.08)): Subtle tint for active nav backgrounds and hover states.
- **Purple** — `var(--accent-purple)` (#bc8cff): Secondary accent. Used in gradients and complementary highlights.
- **Purple Glass** — `var(--accent-purple-glass)` (rgba(188, 140, 255, 0.08)): Subtle purple tint for secondary emphasis areas.
- **Rose** — `var(--accent-rose)` (#ff4d94): Danger/error/disconnected states. Hot pink for urgency.
- **Emerald** — `var(--accent-emerald)` (#10b981): Success/connected states. Green for health.
- **Amber** — `var(--accent-amber)` (#f59e0b): Warning/paused states. Warm caution.
- **Fuchsia** — `var(--accent-fuchsia)` (#d946ef): Tertiary accent for special highlights.

### Text & Content
- **Primary Text** — `var(--text-primary)` (#f0f2f5): Near-white with a cool cast. Default text color on dark backgrounds.
- **Secondary Text** — `var(--text-secondary)` (#94a3b8): Cool silver-gray for body text, descriptions, metadata.
- **Muted Text** — `var(--text-muted)` (#64748b): Dimmed gray for labels, placeholders, de-emphasized content.
- **Inverse Text** — `var(--text-inverse)` (#000): Pure black text used for extreme high-contrast overlays on cyan, amber, or light elements.

### Borders & Glass
- **Border Glass** — `var(--border-glass)` (rgba(255, 255, 255, 0.08)): Default border on all glass surfaces. Semi-transparent white.
- **Border Glass Bright** — `var(--border-glass-bright)` (rgba(255, 255, 255, 0.15)): Hover state border. Brighter for emphasis.

### Status Colors
- Connected: `var(--accent-emerald)` with glow
- Disconnected: `var(--accent-rose)` with glow
- Paused: `var(--accent-amber)` with glow

---

## Typography

### Font Families (Artemis Design System)
- **Display**: `var(--font-display)` — `'Outfit', 'Artemis Inter', system-ui, sans-serif` — Geometric sans-serif for headings. Bold, wide letter-spacing reinforces precision.
- **Main/Body**: `var(--font-main)` — `'Artemis Inter', 'Inter', system-ui, -apple-system, sans-serif` — Highly legible body text optimized for screens.
- **Monospace**: `var(--font-mono)` — `'JetBrains Mono', 'Fira Code', monospace` — Technical data, code blocks, system labels.

### Hierarchy

| Role | Font | Size | Weight | Letter Spacing | Transform | Notes |
|------|------|------|--------|----------------|-----------|-------|
| Display (h1) | var(--font-display) | 2rem | 900 | 0.04em | uppercase | Hero titles. Gradient fill: cyan→purple |
| Section Title (h2) | var(--font-display) | 1.5rem | 900 | 0.06em | uppercase | `.artemis-heading` class. Gradient + drop-shadow |
| Heading (h3–h5) | var(--font-display) | inherit | 700–600 | 0.03em | — | Sub-section headers |
| Body | var(--font-main) | 0.95rem | 400 | normal | — | Standard reading text |
| Label | var(--font-mono) | 0.75rem | 400 | 0.2em | uppercase | `.artemis-label` — System labels, category headers |
| Status | var(--font-display) | 0.8rem | 800 | 0.1em | uppercase | `.artemis-status` — Badge text |
| Stat Value | var(--font-display) | 2rem | 800 | -0.02em | — | Dashboard metric numbers. Text-shadow glow |
| Code | var(--font-mono) | inherit | 400 | 0.02em | — | Code blocks, technical data |

### Principles
- **Outfit for impact**: All headings use Outfit with wide letter-spacing and `text-transform: uppercase` to maximize geometric distinction from system fonts
- **Inter for reading**: Body text uses Inter (Artemis variant) for long-form readability
- **JetBrains for truth**: All data/metrics/code use monospace to signal precision and trustworthiness
- **Gradient as identity**: h1/h2 headings use `linear-gradient(135deg, var(--accent-cyan), var(--accent-purple))` with `background-clip: text` as the primary brand expression
- **Glow for hierarchy**: Display text includes `filter: drop-shadow(0 0 12px rgba(var(--accent-cyan-rgb), 0.3))` to create depth

---

## Components

### Buttons

**Primary Button**
- Background: `var(--accent-cyan)` (#00f2ff)
- Text: `var(--text-inverse)` (#000) — dark on bright for maximum contrast and readability
- Padding: 0.6rem 1.2rem
- Radius: `var(--radius-sm)` (8px)
- Font-weight: 700
- Hover: `filter: brightness(1.1)` + `box-shadow: 0 0 20px rgba(0, 242, 255, 0.3)`
- Use: Primary CTAs

**Glass Panel Button / Nav Item**
- Background: transparent → `var(--bg-glass-light)` on hover
- Text: `var(--text-secondary)` → `var(--text-primary)` on hover
- Border: `1px solid transparent` → `1px solid var(--border-glass)` on hover
- Radius: `var(--radius-md)` (12px)
- Hover transform: `translateX(6px)` (slide right) + holographic sweep animation
- Active state: `background: rgba(var(--accent-cyan-rgb), 0.08)`, `color: var(--accent-cyan)`, active-bar glow
- Use: Navigation items, sidebar links

**Card Hover Button**
- Border: `1px solid var(--border-glass)`
- Hover: `border-color: var(--accent-cyan)` + `translateY(-4px)` + cyan glow shadow
- Use: Clickable cards, interactive containers

### Cards & Containers

**Stat Card**
- Background: `var(--bg-glass-heavy)`
- Backdrop-filter: `blur(15px)`
- Border: `1px solid var(--border-glass)`
- Radius: `var(--radius-lg)` (20px)
- Padding: `var(--space-lg)` (2rem)
- Hover: `translateY(-6px)` + `var(--shadow-deep)` + border brightens
- Use: Dashboard metrics grid

**Main Panel**
- Background: `var(--bg-glass-heavy)`
- Backdrop-filter: `blur(25px)`
- Border: `1px solid var(--border-glass)`
- Radius: `var(--radius-xl)` (28px)
- Min-height: 400px
- Shadow: `var(--shadow-deep)` — `0 20px 50px rgba(0, 0, 0, 0.5)`
- Use: Primary content panels

**Panel Header**
- Background: `rgba(255, 255, 255, 0.02)` — ⚠️ Not yet tokenized; matches App.css literal
- Border-bottom: `1px solid var(--border-glass)`
- Padding: `var(--space-md) var(--space-lg)`
- Use: Top bar of main panels

**Glass Panel** (utility class)
- Background: `var(--bg-glass-heavy)`
- Backdrop-filter: `blur(20px) saturate(180%)`
- Border: `1px solid var(--border-glass)`
- Shadow: `0 8px 32px 0 rgba(0, 0, 0, 0.37)`

### Inputs & Forms

**Neural Input**
- Background: `rgba(255, 255, 255, 0.02)`
- Border: `1px solid var(--border-glass)`
- Radius: `var(--radius-md)` (12px)
- Color: `var(--text-primary)`
- Padding: 0.75rem 1rem
- Focus: border → `var(--accent-cyan)`, bg → `rgba(255, 255, 255, 0.05)`, glow shadow `0 0 15px rgba(0, 242, 255, 0.1)`
- Use: All text inputs in the system

### Badges & Status

**Status Badge (Connected)**
- Background: `rgba(16, 185, 129, 0.05)`
- Border: `1px solid rgba(16, 185, 129, 0.15)`
- Color: `var(--accent-emerald)`
- Radius: 20px (pill)
- Font: 0.8rem weight 600
- Includes animated status dot with `pulse` keyframe

**Status Badge (Disconnected)**
- Same structure, color shifted to `var(--accent-rose)`

**Status Badge (Paused)**
- Same structure, color shifted to `var(--accent-amber)`

### TreasureBox (Marketplace Card Grid)

> ⚠️ **Tech Debt Notice**: `TreasureBox.css` (279 lines) was converted from Tailwind and contains residual Tailwind-mapping comments (`/* p-6 */`, `/* text-sm */` etc.) and raw `#ffffff` usage. Future refactoring should replace `#ffffff` → `var(--text-primary)` and raw rgba colors → design tokens.

**Container** (`.artemis-treasure-box`)
- Background: `rgba(255, 255, 255, 0.05)` — ⚠️ Should use `var(--bg-glass-light)` or new token. Note: also references `var(--card-bg-light)` which is **undefined everywhere** (ghost variable, immediately overridden)
- Backdrop-filter: `blur(24px)`
- Border: `1px solid rgba(255, 255, 255, 0.1)`
- Radius: `var(--radius-lg)` (20px)
- Shadow: `var(--shadow-deep)`
- Used in: HomePage, BiotopeView

**Item Card** (`.artemis-treasure-item`)
- Background: `rgba(255, 255, 255, 0.05)`
- Border: `1px solid rgba(255, 255, 255, 0.1)`
- Radius: `var(--radius-md)` (12px)
- Hover: bg → `rgba(255, 255, 255, 0.1)`, border → indigo tint `rgba(99, 102, 241, 0.3)`
- Transition: `all 0.3s cubic-bezier(0.4, 0, 0.2, 1)`

**Item Label** (`.artemis-treasure-label`)
- Background: `rgba(99, 102, 241, 0.1)` — indigo tint
- Border: `1px solid rgba(99, 102, 241, 0.2)`
- Font: 0.625rem weight 500, uppercase, `letter-spacing: 0.05em`
- Color: `rgba(165, 180, 252, 1)` — light indigo

**Resonance Overlay Badge**
- Background: `var(--accent-purple)`
- Text: `#ffffff` — ⚠️ Should use `var(--text-primary)`
- Radius: 9999px (full pill)
- Shadow: `0 10px 15px -3px rgba(99, 102, 241, 0.5)` — indigo glow

### Navigation

**Sidebar**
- Width: `var(--layout-sidebar-width)` (280px)
- Background: `var(--bg-dark-sidebar)`
- Backdrop-filter: `blur(40px)`
- Border-right: `1px solid var(--border-glass)`
- Shadow: `10px 0 30px rgba(0, 0, 0, 0.4)` (right-cast shadow)
- Padding: `var(--space-lg) var(--space-md)`

**Brand Mark**
- Font: `var(--font-display)`, 1.6rem, weight 800
- Fill: `linear-gradient(135deg, var(--accent-cyan), var(--accent-purple))` with background-clip
- Glow: `filter: drop-shadow(0 0 10px rgba(var(--accent-cyan-rgb), 0.2))`

**Nav Group Label**
- Font: 0.75rem, uppercase, `letter-spacing: 0.2em`, `var(--text-muted)`

**Active Nav Indicator**
- Left bar: 4px wide, `var(--accent-cyan)`, `border-radius: 4px`
- Glow: `var(--glow-cyan)` — `0 0 25px rgba(0, 242, 255, 0.25)`

---

## Animation & Motion

### Timing Functions
- **Soft** — `var(--timing-soft)`: `cubic-bezier(0.16, 1, 0.3, 1)` — Default easing. Smooth start, gentle settle.
- **Bouncy** — `var(--timing-bouncy)`: `cubic-bezier(0.34, 1.56, 0.64, 1)` — Overshoot for playful interactions.
- **Linear** — `var(--timing-linear)`: `linear` — Constant-speed loops (holographic sweep, spin).

### Speed Scale
- **Fast** — `var(--speed-fast)`: 0.2s — Hover state transitions, focus rings
- **Base** — `var(--speed-base)`: 0.4s — Standard transitions, slide-in animations
- **Slow** — `var(--speed-slow)`: 0.7s — Modal entrances, complex transitions

### Keyframe Catalog

| Animation | Duration | Use | Description |
|-----------|----------|-----|-------------|
| `fadeIn` | var(--speed-base) | Page transitions, lazy-loaded content | Opacity 0→1 |
| `slideInUp` | var(--speed-base) | Card entrances, list items | translateY(20px)→0 + fade |
| `slideInRight` | var(--speed-base) | Sidebar items, drawer content | translateX(-15px)→0 + fade |
| `breathEffect` | 3–4s infinite | Avatar presence, active indicators | scale(1→1.05→1) + opacity cycle |
| `floatElement` | 6s infinite | Decorative elements | translateY(0→-8px→0) — gentle bob |
| `glowPulse` | 2s infinite | Status indicators | opacity(0.4→0.8→0.4) |
| `holographicSweep` | 1.5s infinite | Nav item hover | Gradient position -200%→200% |
| `neural-flow` | 15s infinite | Background ambience | Multi-stop gradient position cycling |
| `pulse` | 2s infinite | Status dot | scale(0.95→1.1→0.95) + glow surge |
| `spin` | 1s linear infinite | Loading spinners | rotate(0→360deg) |

### Hover Patterns
- **Lift**: `translateY(-4px)` — Cards and interactive elements
- **Slide Right**: `translateX(6px)` — Nav items slide toward content
- **Holographic**: Gradient sweep across surface on hover
- **Glow Intensify**: Box-shadow increases from base to bright

---

## Spacing & Grid

### Spacing Scale (tokens.css)
| Token | Value | Use |
|-------|-------|-----|
| `var(--space-xs)` | 0.5rem (8px) | Tight gaps, inline spacing, badge padding |
| `var(--space-sm)` | 1rem (16px) | Standard gap, list spacing |
| `var(--space-md)` | 1.5rem (24px) | Section padding, card internal spacing |
| `var(--space-lg)` | 2rem (32px) | Major section separation, card padding |
| `var(--space-xl)` | 3rem (48px) | Page-level outer padding, sidebar vertical gap |

### Grid System
- **Stats Grid**: `grid-template-columns: repeat(auto-fit, minmax(260px, 1fr))` — Responsive metric cards
- **App Container**: Flexbox. Sidebar (fixed width) + Main Content (flex: 1)
- **Main Content**: Scrollable content area with `scrollbar-gutter: stable`

### Layout Variables (Single Source of Truth)
| Token | Value | Use |
|-------|-------|-----|
| `var(--layout-sidebar-width)` | 280px | Sidebar width. Referenced by overlay positioning |
| `var(--layout-main-padding)` | var(--space-xl) = 3rem | Main content area outer padding |
| `var(--layout-panel-gap)` | var(--space-md) = 1.5rem | Gap between panels |
| `var(--layout-right-panel-width)` | 320px | Right-side context panel width |

> **SSOT Rule**: All layout dimensions are defined in `tokens.css`. Components derive positions using `calc(var(--layout-sidebar-width) + var(--layout-main-padding))`. Never hardcode pixel values in components.

---

## Depth & Elevation

### Glassmorphism Layers

| Level | Treatment | Use |
|-------|-----------|-----|
| Level 0 (Canvas) | No glass. Gradient: `radial-gradient(circle at 50% 50%, #161625 0%, #0b0b0f 100%)` with star particles via `::before` | App background |
| Level 1 (Sidebar) | `var(--bg-dark-sidebar)` + `blur(40px)` + right box-shadow | Navigation panel |
| Level 2 (Surface) | `var(--bg-glass-heavy)` + `blur(15px)` + border glass | Stat cards, containers |
| Level 3 (Panel) | `var(--bg-glass-heavy)` + `blur(25px)` + `var(--shadow-deep)` | Main content panels |
| Level 4 (Glass) | `var(--bg-glass-heavy)` + `blur(20px) saturate(180%)` + deep shadow | Elevated glass elements |
| Level 5 (Deep) | `var(--bg-deep-glass)` + heavy blur | Modals, overlays, command palette |

### Shadow System
- **Glow Cyan** — `var(--glow-cyan)`: `0 0 25px rgba(0, 242, 255, 0.25)` — Active/focus states
- **Glow Purple** — `var(--glow-purple)`: `0 0 25px rgba(188, 140, 255, 0.25)` — Secondary glow
- **Glow Rose** — `var(--glow-rose)`: `0 0 25px rgba(255, 77, 148, 0.25)` — Error/danger glow
- **Shadow Deep** — `var(--shadow-deep)`: `0 20px 50px rgba(0, 0, 0, 0.5)` — Panel elevation

### Border Radius Scale
| Token | Value | Use |
|-------|-------|-----|
| `var(--radius-sm)` | 8px | Buttons, small inputs, scrollbar thumb |
| `var(--radius-md)` | 12px | Nav items, inputs, medium containers |
| `var(--radius-lg)` | 20px | Stat cards, status badges |
| `var(--radius-xl)` | 28px | Main panels, hero containers |

---

## Do's and Don'ts

### Do
- Always reference CSS custom properties via `var(--xxx)` — this is the #1 rule (Golden Rule U-002)
- Use `var(--font-display)` (Outfit) for ALL headings with `letter-spacing: 0.03em+` and `text-transform: uppercase`
- Apply `linear-gradient(135deg, var(--accent-cyan), var(--accent-purple))` with `background-clip: text` for brand headings
- Build on `var(--bg-dark-obsidian)` canvas with glass layers using `backdrop-filter: blur()`
- Use `var(--border-glass)` for all surface borders — never solid colored borders
- Apply hover transforms: `translateY(-4px)` for cards, `translateX(6px)` for nav items
- Add glow effects (`var(--glow-cyan)`) to active/focus states
- Keep spacing on the token scale: xs(0.5rem) → sm(1rem) → md(1.5rem) → lg(2rem) → xl(3rem)
- Use `var(--timing-soft)` as the default transition easing
- Reference layout dimensions via `calc(var(--layout-sidebar-width) + ...)` — never hardcode px

### Don't
- **NEVER** use Tailwind utility classes (`flex`, `mb-4`, etc.) — this project uses Vanilla CSS only (Golden Rule U-001)
- **NEVER** hardcode color hex values (`#00f2ff`), spacing pixels (`12px`), or layout widths (`280px`) in **CSS files or inline `style={{}}` props** — always use `var()` (Golden Rule U-002)
- Don't use pure white (`#ffffff`) as text — `var(--text-primary)` (#f0f2f5) is the correct near-white
- Don't use solid opaque backgrounds for cards — glass transparency with `backdrop-filter` is the system
- Don't skip `backdrop-filter: blur()` on glass surfaces — without blur, it's just a dark rectangle, not glass
- Don't use positive letter-spacing / lowercase on heading elements — Outfit uses wide tracking + uppercase
- Don't introduce warm colors (orange, yellow) for decorative purposes — warmth is reserved for status (amber = warning only)
- Don't use `setState` inside `useFrame` in WebGL components (Golden Rule U-005)
- Don't create CSS without checking `tokens.css` first — the token must exist before you reference it
- Don't add motion without using the timing scale (`--speed-fast/base/slow` + `--timing-soft/bouncy`)

### Exceptions
- **WebGL / three.js / react-three-fiber**: `color` props on `<spotLight>`, `<pointLight>`, `<fog>`, `<meshStandardMaterial>`, and `<Sparkles>` require raw HEX strings (`"#00f2ff"`). CSS custom properties do not work in WebGL context. This does NOT violate U-002.
- **Lucide icon `color` prop**: React icon components accept string colors. Prefer `var(--accent-cyan)` when possible, but raw HEX is tolerable for icons rendered outside CSS context.

> ⚠️ **Legacy Disclosure**: The current codebase has ~30+ inline HEX hardcodes in TSX components (especially `AgentConsole.tsx`, `CausalVisualizer.tsx`, `DemoView.tsx`, `LoraTrainingView.tsx`). These are **existing tech debt**, not permission to add more. New code MUST follow the Don't rules above.

---

## Quick Color Reference
- Primary CTA: `var(--accent-cyan)` (#00f2ff)
- Canvas Background: `var(--bg-dark-obsidian)` (#05070a)
- Glass Surface: `var(--bg-glass-heavy)` (rgba(16, 20, 28, 0.8))
- Heading Text: Gradient `var(--accent-cyan)` → `var(--accent-purple)`
- Body Text: `var(--text-primary)` (#f0f2f5)
- Secondary Text: `var(--text-secondary)` (#94a3b8)
- Muted Text: `var(--text-muted)` (#64748b)
- Border: `var(--border-glass)` (rgba(255, 255, 255, 0.08))
- Success: `var(--accent-emerald)` (#10b981)
- Error: `var(--accent-rose)` (#ff4d94)
- Warning: `var(--accent-amber)` (#f59e0b)

---

## Example Component Prompts

- "Create a dashboard stat card: `var(--bg-glass-heavy)` background with `backdrop-filter: blur(15px)`. Border `1px solid var(--border-glass)`, radius `var(--radius-lg)`. Label at 0.85rem `var(--text-muted)`, value at 2rem weight 800 `var(--font-display)` with `text-shadow: 0 0 15px rgba(255, 255, 255, 0.2)`. Hover: `translateY(-6px)` with `var(--shadow-deep)` and brighter border."

- "Build a navigation sidebar: width `var(--layout-sidebar-width)`, background `var(--bg-dark-sidebar)` with `backdrop-filter: blur(40px)`. Nav items use 0.95rem `var(--text-secondary)`, radius `var(--radius-md)`. Hover: `translateX(6px)`, background `var(--bg-glass-light)`, holographic sweep animation. Active: cyan tinted bg `rgba(var(--accent-cyan-rgb), 0.08)` with left glow bar."

- "Design a glass input field: background `rgba(255, 255, 255, 0.02)`, border `1px solid var(--border-glass)`, radius `var(--radius-md)`. Text `var(--text-primary)`. On focus: border glows `var(--accent-cyan)`, background shifts to `rgba(255, 255, 255, 0.05)`, shadow `0 0 15px rgba(0, 242, 255, 0.1)`."

- "Create a status badge: pill shape (20px radius), background `rgba(16, 185, 129, 0.05)`, border `rgba(16, 185, 129, 0.15)`, text `var(--accent-emerald)`. Include animated dot with `pulse` keyframe. Disconnected variant uses `var(--accent-rose)` instead."

- "Build a section heading: `var(--font-display)` at 1.5rem weight 900, `letter-spacing: 0.06em`, `text-transform: uppercase`. Apply gradient fill: `linear-gradient(135deg, var(--accent-cyan), var(--accent-purple))` with `background-clip: text` and `filter: drop-shadow(0 0 12px rgba(var(--accent-cyan-rgb), 0.3))`."

---

## Iteration Guide

1. **Always start with tokens.css and animations.css**: Check if the design token you need already exists. Color/spacing/radius tokens live in `tokens.css`; timing/speed tokens live in `animations.css`. If not found, add it to the appropriate `:root` block before referencing it.
2. **Glass > Solid**: Every container should be a glass surface. Use `var(--bg-glass-heavy)` + `backdrop-filter: blur()` — never solid dark backgrounds.
3. **Glow for interactivity**: Active/focus states must include a subtle glow. Use `var(--glow-cyan)` or `box-shadow: 0 0 Npx rgba(var(--accent-cyan-rgb), 0.N)`.
4. **Motion is mandatory**: Every state change needs a transition. Default: `transition: all var(--speed-base) var(--timing-soft)`.
5. **Gradient headings**: If it's a heading that names a section, it gets the cyan→purple gradient treatment.
6. **Spacing on the scale**: Only use `var(--space-xs/sm/md/lg/xl)`. No arbitrary values.
7. **Radius on the scale**: Only use `var(--radius-sm/md/lg/xl)`. No arbitrary roundings.
8. **Layout from SSOT**: All positioning offsets derive from `var(--layout-*)` tokens via `calc()`.
9. **Verify after edit**: Always run `npm run lint` after any CSS/TSX change (Golden Rule B-001, U-003).
10. **Check component CSS too**: Design tokens live in `styles/tokens.css`, but component-specific styles exist in `src/components/*.css` (e.g., `TreasureBox.css`). When modifying these, follow the same `var()` rules — do not copy the existing Tailwind-remnant patterns.
