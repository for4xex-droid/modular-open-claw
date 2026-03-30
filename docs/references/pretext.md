# Pretext: DOM-Free Multiline Text Measurement & Layout

**Source**: [github.com/chenglou/pretext](https://github.com/chenglou/pretext)
**Date Added**: 2026-03-30

## Overview
Pure JavaScript/TypeScript library for multiline text measurement & layout. Fast, accurate & supports all languages (including Japanese, Arabic, emoji, mixed-bidi). Allows rendering to DOM, Canvas, SVG and server-side.

Pretext side-steps the need for DOM measurements (`getBoundingClientRect`, `offsetHeight`), which trigger layout reflow — one of the most expensive browser operations. It implements its own text measurement logic using the browser's font engine as ground truth.

## Key Features
- **`prepare()`**: One-time text analysis + measurement pass (~19ms for 500 texts)
- **`layout()`**: Pure arithmetic layout calculation (~0.09ms for 500 texts)
- **`layoutWithLines()`**: Manual line layout for Canvas/SVG rendering
- **`walkLineRanges()`**: Speculative width/height calculations (shrink-wrap, balanced text)
- **`layoutNextLine()`**: Per-line iterator for variable-width layouts (e.g., text flowing around images)

## Relevance to Aiome
Currently **not urgently needed**, but valuable for future UI performance optimization:

- **Chat UI Virtualization**: When `BiomeDialogueView.tsx` handles 1000+ messages, Pretext can pre-calculate message heights for `react-virtuoso` style virtual scrolling without DOM reflow.
- **Timeline/Karma Virtualization**: Same benefit for large karma event lists in `Timeline.tsx`.
- **GraphView Label Calculation**: Pre-compute node label widths for CausalVisualizer canvas rendering.
- **SSR/PDF Generation**: DOM-free text layout for server-side report generation.

### When to Integrate
Trigger: Chat UI scroll performance degrades with 1000+ messages.
Action: Integrate Pretext + `react-virtuoso` in `BiomeDialogueView.tsx` refactoring.
