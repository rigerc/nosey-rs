---
name: gpui-styling
description: Styling GPUI elements with Tailwind-like utility methods. Use when applying styles, layouts, colors, spacing, or creating responsive designs.
---

# GPUI Styling

This skill covers styling GPUI elements using Tailwind-like utility methods.

## Overview

GPUI provides chainable methods similar to Tailwind CSS:
- **Sizing**: `.w()`, `.h()`, `.size_full()`
- **Flexbox**: `.flex()`, `.flex_col()`, `.items_center()`, `.justify_center()`
- **Spacing**: `.p()`, `.m()`, `.gap()`
- **Colors**: `.bg()`, `.text_color()`, `.border_color()`
- **Borders**: `.border()`, `.rounded()`

## Sizing

### Width and Height

```rust
use gpui::*;

div()
    .w(px(200.0))              // Fixed width
    .h(px(100.0))              // Fixed height
    .w_full()                   // Width: 100%
    .h_full()                   // Height: 100%
    .size_full()                // Width & Height: 100%
```

### Relative Units

```rust
div()
    .w(relative(0.5))           // 50% of parent
    .h(relative(1.0))           // 100% of parent
```

### Min/Max Sizes

```rust
div()
    .min_w(px(100.0))
    .max_w(px(500.0))
    .min_h(px(50.0))
    .max_h(px(300.0))
```

## Colors

### Background Colors

```rust
use gpui::*;

div()
    .bg(rgb(0x3b82f6))          // Hex color
    .bg(rgb(59, 130, 246))      // RGB
    .bg(rgba(59, 130, 246, 0.5)) // RGBA with alpha
    .bg(hsla(0.6, 0.8, 0.6, 1.0)) // HSLA
```

### Text Colors

```rust
div()
    .text_color(rgb(0xffffff))
    .text_color(rgba(255, 255, 255, 0.9))
```

### Border Colors

```rust
div()
    .border_color(rgb(0x4a4a4a))
```

## Borders

### Border Width

```rust
div()
    .border()                   // 1px all sides
    .border_1()                 // 1px
    .border_2()                 // 2px
    .border_t()                 // Top border
    .border_r()                 // Right border
    .border_b()                 // Bottom border
    .border_l()                 // Left border
```

### Border Radius

```rust
div()
    .rounded(px(4.0))           // Custom radius
    .rounded_sm()               // Small radius
    .rounded_md()               // Medium radius
    .rounded_lg()               // Large radius
    .rounded_full()             // Fully rounded (circle/pill)
```

## Typography

### Font Sizes

```rust
div()
    .text_xs()                  // Extra small
    .text_sm()                  // Small
    .text_base()                // Base size (default)
    .text_lg()                  // Large
    .text_xl()                  // Extra large
    .text_2xl()                 // 2x extra large
    .text_3xl()                 // 3x extra large
```

### Font Weight

```rust
div()
    .font_normal()              // Normal weight
    .font_medium()              // Medium weight
    .font_semibold()            // Semibold
    .font_bold()                // Bold
```

### Text Alignment

```rust
div()
    .text_left()
    .text_center()
    .text_right()
```

## Cursor

```rust
div()
    .cursor_pointer()           // Pointer cursor on hover
    .cursor_default()           // Default cursor
    .cursor_text()              // Text cursor
```

## Hover States

```rust
div()
    .bg(rgb(0x3b82f6))
    .hover(|style| {
        style.bg(rgb(0x2563eb))
    })
```

## Visibility

```rust
div()
    .visible()
    .invisible()
```

## Theme System

### Constant Colors

```rust
// In theme.rs
pub const BG_PRIMARY: Hsla = hsla(0.0, 0.0, 0.05, 1.0);
pub const BG_SECONDARY: Hsla = hsla(0.0, 0.0, 0.1, 1.0);
pub const TEXT_PRIMARY: Hsla = hsla(0.0, 0.0, 0.9, 1.0);
pub const TEXT_SECONDARY: Hsla = hsla(0.0, 0.0, 0.6, 1.0);
pub const ACCENT: Hsla = hsla(0.6, 0.8, 0.5, 1.0);

// Usage
div()
    .bg(theme::BG_PRIMARY)
    .text_color(theme::TEXT_PRIMARY)
```

## More Detail

Read [references/details.md](references/details.md) when you need these sections:

- Flexbox Layout
- Spacing
- Common Patterns
- Color Utilities

## Best Practices

1. **Use spacing scale**: Stick to `.p_1()`, `.p_2()`, `.p_4()`, etc. for consistency
2. **Define color constants**: Create a theme module with color constants
3. **Use HSLA for theming**: Easier to adjust lightness/saturation
4. **Combine utilities**: Chain methods for concise styling
5. **Extract common styles**: Create reusable components for repeated patterns

## Summary

- Use `.size_full()` for full width/height
- Use `.v_flex()` for vertical layouts, `.h_flex()` for horizontal
- Use spacing utilities `.p_4()`, `.gap_2()` for consistent spacing
- Use `.bg()`, `.text_color()` for colors
- Chain methods for concise styling
- Use `.hover()` for interactive states

## References

- [GPUI Documentation](https://gpui.rs)
- [Tailwind CSS](https://tailwindcss.com) - Similar utility concepts
