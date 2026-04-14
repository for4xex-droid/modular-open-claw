# Aiome Design System (Artemis) 🎨

## 🌟 Philosophy
Aiome implements the **Artemis Design System**, focusing on a premium, futuristic, and highly responsive user interface designed for agent-human interaction. The system embraces "Glassmorphism" combined with a dark, high-contrast palette, and relies entirely on centralized CSS tokens to maintain 100% architectural consistency and programmatic control.

**Strict Rule:** No hardcoded color values (HEX, RGB, RGBA) are allowed anywhere in the application. Always use the predefined CSS variables from `tokens.css`.

## 📦 CSS Tokens (`tokens.css`)

### 🌌 Base Backgrounds
- `--bg-primary`: Deepest dark (`#0b0b0f`), used for the root background.
- `--bg-secondary`: Secondary elements (`#0f0f18`).
- `--bg-tertiary`: Elevated cards, slight separation (`#161625`).

### 💠 Glassmorphism System
Aiome features a comprehensive set of generic glass and backdrop modifiers ranging from `--white-01` to `--white-90` and `--black-05` to `--black-90`.
- **Heavy Glass**: `--bg-glass-heavy`
- **Light Glass**: `--bg-glass-light`
- Use these variables to create layered, semi-transparent panels atop 3D visualizers.

### ✨ Brand Accents & Gradients
Accents provide semantic meaning and brand identity. Each primary accent has RGB variants and opacity scales for overlays and shadows.
- **Cyan** (`--accent-cyan`): Used for system neutrality, active states, and tech identity.
- **Purple** (`--accent-purple`): Used for "Samsara", LLM operations, or mystical intelligence elements.
- **Rose / Fuchsia**: Used for highlights or crucial warnings.
- **Emerald / Amber**: Used for success and advisory statuses.

### 📊 Chart Colors
Explicitly mapped semantic colors (`--chart-1` to `--chart-5`) to maintain consistent token-driven styling for all data visualizations (Recharts/Vis.js).

### 🔤 Typography
- **Display**: `'Outfit'` - Clean, geometric, used for massive titles and emphasis.
- **Body**: `'Artemis Inter'` (Inter variant) - Highly legible UI font.
- **Code**: `'JetBrains Mono'` - Predictable, gorgeous monospace for agent logs.

## 📐 Layout Architecture
To ensure seamless integration between the UI DOM layers and WebGL/Canvas backgrounds, layout dimensions are strict `tokens`:
- `--layout-sidebar-width`: 280px
- `--layout-main-padding`: 3rem
- `--layout-panel-gap`: 1.5rem
- `--layout-right-panel-width`: 320px
*These variables enable background visualizers (e.g. DioramaView) to compute the true, unobstructed center of the viewport.*

## 🎬 Animations & Effects
- Defines glow variables `--glow-cyan`, `--glow-purple`, etc.
- Speed semantic tokens (`--speed-fast`, `--speed-normal`, `--speed-slow`, `--speed-genesis`).

## 📚 Compliance Checklist
When contributing to the UI:
1. Ensure `DESIGN.md` is updated if core tokens are added.
2. Scan the new components for raw HEX using the enforcer script.
3. Validate Glassmorphism contrast for accessibility.
